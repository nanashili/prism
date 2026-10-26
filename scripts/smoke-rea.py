#!/usr/bin/env python3
"""Run a read-only Prism headless MCP smoke test for local binaries or apps."""

from __future__ import annotations

import argparse
import json
import os
import queue
import re
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any


class RpcError(RuntimeError):
    """An MCP request failed or the helper process stopped responding."""


class StdioClient:
    """Own a bounded line-delimited MCP subprocess and its readers."""

    def __init__(self, command: list[str], support: Path | None) -> None:
        environment = os.environ.copy()
        if support is not None:
            environment["PRISM_SUPPORT"] = str(support)
        self.process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            text=True,
            bufsize=1,
        )
        self.messages: queue.Queue[object] = queue.Queue()
        self.reader = threading.Thread(target=self._read_messages, daemon=True)
        self.reader.start()
        self.stderr_tail: list[str] = []
        self.reader_errors: list[str] = []
        self.stderr_reader = threading.Thread(target=self._read_stderr, daemon=True)
        self.stderr_reader.start()
        self.next_id = 1

    def _read_messages(self) -> None:
        assert self.process.stdout is not None
        try:
            for line in self.process.stdout:
                try:
                    self.messages.put(json.loads(line))
                except json.JSONDecodeError as error:
                    self.messages.put(RpcError(f"invalid JSON from Prism: {error}"))
        except OSError as error:
            self.messages.put(RpcError(f"reading Prism MCP output failed: {error}"))
        finally:
            self.messages.put(EOFError("Prism MCP output closed"))

    def _read_stderr(self) -> None:
        assert self.process.stderr is not None
        try:
            for line in self.process.stderr:
                self.stderr_tail.append(line.rstrip())
                self.stderr_tail = self.stderr_tail[-80:]
        except OSError as error:
            self.reader_errors.append(f"reading Prism diagnostics failed: {error}")

    def request(
        self,
        method: str,
        params: dict[str, Any],
        timeout: float,
        *,
        notify: bool = False,
    ) -> dict[str, Any]:
        request_id = self.next_id
        self.next_id += 1
        if self.process.stdin is None:
            raise RpcError("Prism MCP stdin is closed")
        request = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        if notify:
            request.pop("id")
        try:
            self.process.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
            self.process.stdin.flush()
        except OSError as error:
            raise RpcError(f"writing MCP request failed: {error}") from error
        if notify:
            return {}

        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RpcError(f"timed out waiting for {method}")
            try:
                message = self.messages.get(timeout=remaining)
            except queue.Empty as error:
                raise RpcError(f"timed out waiting for {method}") from error
            if isinstance(message, BaseException):
                raise RpcError(str(message))
            if not isinstance(message, dict) or message.get("id") != request_id:
                continue
            if "error" in message:
                raise RpcError(json.dumps(message["error"], separators=(",", ":")))
            result = message.get("result")
            if not isinstance(result, dict):
                raise RpcError(f"{method} returned a non-object result")
            return result

    def _wait_for_exit(self, timeout: float, phase: str, errors: list[str]) -> bool:
        try:
            self.process.wait(timeout=timeout)
            return True
        except subprocess.TimeoutExpired:
            return False
        except OSError as error:
            errors.append(f"waiting for Prism ({phase}) failed: {error}")
            return False

    def _stop_process(self, errors: list[str]) -> None:
        if self._wait_for_exit(5, "normal shutdown", errors):
            return

        for action, wait_seconds in (("terminate", 2), ("kill", 2)):
            try:
                getattr(self.process, action)()
            except OSError as error:
                errors.append(f"{action} Prism failed: {error}")
            if self._wait_for_exit(wait_seconds, f"after {action}", errors):
                return
        errors.append("Prism did not exit after kill")

    def _join_reader(self, reader: threading.Thread, name: str, errors: list[str]) -> bool:
        reader.join(timeout=1)
        if reader.is_alive():
            errors.append(f"Prism {name} reader did not stop")
            return False
        return True

    def _close_stream(self, stream: Any, name: str, errors: list[str]) -> None:
        if stream is None:
            return
        try:
            stream.close()
        except OSError as error:
            errors.append(f"closing Prism {name} failed: {error}")

    def close(self) -> list[str]:
        errors: list[str] = []
        self._close_stream(self.process.stdin, "stdin", errors)
        self._stop_process(errors)
        if self.process.returncode is not None and self.process.returncode != 0:
            errors.append(f"Prism exited with status {self.process.returncode}")

        stdout_stopped = self._join_reader(self.reader, "stdout", errors)
        stderr_stopped = self._join_reader(self.stderr_reader, "stderr", errors)
        if stdout_stopped:
            self._close_stream(self.process.stdout, "stdout", errors)
        if stderr_stopped:
            self._close_stream(self.process.stderr, "stderr", errors)
        errors.extend(self.reader_errors)
        return errors


