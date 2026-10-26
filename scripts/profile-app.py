#!/usr/bin/env python3
"""Profile Prism MCP workloads with hotpath-rs in an isolated support directory.

The harness is deliberately opt-in: it never uses the user's default profile and
only writes below ``--output``.  It records MCP request latency, workload results,
and the hotpath MCP snapshots that were available in the selected build.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import http.client
import importlib.util
import json
import os
import secrets
import socket
import sqlite3
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Iterator


ROOT = Path(__file__).resolve().parents[1]
PROTOCOL_VERSION = "2025-11-25"


class ProfileError(RuntimeError):
    """A profiling transport or workload operation failed."""


def load_smoke_client() -> tuple[type[Any], Any, Any, type[Any]]:
    """Load the existing REA MCP client without duplicating its protocol logic."""
    path = ROOT / "scripts" / "smoke-rea.py"
    spec = importlib.util.spec_from_file_location("prism_smoke_rea", path)
    if spec is None or spec.loader is None:
        raise ProfileError(f"Unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.McpClient, module.tool_call, module.RpcError, module.StdioClient


class HotpathClient:
    """Small stdlib streamable-HTTP MCP client for the hotpath server."""

    def __init__(self, port: int, token: str, timeout: float) -> None:
        self.port = port
        self.token = token
        self.timeout = timeout
        self.session_id: str | None = None
        self.next_id = 1
        self.latencies: list[dict[str, Any]] = []

    def _decode(self, content_type: str, body: bytes) -> dict[str, Any] | None:
        text = body.decode("utf-8", errors="replace")
        if "text/event-stream" not in content_type:
            value = json.loads(text) if text.strip() else None
            return value if isinstance(value, dict) else None
        candidate: dict[str, Any] | None = None
        for line in text.splitlines():
            if line.startswith("data:"):
                try:
                    value = json.loads(line[5:].strip())
                except json.JSONDecodeError:
                    continue
                if isinstance(value, dict):
                    candidate = value
        return candidate

    def request(
        self,
        method: str,
        params: dict[str, Any] | None = None,
        *,
        notify: bool = False,
    ) -> dict[str, Any] | None:
        payload: dict[str, Any] = {"jsonrpc": "2.0", "method": method}
        if not notify:
            payload["id"] = self.next_id
            self.next_id += 1
        if params is not None:
            payload["params"] = params
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
            "Authorization": self.token,
        }
        if self.session_id:
            headers["Mcp-Session-Id"] = self.session_id
        started = time.monotonic()
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=self.timeout)
        try:
            connection.request("POST", "/mcp", json.dumps(payload, separators=(",", ":")), headers)
            response = connection.getresponse()
            body = response.read()
            session = response.getheader("Mcp-Session-Id")
            if session:
                self.session_id = session
            result = self._decode(response.getheader("Content-Type", ""), body)
            elapsed_ms = (time.monotonic() - started) * 1000
            self.latencies.append({"method": method, "elapsed_ms": round(elapsed_ms, 3), "status": response.status})
            if response.status >= 400:
                raise ProfileError(f"hotpath {method} returned HTTP {response.status}")
            if notify:
                return None
            if result is None:
                raise ProfileError(f"hotpath {method} returned no JSON")
            if "error" in result:
                raise ProfileError(f"hotpath {method} failed: {result['error']}")
            return result.get("result") if isinstance(result.get("result"), dict) else result
        except (OSError, json.JSONDecodeError) as error:
            raise ProfileError(f"hotpath {method} request failed: {error}") from error
        finally:
            connection.close()

    def initialize(self) -> None:
        self.request(
            "initialize",
            {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "prism-profile-app", "version": "1"},
            },
        )
        self.request("notifications/initialized", {}, notify=True)

    def list_tool_catalog(self) -> list[dict[str, Any]]:
        result = self.request("tools/list", {}) or {}
        tools = result.get("tools", [])
        return [item for item in tools if isinstance(item, dict) and isinstance(item.get("name"), str)]

    def list_tools(self) -> list[str]:
        return [item["name"] for item in self.list_tool_catalog()]

    def call(self, name: str, arguments: dict[str, Any] | None = None) -> Any:
        result = self.request("tools/call", {"name": name, "arguments": arguments or {}}) or {}
        if result.get("isError"):
            return {"error": result}
        structured = result.get("structuredContent")
        if structured is not None:
            return structured
        content = result.get("content", [])
        text = "".join(item.get("text", "") for item in content if isinstance(item, dict))
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return {"text": text}


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def stage_chat_fixture(
    source: Path | None,
    target: Path,
    support: Path,
) -> dict[str, Any] | None:
    """Copy an optional chat fixture into this run's isolated support directory."""
    if source is None:
        return None
    source = source.expanduser().resolve()
    if not source.is_file():
        raise ProfileError(f"chat fixture is not a regular file: {source}")
    if not target.is_file():
        raise ProfileError(f"chat fixture target is not a regular file: {target}")
    target_digest = hashlib.sha256()
    try:
        with target.open("rb") as target_file:
            for chunk in iter(lambda: target_file.read(1024 * 1024), b""):
                target_digest.update(chunk)
    except OSError as error:
        raise ProfileError(f"hashing chat fixture target failed: {error}") from error
    target_sha256 = target_digest.hexdigest()

    destination = support / "prism.db"
    if destination.exists():
        raise ProfileError(f"chat fixture destination already exists: {destination}")
    support.mkdir(parents=True, exist_ok=True)
    source_connection: sqlite3.Connection | None = None
    destination_connection: sqlite3.Connection | None = None
    try:
        source_connection = sqlite3.connect(f"{source.as_uri()}?mode=ro", uri=True)
        destination_connection = sqlite3.connect(destination)
        source_connection.backup(destination_connection)
        row = destination_connection.execute(
            "SELECT (SELECT COUNT(*) FROM chats), (SELECT COUNT(*) FROM messages)"
        ).fetchone()
        if not row or not all(isinstance(value, int) for value in row):
            raise ProfileError("chat fixture did not expose chats/messages counts")
        chat_count, message_count = row
        if message_count < 500:
            raise ProfileError(f"chat fixture contains {message_count} messages; expected at least 500")
        destination_connection.execute(
            "UPDATE chats SET project=? WHERE id=1",
            (target_sha256,),
        )
        destination_connection.execute(
            "INSERT INTO chat_selection(project,chat) VALUES(?,1) "
            "ON CONFLICT(project) DO UPDATE SET chat=excluded.chat",
            (target_sha256,),
        )
        destination_connection.commit()
    except (OSError, sqlite3.Error) as error:
        raise ProfileError(f"copying chat fixture failed: {error}") from error
    finally:
        if destination_connection is not None:
            destination_connection.close()
        if source_connection is not None:
            source_connection.close()
    return {
        "source": str(source),
        "path": str(destination),
        "chats": chat_count,
        "messages": message_count,
        "target_sha256": target_sha256,
    }


