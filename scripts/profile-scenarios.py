#!/usr/bin/env python3
"""Run bounded MCP lifecycle and error scenarios against a Prism binary."""

from __future__ import annotations

import argparse
import importlib.util
import json
import tempfile
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


def smoke_module() -> Any:
    path = ROOT / "scripts" / "smoke-rea.py"
    spec = importlib.util.spec_from_file_location("prism_smoke_rea", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def expect_error(
    client: Any,
    tool_call: Any,
    error_type: type[BaseException],
    name: str,
    arguments: dict[str, Any],
    timeout: float,
) -> str:
    try:
        tool_call(client, name, arguments, timeout)
    except error_type as error:
        return str(error)
    raise AssertionError(f"{name} unexpectedly succeeded")


def addresses(inventory: Any, limit: int = 4) -> list[str]:
    rows = inventory.get("functions", []) if isinstance(inventory, dict) else []
    result: list[str] = []
    for row in rows if isinstance(rows, list) else []:
        address = row.get("address") if isinstance(row, dict) else None
        if isinstance(address, str) and address and address not in result:
            result.append(address)
        if len(result) >= limit:
            break
    return result


def status(client: Any, tool_call: Any, job: str, timeout: float) -> dict[str, Any]:
    value = tool_call(client, "decompile_batch", {"action": "status", "job": job}, timeout)
    if not isinstance(value, dict):
        raise AssertionError("decompile_batch status was not an object")
    return value


def remaining_timeout(deadline: float, configured: float) -> float:
    """Return a positive request budget without allowing polling to run unbounded."""
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise TimeoutError("scenario deadline expired")
    return min(configured, remaining)


def run(prism: Path, target: Path, timeout: float, output: Path) -> dict[str, Any]:
    smoke = smoke_module()
    output.mkdir(parents=True, exist_ok=True)
    support = output / "support"
    support.mkdir(parents=True, exist_ok=True)
    client: Any | None = None
    result: dict[str, Any] = {"target": str(target), "errors": [], "status": "error"}
    try:
        client = smoke.McpClient(prism, target, support)
        result["initialize"] = smoke.initialize_session(client, timeout)
        client.request("notifications/initialized", {}, timeout, notify=True)
        catalog = client.request("tools/list", {}, timeout)
        result["tool_count"] = len(catalog.get("tools", []))

        inventory = smoke.tool_call(client, "list_functions", {"offset": 0}, timeout)
        selected = addresses(inventory)
        result["function_count"] = len(inventory.get("functions", []))
        result["selected_addresses"] = selected
        result["invalid_read_bytes"] = expect_error(
            client, smoke.tool_call, smoke.RpcError, "read_bytes", {"offset": -1, "length": 8}, timeout
        )
        result["blocked_direct_decompile"] = expect_error(
            client,
            smoke.tool_call,
            smoke.RpcError,
            "analysis_call",
            {"name": "procedure_pseudo_code", "arguments": {"address": selected[0] if selected else "0x0"}},
            timeout,
        )
        result["unknown_tool"] = expect_error(
            client, smoke.tool_call, smoke.RpcError, "definitely_unknown_prism_tool", {}, timeout
        )

        if selected:
            started = smoke.tool_call(
                client, "decompile_batch", {"action": "start", "addresses": selected[:2]}, timeout
            )
            job = started.get("job") if isinstance(started, dict) else None
            if not isinstance(job, str):
                raise AssertionError("decompile_batch start returned no job")
            result["decompile_job"] = job
            deadline = time.monotonic() + timeout
            first = status(
                client,
                smoke.tool_call,
                job,
                remaining_timeout(deadline, timeout),
            )
            result["decompile_initial_status"] = first
            state = first.get("state")
            if state not in {"completed", "completed_with_errors", "complete", "succeeded", "failed", "cancelled"}:
                result["decompile_pause"] = smoke.tool_call(
                    client,
                    "decompile_batch",
                    {"action": "pause", "job": job},
                    remaining_timeout(deadline, timeout),
                )
                result["decompile_paused_status"] = status(
                    client,
                    smoke.tool_call,
                    job,
                    remaining_timeout(deadline, timeout),
                )
                result["decompile_resume"] = smoke.tool_call(
                    client,
                    "decompile_batch",
                    {"action": "resume", "job": job},
                    remaining_timeout(deadline, timeout),
                )
                while time.monotonic() < deadline:
                    current = status(
                        client,
                        smoke.tool_call,
                        job,
                        remaining_timeout(deadline, timeout),
                    )
                    if current.get("state") in {"completed", "completed_with_errors", "complete", "succeeded", "failed", "cancelled"}:
                        result["decompile_final_status"] = current
                        break
                    time.sleep(0.25)
                if "decompile_final_status" not in result:
                    raise TimeoutError("decompilation did not reach a terminal state")
            result["decompile_results"] = smoke.tool_call(
                client,
                "decompile_batch",
                {"action": "results", "job": job, "offset": 0, "limit": 4, "include_data": True},
                timeout,
            )

        root_path = output / "reconstruction-root"
        root_path.mkdir(parents=True, exist_ok=True)
        with_root = {"root": str(root_path)}
        result["reconstruction_start"] = smoke.tool_call(
                client,
                "reconstruction_start",
                {**with_root, "language": "Rust", "instructions": "bounded profiling scenario"},
                timeout,
            )
        result["reconstruction_status"] = smoke.tool_call(
                client, "reconstruction_status", with_root, timeout
            )
        result["reconstruction_modules"] = smoke.tool_call(
                client, "reconstruction_modules", {**with_root, "offset": 0}, timeout
            )
        result["reconstruction_dependencies"] = smoke.tool_call(
                client, "reconstruction_dependencies", {**with_root, "dependencies": []}, timeout
            )
        claim = smoke.tool_call(
                client, "reconstruction_claim", {**with_root, "worker": "profile-scenario"}, timeout
            )
        result["reconstruction_claim"] = claim
        lease = claim.get("lease") if isinstance(claim, dict) else None
        if isinstance(lease, str):
            result["reconstruction_evidence"] = smoke.tool_call(
                    client,
                    "reconstruction_evidence",
                    {**with_root, "lease": lease, "offset": 0},
                    timeout,
                )
            units = claim.get("units", []) if isinstance(claim, dict) else []
            results = [
                    {"address": unit.get("address", "__project__"), "state": "blocked",
                     "reason": "bounded profile scenario intentionally leaves implementation unresolved",
                     "paths": []}
                    for unit in units if isinstance(unit, dict)
                ]
            if results:
                result["reconstruction_submit"] = smoke.tool_call(
                        client,
                        "reconstruction_submit",
                        {**with_root, "submission": {
                            "lease": lease,
                            "files": [{
                                "path": "src/lib.rs",
                                "content": "pub fn profile_scenario() {}\n",
                                "previous_sha256": "",
                            }],
                            "results": results,
                        }},
                        timeout,
                    )
        result["reconstruction_report"] = smoke.tool_call(
                client, "reconstruction_report", with_root, timeout
            )
        result["reconstruction_build"] = smoke.tool_call(
                client, "reconstruction_build", {**with_root, "export_only": True}, timeout
            )
        result["status"] = "ok"
    except Exception as error:
        result["errors"].append(str(error))
    finally:
        if client is not None:
            try:
                cleanup = client.close()
            except Exception as error:
                result["errors"].append(f"MCP cleanup failed: {error}")
            else:
                if cleanup:
                    result["errors"].extend(cleanup)
            if client.stderr_tail:
                result["stderr_tail"] = client.stderr_tail
        if result["status"] == "ok" and result["errors"]:
            result["status"] = "cleanup_error"
    (output / "scenarios.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prism", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=30.0)
    args = parser.parse_args()
    result = run(
        args.prism.resolve(),
        args.target.resolve(),
        args.timeout,
        args.output.resolve(),
    )
    print(json.dumps(result, indent=2))
    return 0 if result["status"] == "ok" and not result["errors"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