class McpClient(StdioClient):
    """Launch Prism's headless application endpoint."""

    def __init__(self, prism: Path, target: Path, support: Path | None) -> None:
        super().__init__([str(prism), "--headless", str(target)], support)


def tool_call(client: StdioClient, name: str, arguments: dict[str, Any], timeout: float) -> Any:
    result = client.request(
        "tools/call",
        {"name": name, "arguments": arguments},
        timeout,
    )
    if result.get("isError"):
        raise RpcError(json.dumps(result, separators=(",", ":")))
    structured = result.get("structuredContent")
    if structured is not None:
        return structured
    content = result.get("content", [])
    text = "".join(
        block.get("text", "") for block in content if isinstance(block, dict)
    )
    try:
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise RpcError(f"tool {name} returned invalid JSON: {error}") from error


def function_addresses(value: Any, limit: int) -> list[str]:
    rows = value.get("functions", []) if isinstance(value, dict) else []
    if not isinstance(rows, list):
        return []
    addresses: list[str] = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        address = row.get("address")
        if isinstance(address, str) and address and address not in addresses:
            addresses.append(address)
        if len(addresses) == limit:
            break
    return addresses


def initialize_session(client: McpClient, timeout: float) -> dict[str, Any]:
    return client.request(
        "initialize",
        {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "prism-rea-smoke", "version": "1"},
        },
        timeout,
    )


def inventory_functions(client: McpClient, timeout: float, count: int) -> tuple[Any, list[str]]:
    inventory = tool_call(client, "list_functions", {"offset": 0}, timeout)
    return inventory, function_addresses(inventory, count)


def run_decompilation(client: McpClient, addresses: list[str], timeout: float) -> tuple[str, Any]:
    started = tool_call(client, "decompile_batch", {"action": "start", "addresses": addresses}, timeout)
    job = started.get("job") if isinstance(started, dict) else None
    if not isinstance(job, str):
        raise RpcError("decompile_batch start returned no job id")

    deadline = time.monotonic() + timeout
    state = None
    status: Any = None
    while time.monotonic() < deadline:
        status = tool_call(client, "decompile_batch", {"action": "status", "job": job}, timeout)
        state = status.get("state") if isinstance(status, dict) else None
        if state in {
            "completed",
            "completed_with_errors",
            "complete",
            "succeeded",
            "failed",
            "cancelled",
        }:
            break
        time.sleep(0.5)
    if state not in {
        "completed",
        "completed_with_errors",
        "complete",
        "succeeded",
        "failed",
        "cancelled",
    }:
        raise RpcError(f"decompilation did not complete: {state or 'status timeout'}")
    return job, status


def fetch_results(client: McpClient, job: str, timeout: float, count: int) -> list[dict[str, Any]]:
    results = tool_call(
        client,
        "decompile_batch",
        {"action": "results", "job": job, "offset": 0, "limit": count, "include_data": True},
        timeout,
    )
    rows = results.get("results", []) if isinstance(results, dict) else []
    return [row for row in rows if isinstance(row, dict)]