def verify_chat_fixture(summary: dict[str, Any], support: Path) -> None:
    """Record final fixture counts after the application has released the database."""
    fixture = summary.get("chat_fixture")
    if not isinstance(fixture, dict):
        return
    database = support / "prism.db"
    connection: sqlite3.Connection | None = None
    try:
        connection = sqlite3.connect(database)
        row = connection.execute(
            "SELECT (SELECT COUNT(*) FROM chats), (SELECT COUNT(*) FROM messages)"
        ).fetchone()
        if not row or not all(isinstance(value, int) for value in row):
            raise ProfileError("final chat fixture counts were unavailable")
        fixture["final_chats"], fixture["final_messages"] = row
    except (OSError, sqlite3.Error, ProfileError) as error:
        summary["errors"].append(f"final chat fixture validation failed: {error}")
    finally:
        if connection is not None:
            connection.close()


def prepare_desktop_profile(support: Path) -> bool:
    """Seed only fresh desktop supports so the benchmark starts past intro/setup."""
    settings = support / "settings.json"
    if settings.exists():
        return False
    support.mkdir(parents=True, exist_ok=True)
    try:
        settings.write_text(
            json.dumps(
                {
                    "desktop": {"intro_seen": True},
                    "research": {"setup_complete": True},
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
    except OSError as error:
        raise ProfileError(f"writing isolated desktop settings failed: {error}") from error
    return True


@contextlib.contextmanager
def profiling_environment(support: Path, port: int, token: str) -> Iterator[None]:
    keys = (
        "PRISM_SUPPORT",
        "HOTPATH_MCP_PORT",
        "HOTPATH_MCP_AUTH_TOKEN",
        "HOTPATH_FUNCTIONS_LIMIT",
        "HOTPATH_OUTPUT_PATH",
    )
    previous = {key: os.environ.get(key) for key in keys}
    os.environ.update({"PRISM_SUPPORT": str(support), "HOTPATH_MCP_PORT": str(port), "HOTPATH_MCP_AUTH_TOKEN": token})
    os.environ.setdefault("HOTPATH_FUNCTIONS_LIMIT", "1000")
    os.environ.pop("HOTPATH_OUTPUT_PATH", None)
    try:
        yield
    finally:
        for key, value in previous.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def wait_for_hotpath(port: int, token: str, timeout: float) -> HotpathClient:
    client = HotpathClient(port, token, timeout)
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            client.initialize()
            client.list_tools()
            return client
        except ProfileError as error:
            last_error = error
            time.sleep(0.1)
    raise ProfileError(f"hotpath MCP did not start: {last_error}")


def snapshot(client: HotpathClient, tools: list[str]) -> dict[str, Any]:
    names = ["profiler_status", "functions_timing", "functions_alloc", "threads"]
    result: dict[str, Any] = {}
    for name in names:
        if name not in tools:
            result[name] = {"available": False}
            continue
        try:
            value = client.call(name)
            result[name] = {"available": True, "value": value}
            if name == "functions_timing" and isinstance(value, dict):
                rows = value.get("data", [])
                for row in rows[:3] if isinstance(rows, list) else []:
                    if isinstance(row, dict) and isinstance(row.get("id"), int) and "function_timing_logs" in tools:
                        logs = client.call(
                            "function_timing_logs", {"function_id": row["id"]}
                        )
                        result.setdefault("function_timing_logs", {})[str(row["id"])] = logs
            if name == "functions_alloc" and isinstance(value, dict) and "function_alloc_logs" in tools:
                rows = value.get("data", [])
                for row in rows[:3] if isinstance(rows, list) else []:
                    if isinstance(row, dict) and isinstance(row.get("id"), int):
                        logs = client.call(
                            "function_alloc_logs", {"function_id": row["id"]}
                        )
                        result.setdefault("function_alloc_logs", {})[str(row["id"])] = logs
        except ProfileError as error:
            result[name] = {"available": True, "error": str(error)}
    return result


def warm_up(client: HotpathClient, tools: list[str]) -> None:
    """Give the profiler's collection queues one turn before the baseline."""
    if "profiler_status" in tools:
        client.call("profiler_status")
    time.sleep(0.1)


def wait_for_path(path: Path, timeout: float) -> None:
    """Wait for an asynchronously saved artifact while keeping the run bounded."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.is_file():
            return
        time.sleep(0.1)
    raise ProfileError(f"saved artifact did not appear: {path}")


def wait_for_decompilation(
    timed_tool: Any,
    job: str,
    timeout: float,
) -> dict[str, Any]:
    """Poll one durable job and require successful persisted output."""
    terminal_states = {
        "completed",
        "completed_with_errors",
        "complete",
        "succeeded",
        "failed",
        "cancelled",
    }
    deadline = time.monotonic() + timeout
    status: dict[str, Any] = {}
    while time.monotonic() < deadline:
        remaining = deadline - time.monotonic()
        value = timed_tool(
            "decompile_batch",
            {"action": "status", "job": job},
            timeout=remaining,
        )
        status = value if isinstance(value, dict) else {}
        if status.get("state") in terminal_states:
            break
        time.sleep(0.25)
    if status.get("state") not in {"completed", "complete", "succeeded"}:
        raise ProfileError(f"decompilation did not succeed: {status}")
    return status


def validate_decompilation_results(value: Any) -> None:
    """Require at least one saved, non-empty pseudocode result."""
    rows = value.get("results", []) if isinstance(value, dict) else []
    if not isinstance(rows, list) or not any(
        isinstance(row, dict)
        and isinstance(row.get("pseudocode"), str)
        and row["pseudocode"].strip()
        for row in rows
    ):
        raise ProfileError("decompilation returned no saved pseudocode")


def run_headless_target(prism: Path, target: Path, support: Path, args: argparse.Namespace) -> dict[str, Any]:
    McpClient, tool_call, RpcError, _ = load_smoke_client()
    port, token = free_port(), secrets.token_hex(32)
    summary: dict[str, Any] = {
        "target": str(target),
        "status": "error",
        "errors": [],
        "prism_request_latency_ms": [],
    }
    process_client: Any | None = None
    profile_client: HotpathClient | None = None
    with profiling_environment(support, port, token):
        try:
            fixture = stage_chat_fixture(args.chat_fixture, target, support)
            if fixture is not None:
                summary["chat_fixture"] = fixture
            process_client = McpClient(prism, target, support)
            initialized = process_client.request(
                "initialize",
                {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "prism-profile-app", "version": "1"},
                },
                args.timeout,
            )
            process_client.request("notifications/initialized", {}, args.timeout, notify=True)
            summary["protocol_version"] = initialized.get("protocolVersion")
            catalog = process_client.request("tools/list", {}, args.timeout)
            tools = [item.get("name") for item in catalog.get("tools", []) if isinstance(item, dict)]
            summary["tools"] = tools
            profile_client = wait_for_hotpath(port, token, args.timeout)
            profile_catalog = profile_client.list_tool_catalog()
            profile_tools = [item["name"] for item in profile_catalog]
            summary["hotpath_tools"] = profile_tools
            summary["hotpath_tool_catalog"] = profile_catalog
            summary["hotpath_tool_count"] = len(profile_catalog)
            warm_up(profile_client, profile_tools)
            summary["before"] = snapshot(profile_client, profile_tools)

            def timed_tool(
                name: str,
                arguments: dict[str, Any],
                *,
                timeout: float | None = None,
            ) -> Any:
                started = time.monotonic()
                try:
                    request_timeout = args.timeout if timeout is None else max(timeout, 0.1)
                    return tool_call(process_client, name, arguments, request_timeout)
                finally:
                    summary["prism_request_latency_ms"].append(
                        {
                            "method": name,
                            "elapsed_ms": round((time.monotonic() - started) * 1000, 3),
                        }
                    )

            inventory = timed_tool("list_functions", {"offset": 0})
            addresses = []
            for row in inventory.get("functions", [])[: args.samples] if isinstance(inventory, dict) else []:
                if isinstance(row, dict) and isinstance(row.get("address"), str):
                    addresses.append(row["address"])
            summary["function_count"] = len(inventory.get("functions", [])) if isinstance(inventory, dict) else 0
            summary["selected_addresses"] = addresses
            summary["workload"] = {
                "list_functions_page": timed_tool("list_functions", {"offset": 1}),
                "list_functions_filter": timed_tool("list_functions", {"filter": "main", "offset": 0}),
                "analysis_tools": timed_tool("analysis_tools", {}),
                "read_bytes": timed_tool("read_bytes", {"offset": 0, "length": 16}),
            }
            if addresses:
                started = timed_tool(
                    "decompile_batch",
                    {"action": "start", "addresses": addresses},
                )
                job = started.get("job")
                if not isinstance(job, str):
                    raise ProfileError("decompilation start returned no job id")
                summary["job"] = job
                summary["job_status"] = wait_for_decompilation(
                    timed_tool,
                    job,
                    args.timeout,
                )
                summary["results"] = timed_tool(
                    "decompile_batch",
                    {
                        "action": "results",
                        "job": job,
                        "offset": 0,
                        "limit": args.samples,
                        "include_data": True,
                    },
                )
                validate_decompilation_results(summary["results"])
            summary["after"] = snapshot(profile_client, profile_tools)
            summary["hotpath_request_latency_ms"] = profile_client.latencies
            summary["status"] = "ok"
        except (OSError, RpcError, ProfileError, ValueError, KeyError) as error:
            summary["errors"].append(str(error))
            if profile_client is not None:
                summary["hotpath_request_latency_ms"] = profile_client.latencies
        finally:
            if process_client is not None:
                try:
                    cleanup = process_client.close()
                except Exception as error:
                    summary["errors"].append(f"Prism MCP cleanup failed: {error}")
                    cleanup = []
                if cleanup:
                    summary["errors"].extend(cleanup)
                    if summary["status"] == "ok":
                        summary["status"] = "cleanup_error"
                if process_client.stderr_tail:
                    summary["prism_stderr_tail"] = process_client.stderr_tail
                    try:
                        (support / "prism.stderr.log").write_text(
                            "\n".join(process_client.stderr_tail) + "\n",
                            encoding="utf-8",
                        )
                    except OSError as error:
                        summary["errors"].append(f"writing Prism diagnostics failed: {error}")
            verify_chat_fixture(summary, support)
        if summary["status"] == "ok" and summary["errors"]:
            summary["status"] = "cleanup_error"
    return summary


def stop_desktop_process(process: subprocess.Popen[str], errors: list[str]) -> None:
    """Stop a GUI process and retain failures from every cleanup phase."""
    if process.poll() is not None:
        return
    for action in ("terminate", "kill"):
        try:
            getattr(process, action)()
        except OSError as error:
            errors.append(f"desktop {action} failed: {error}")
        try:
            process.wait(timeout=5)
            return
        except subprocess.TimeoutExpired:
            continue
        except OSError as error:
            errors.append(f"desktop wait after {action} failed: {error}")
    errors.append("desktop Prism did not exit after kill")


@contextlib.contextmanager
def desktop_process(
    prism: Path,
    target: Path,
    support: Path,
    environment: dict[str, str],
    timeout: float,
    errors: list[str],
) -> Iterator[subprocess.Popen[str]]:
    with (support / "prism.stdout.log").open("w", encoding="utf-8") as stdout, (
        support / "prism.stderr.log"
    ).open("w", encoding="utf-8") as stderr:
        process = subprocess.Popen(
            [str(prism), str(target)], env=environment, stdout=stdout, stderr=stderr, text=True
        )
    try:
        deadline = time.monotonic() + timeout
        endpoint = support / "window.json"
        while not endpoint.exists():
            if process.poll() is not None:
                raise ProfileError(f"desktop Prism exited with status {process.returncode}")
            if time.monotonic() >= deadline:
                raise ProfileError("desktop Prism did not publish window.json")
            time.sleep(0.1)
        yield process
    finally:
        stop_desktop_process(process, errors)


class WindowClient:
    """Record complete Prism window MCP round trips through the shared client."""

    def __init__(self, client: Any, tool_call: Any, timeout: float) -> None:
        self.client = client
        self.tool_call = tool_call
        self.timeout = timeout
        self.responses: list[dict[str, Any]] = []
        self.latencies: list[dict[str, Any]] = []

    def request(
        self,
        method: str,
        params: dict[str, Any],
        timeout: float | None = None,
        *,
        notify: bool = False,
    ) -> dict[str, Any]:
        started = time.monotonic()
        entry: dict[str, Any] = {"method": method}
        if method == "tools/call":
            entry["tool"] = params.get("name")
        try:
            result = self.client.request(
                method, params, self.timeout if timeout is None else timeout, notify=notify
            )
            self.responses.append({**entry, "response": result})
            return result
        finally:
            entry["elapsed_ms"] = round((time.monotonic() - started) * 1000, 3)
            self.latencies.append(entry)

    def initialize(self) -> None:
        self.request("initialize", {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "prism-profile-app", "version": "1"},
        })
        self.request("notifications/initialized", {}, notify=True)
        catalog = self.request("tools/list", {})
        tools = {item.get("name") for item in catalog.get("tools", [])}
        required = {
            "get_state",
            "show_screen",
            "set_view",
            "capture_window",
            "save_project",
            "open_file",
        }
        if missing := required - tools:
            raise ProfileError(f"desktop MCP is missing required tools: {sorted(missing)}")

    def call(self, name: str, arguments: dict[str, Any], timeout: float | None = None) -> Any:
        return self.tool_call(self, name, arguments, self.timeout if timeout is None else timeout)

    def wait_idle(self) -> dict[str, Any]:
        deadline = time.monotonic() + self.timeout
        while (remaining := deadline - time.monotonic()) > 0:
            state = self.call("get_state", {}, remaining)
            if not isinstance(state, dict):
                raise ProfileError("desktop get_state returned a non-object result")
            if state.get("pending") == 0:
                return state
            time.sleep(min(0.1, remaining))
        raise ProfileError("desktop operation did not finish before its deadline")


def select_and_decompile(
    window: WindowClient,
    observations: dict[str, Any],
    initial: dict[str, Any],
) -> str:
    """Open the target, index it, and exercise one durable decompilation job."""
    if not initial.get("target"):
        raise ProfileError(f"desktop did not open the initial target: {initial.get('error')}")
    window.call("analyze", {})
    observations["indexed"] = window.wait_idle()

    inventory = window.call("list_functions", {"offset": 0})
    functions = inventory.get("functions", []) if isinstance(inventory, dict) else []
    if (
        not functions
        or not isinstance(functions[0], dict)
        or not isinstance(functions[0].get("address"), str)
    ):
        raise ProfileError("desktop analysis returned no selectable function")
    address = functions[0]["address"]
    started = window.call("decompile_batch", {"action": "start", "addresses": [address]})
    job = started.get("job")
    if not isinstance(job, str):
        raise ProfileError("desktop decompilation returned no job id")
    observations["decompilation"] = wait_for_decompilation(window.call, job, window.timeout)
    results = window.call(
        "decompile_batch",
        {
            "action": "results",
            "job": job,
            "offset": 0,
            "limit": 1,
            "include_data": True,
        },
    )
    validate_decompilation_results(results)
    window.call("open_function", {"address": address, "view": "Assembly"})
    window.wait_idle()
    return address


def exercise_views(window: WindowClient, support: Path) -> dict[str, Any]:
    """Render each supported workbench view and retain compact observations."""
    observations: dict[str, Any] = {}
    views = (
        "Assembly", "Pseudocode", "References", "Strings", "Imports", "Segments",
        "Hex", "Resources", "Notes", "Graph", "Diff", "Debugger",
        "Reconstruct project",
    )
    for view in views:
        window.call("set_view", {"view": view})
        current = window.wait_idle()
        time.sleep(0.15)
        observations[view] = {
            "view": current.get("view"),
            "error": current.get("error"),
            "visible_characters": len(current.get("visible_text", "")),
        }
    window.call("set_view", {"view": "Assembly"})
    window.wait_idle()
    window.call("capture_window", {"path": str(support / "capture.png")})
    return observations


def exercise_annotations(window: WindowClient, address: str) -> Any:
    """Exercise function rename, notes, bookmarks, and mark enumeration."""
    window.call("rename_function", {"function": address, "new_name": "profiled_function"})
    window.wait_idle()
    window.call("add_note", {"function": address, "note": "Isolated profiling workload"})
    window.wait_idle()
    window.call("bookmark", {"address": address})
    window.wait_idle()
    return window.call("list_marks", {})


def exercise_settings_and_navigation(window: WindowClient, support: Path) -> list[str]:
    """Visit settings pages and top-level screens while waiting for each transition."""
    pages = (
        "General", "Workspace", "Appearance", "Keyboard", "Chat", "Agents",
        "Permissions", "Privacy", "Skills", "Engines", "Code views", "Debugging",
        "GPU", "Storage", "About",
    )
    visited: list[str] = []
    for page in pages:
        window.call("show_screen", {"screen": "Settings", "page": page})
        window.wait_idle()
        time.sleep(0.15)
        visited.append(page)
    window.call("show_screen", {"screen": "Settings", "page": "Storage"})
    window.wait_idle()
    time.sleep(0.2)
    window.call("capture_window", {"path": str(support / "storage.png")})
    for screen in ("Home", "Setup", "Workbench"):
        window.call("show_screen", {"screen": screen})
        window.wait_idle()
    return visited


def save_and_reopen(window: WindowClient, target: Path, support: Path) -> dict[str, Any]:
    """Save a project in the isolated profile, reopen it, and restore the target."""
    saved_project = support / "saved-project.prism"
    window.call("save_project", {"path": str(saved_project)})
    window.wait_idle()
    wait_for_path(saved_project, window.timeout)
    window.call("open_file", {"path": str(saved_project)})
    reopened_project = window.wait_idle()
    window.call("open_file", {"path": str(target)})
    reopened_target = window.wait_idle()
    return {"project": reopened_project, "target": reopened_target}


def desktop_workload(window: WindowClient, target: Path, support: Path) -> dict[str, Any]:
    """Exercise analysis, rendering, editing, settings, and project persistence."""
    initial = window.wait_idle()
    if not initial.get("target"):
        raise ProfileError(f"desktop did not open the initial target: {initial.get('error')}")
    observations: dict[str, Any] = {"initial": initial}
    address = select_and_decompile(window, observations, initial)
    observations["views"] = exercise_views(window, support)
    observations["marks"] = exercise_annotations(window, address)
    observations["settings"] = exercise_settings_and_navigation(window, support)
    reopened = save_and_reopen(window, target, support)
    observations["reopened_project"] = reopened["project"]
    observations["reopened_target"] = reopened["target"]
    return observations


@contextlib.contextmanager
def desktop_helper(
    mcp: Path,
    support: Path,
    timeout: float,
    errors: list[str],
) -> Iterator[tuple[WindowClient, HotpathClient]]:
    _, tool_call, _, StdioClient = load_smoke_client()
    port, token = free_port(), secrets.token_hex(32)
    with profiling_environment(support, port, token):
        client = StdioClient([str(mcp)], support)
    try:
        profiler = wait_for_hotpath(port, token, timeout)
        window = WindowClient(client, tool_call, timeout)
        window.initialize()
        yield window, profiler
    finally:
        try:
            errors.extend(client.close())
        except (OSError, RuntimeError) as error:
            errors.append(f"desktop MCP cleanup failed: {error}")
        try:
            (support / "prism-mcp.stderr.log").write_text("\n".join(client.stderr_tail) + "\n", encoding="utf-8")
        except OSError as error:
            errors.append(f"writing desktop MCP diagnostics failed: {error}")


def run_desktop(prism: Path, mcp: Path, target: Path, support: Path, args: argparse.Namespace) -> dict[str, Any]:
    summary: dict[str, Any] = {"target": str(target), "status": "error", "errors": []}
    _, _, RpcError, _ = load_smoke_client()
    port, token = free_port(), secrets.token_hex(32)
    environment = os.environ.copy()
    environment.update({"PRISM_SUPPORT": str(support), "HOTPATH_MCP_PORT": str(port), "HOTPATH_MCP_AUTH_TOKEN": token})
    environment.setdefault("HOTPATH_FUNCTIONS_LIMIT", "1000")
    environment.pop("HOTPATH_OUTPUT_PATH", None)
    try:
        fixture = stage_chat_fixture(args.chat_fixture, target, support)
        if fixture is not None:
            summary["chat_fixture"] = fixture
        summary["intro_disabled"] = prepare_desktop_profile(support)
        with desktop_process(prism, target, support, environment, args.timeout, summary["errors"]):
            profiler = wait_for_hotpath(port, token, args.timeout)
            catalog = profiler.list_tool_catalog()
            tools = [item["name"] for item in catalog]
            summary.update({"hotpath_tools": tools, "hotpath_tool_catalog": catalog, "hotpath_tool_count": len(catalog)})
            warm_up(profiler, tools)
            summary["before"] = snapshot(profiler, tools)
            with desktop_helper(mcp, support, args.timeout, summary["errors"]) as (window, helper_profiler):
                try:
                    summary["workload"] = desktop_workload(window, target, support)
                finally:
                    summary["responses"] = window.responses
                    summary["mcp_request_latency_ms"] = window.latencies
                    try:
                        helper_tools = helper_profiler.list_tools()
                        summary["helper_profile"] = snapshot(helper_profiler, helper_tools)
                    except (OSError, ProfileError, ValueError) as error:
                        summary["errors"].append(f"helper profiling snapshot failed: {error}")
                    try:
                        summary["after"] = snapshot(profiler, tools)
                        summary["hotpath_request_latency_ms"] = profiler.latencies
                    except (OSError, ProfileError, ValueError) as error:
                        summary["errors"].append(f"hotpath profiling snapshot failed: {error}")
        summary["status"] = "error" if summary["errors"] else "ok"
    except (OSError, ProfileError, RpcError, ValueError) as error:
        summary["errors"].append(str(error))
    finally:
        verify_chat_fixture(summary, support)
        if summary["status"] == "ok" and summary["errors"]:
            summary["status"] = "cleanup_error"
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prism", required=True, type=Path)
    parser.add_argument("--mcp", type=Path, help="prism-mcp helper for --mode desktop")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--mode", choices=("headless", "desktop"), default="headless")
    parser.add_argument(
        "--chat-fixture",
        type=Path,
        help="copy a validated SQLite chat fixture into each target's isolated support directory",
    )
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=300.0)
    parser.add_argument("targets", type=Path, nargs="+")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.samples < 1 or args.samples > 3:
        raise SystemExit("--samples must be between 1 and 3")
    if args.timeout <= 0:
        raise SystemExit("--timeout must be positive")
    args.prism = args.prism.expanduser().resolve()
    args.output = args.output.expanduser().resolve()
    if args.chat_fixture is not None:
        args.chat_fixture = args.chat_fixture.expanduser().resolve()
    args.targets = [target.expanduser().resolve() for target in args.targets]
    args.output.mkdir(parents=True, exist_ok=True)
    run_label = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + f"-{secrets.token_hex(4)}"
    results = []
    for index, target in enumerate(args.targets):
        support = args.output / "support" / run_label / str(index)
        support.mkdir(parents=True, exist_ok=True)
        if not target.is_file():
            results.append({"target": str(target), "status": "error", "errors": ["target is not a regular file"]})
            continue
        if args.mode == "desktop":
            if args.mcp is None:
                results.append(
                    {
                        "target": str(target),
                        "status": "error",
                        "errors": ["--mcp is required for desktop mode"],
                    }
                )
            else:
                results.append(run_desktop(args.prism, args.mcp.expanduser().resolve(), target, support, args))
        else:
            results.append(run_headless_target(args.prism, target, support, args))
    summary = {"prism": str(args.prism), "mode": args.mode, "targets": results}
    report = json.dumps(summary, indent=2, ensure_ascii=False) + "\n"
    report_path = args.output / "profile.json"
    if report_path.exists():
        report_path = args.output / f"profile-{run_label}.json"
    report_path.write_text(report, encoding="utf-8")
    return 0 if all(item.get("status") == "ok" for item in results) else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ProfileError, ValueError) as error:
        print(f"profiling failed: {error}", file=sys.stderr)
        raise SystemExit(1)