def summarize_results(rows: list[dict[str, Any]], addresses: list[str]) -> tuple[str, list[dict[str, Any]]]:
    def pseudocode(row: dict[str, Any]) -> Any:
        # DecompilationResult serializes its data field under this public key.
        return row.get("pseudocode", row.get("data"))

    selected = {address for address in addresses}
    compact = [
        {
            "address": row.get("address"),
            "status": row.get("status"),
            "pseudocode": pseudocode(row),
            "error": row.get("error"),
            "error_details": row.get("error_details"),
        }
        for row in rows
    ]
    successful = {
        row.get("address")
        for row in rows
        if row.get("status") == "succeeded" and isinstance(pseudocode(row), str)
        and pseudocode(row).strip()
    }
    if selected and selected.issubset(successful):
        return "ok", compact
    return "completed_with_errors", compact


def run_target(
    prism: Path,
    target: Path,
    support: Path | None,
    timeout: float,
    count: int,
    diagnostics_dir: Path | None,
) -> dict[str, Any]:
    summary: dict[str, Any] = {"target": str(target), "status": "error", "errors": []}
    client: McpClient | None = None
    try:
        client = McpClient(prism, target, support)
        initialized = initialize_session(client, timeout)
        summary["protocol_version"] = initialized.get("protocolVersion")
        catalog = client.request("tools/list", {}, timeout)
        tools = catalog.get("tools", [])
        summary["tools"] = [tool.get("name") for tool in tools if isinstance(tool, dict)]

        inventory, addresses = inventory_functions(client, timeout, count)
        summary["function_count"] = len(inventory.get("functions", [])) if isinstance(inventory, dict) else 0
        summary["selected_addresses"] = addresses
        if not addresses:
            summary["status"] = "no_functions"
            summary["limitation"] = "Prism opened the target but list_functions returned no usable addresses."
            return summary

        job, status = run_decompilation(client, addresses, timeout)
        summary["job"] = job
        summary["job_state"] = status.get("state") if isinstance(status, dict) else None
        rows = fetch_results(client, job, timeout, count)
        summary["status"], summary["results"] = summarize_results(rows, addresses)
        return summary
    except (OSError, RpcError, ValueError) as error:
        summary["errors"].append(str(error))
        return summary
    finally:
        if client is not None:
            cleanup_errors = client.close()
            summary["stderr_tail"] = client.stderr_tail
            if cleanup_errors:
                summary["errors"].extend(cleanup_errors)
                if summary["status"] == "ok":
                    summary["status"] = "cleanup_error"
            if diagnostics_dir is not None and client.stderr_tail:
                diagnostics_dir.mkdir(parents=True, exist_ok=True)
                name = re.sub(r"[^A-Za-z0-9_.-]+", "_", target.name) or "target"
                (diagnostics_dir / f"{name}.stderr.log").write_text(
                    "\n".join(client.stderr_tail) + "\n", encoding="utf-8"
                )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prism", required=True, type=Path, help="Prism executable to test")
    parser.add_argument("--output", type=Path, help="Write the JSON summary to this file")
    parser.add_argument("--support", type=Path, help="Use this isolated PRISM_SUPPORT directory")
    parser.add_argument("--diagnostics-dir", type=Path, help="Save bounded Prism stderr logs per target")
    parser.add_argument("--timeout", type=float, default=300.0, help="Per-request/job timeout in seconds")
    parser.add_argument("--count", type=int, default=3, choices=range(1, 4), help="Functions to decompile per target")
    parser.add_argument("targets", type=Path, nargs="+", help="Local files/apps to open read-only")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    args.prism = args.prism.expanduser().resolve()
    args.targets = [target.expanduser().resolve() for target in args.targets]
    if args.support is not None:
        args.support = args.support.expanduser().resolve()
    if not args.prism.is_absolute() or not args.prism.is_file():
        print(f"Prism executable not found: {args.prism}", file=sys.stderr)
        return 2
    results = []
    for target in args.targets:
        if not target.is_absolute() or not target.is_file():
            results.append({"target": str(target), "status": "error", "errors": ["target is not a regular file"]})
            continue
        results.append(
            run_target(args.prism, target, args.support, args.timeout, args.count, args.diagnostics_dir)
        )
    summary = {"prism": str(args.prism), "targets": results}
    encoded = json.dumps(summary, ensure_ascii=False, separators=(",", ":"))
    if args.output:
        args.output.write_text(encoded + "\n", encoding="utf-8")
    else:
        print(encoded)
    return 0 if all(result["status"] == "ok" for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
