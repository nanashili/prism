#!/usr/bin/env python3
"""Profile new native MCP workflows against isolated, compiler-built fixtures.

Execution observations and type evidence are synthetic inputs. This workload
checks their storage and API behavior; it does not record a live debugger trace
or establish that proposed analysis repairs are correct.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import os
import secrets
import subprocess
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PROTOCOL_VERSION = "2025-11-25"
FIXTURE_FUNCTIONS = 321


def load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def build_fixtures(output: Path) -> tuple[Path, Path]:
    clang = os.environ.get("CC", "clang")
    binaries = []
    for version in (1, 2):
        source = output / f"fixture-v{version}.c"
        binary = output / f"fixture-v{version}"
        functions = [
            f"int profile_func_{index:03d}(int value) "
            f"{{ return value * 3 + {index + version}; }}\n"
            for index in range(FIXTURE_FUNCTIONS - 1)
        ]
        main = "int main(void) { return profile_func_000(7) >= 0 ? 0 : 1; }\n"
        source.write_text("".join(functions) + main, encoding="utf-8")
        subprocess.run(
            [clang, "-g", "-O0", "-fno-inline", str(source), "-o", str(binary)],
            check=True,
            capture_output=True,
            text=True,
        )
        binaries.append(binary)
    return binaries[0], binaries[1]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


class FeatureSession:
    """Own one Prism process and reconnect its profiler after each restart."""

    def __init__(
        self,
        prism: Path,
        target: Path,
        support: Path,
        timeout: float,
        result: dict[str, Any],
        profile: Any,
        smoke: Any,
        port: int,
        token: str,
    ) -> None:
        self.prism = prism
        self.target = target
        self.support = support
        self.timeout = timeout
        self.result = result
        self.profile = profile
        self.smoke = smoke
        self.port = port
        self.token = token
        self.client: Any = None
        self.profiler: Any = None
        self.profiler_tools: list[str] = []

    def start(self) -> None:
        require(self.client is None, "Prism session is already running")
        self.client = self.smoke.McpClient(self.prism, self.target, self.support)
        initialized = self.client.request(
            "initialize",
            {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "prism-profile-features", "version": "2"},
            },
            self.timeout,
        )
        self.result["protocol_version"] = initialized.get("protocolVersion")
        self.client.request("notifications/initialized", {}, self.timeout, notify=True)
        self.profiler = self.profile.wait_for_hotpath(
            self.port, self.token, self.timeout
        )
        self.profiler_tools = self.profiler.list_tools()
        self.result["hotpath_tools"] = self.profiler_tools

    def close(self, label: str = "cleanup_errors") -> None:
        if self.client is None:
            return
        self.result["prism_stderr_tail"] = list(self.client.stderr_tail)
        try:
            errors = self.client.close()
            self.result[label] = errors
            self.result["errors"].extend(errors)
        except Exception as error:
            self.result[label] = [str(error)]
            raise
        finally:
            self.client = None
            self.profiler = None

    def call(
        self,
        name: str,
        arguments: dict[str, Any],
        key: str | None = None,
    ) -> dict[str, Any]:
        started = time.monotonic()
        record: dict[str, Any] = {"tool": name, "arguments": arguments}
        self.result.setdefault("calls", []).append(record)
        try:
            value = self.smoke.tool_call(
                self.client, name, arguments, self.timeout
            )
            require(isinstance(value, dict), f"{name} returned a non-object response")
            record.update(status="ok", response=value)
            if key is not None:
                self.result[key] = value
            return value
        except Exception as error:
            record.update(status="error", error=str(error))
            raise
        finally:
            record["elapsed_ms"] = round((time.monotonic() - started) * 1000, 3)

    def expect_error(
        self,
        name: str,
        arguments: dict[str, Any],
        label: str,
        expected_text: str,
    ) -> None:
        started = time.monotonic()
        record: dict[str, Any] = {"tool": name, "arguments": arguments, "label": label}
        self.result.setdefault("calls", []).append(record)
        try:
            self.smoke.tool_call(self.client, name, arguments, self.timeout)
        except Exception as error:
            record["error"] = str(error)
            record["status"] = "expected_error" if expected_text in str(error) else "error"
            require(
                record["status"] == "expected_error",
                f"{label} failed for an unexpected reason: {error}",
            )
            self.result.setdefault("expected_errors", []).append(record)
            return
        finally:
            record["elapsed_ms"] = round((time.monotonic() - started) * 1000, 3)
        record["status"] = "unexpected_success"
        raise RuntimeError(f"{label} unexpectedly succeeded")

    def snapshot(self, label: str) -> None:
        captured = self.profile.snapshot(self.profiler, self.profiler_tools)
        self.result[label] = captured
        for name in ("profiler_status", "functions_timing", "functions_alloc", "threads"):
            value = captured.get(name)
            require(
                isinstance(value, dict)
                and value.get("available") is True
                and "error" not in value,
                f"{label}.{name} is unavailable or errored",
            )


def wait_for_decompilation(session: FeatureSession, job: str) -> None:
    deadline = time.monotonic() + session.timeout
    failures = {"completed_with_errors", "failed", "cancelled"}
    while time.monotonic() < deadline:
        status = session.call(
            "decompile_batch", {"action": "status", "job": job}, "decompile_status"
        )
        state = status.get("state")
        require(state not in failures, f"Fixture decompilation ended as {state}")
        if state in {"completed", "complete", "succeeded"}:
            return
        time.sleep(0.25)
    raise TimeoutError("Fixture decompilation did not complete")


def run_search(session: FeatureSession) -> list[str]:
    result = session.result
    inventory = session.call("list_functions", {"offset": 0}, "inventory")
    addresses = [row["address"] for row in inventory.get("functions", [])[:2]]
    require(len(addresses) == 2, "Fixture analysis returned fewer than two functions")
    result["addresses"] = addresses
    started = session.call(
        "decompile_batch", {"action": "start", "addresses": addresses}, "decompile_job"
    )
    job = started.get("job")
    require(isinstance(job, str), "Decompilation did not return a job identifier")
    wait_for_decompilation(session, job)
    pseudocode = session.call(
        "decompile_batch",
        {"action": "results", "job": job, "offset": 0, "limit": 2, "include_data": True},
        "pseudocode",
    )
    rows = pseudocode.get("results", [])
    require(
        len(rows) == 2 and all(row.get("status") == "succeeded" for row in rows),
        "Fixture pseudocode results are incomplete",
    )

    session.snapshot("hotpath_before_index")
    pages = result["index_pages"] = []
    offset = 0
    for _ in range(5):
        page = session.call("index_function_search", {"offset": offset})
        pages.append(page)
        next_offset = page.get("next_offset")
        if next_offset is None:
            break
        require(
            isinstance(next_offset, int) and next_offset > offset,
            "Function indexing did not advance its cursor",
        )
        offset = next_offset
    else:
        raise RuntimeError("Fixture indexing exceeded its bounded page budget")
    session.snapshot("hotpath_after_index")
    coverage = pages[-1].get("coverage", {})
    require(
        coverage.get("inventory_functions") == FIXTURE_FUNCTIONS
        and coverage.get("indexed_functions") == FIXTURE_FUNCTIONS,
        "Function indexing did not cover the complete 321-function fixture",
    )
    result["acceptance"].update(coverage)

    query = "multiply an integer and return a computed value"
    semantic = session.call("semantic_search", {"query": query, "limit": 5}, "semantic_search")
    require(bool(semantic.get("matches")), "Saved-evidence search returned no matches")
    structural = session.call(
        "semantic_search",
        {"query": query, "similar_to": addresses[0], "limit": 5},
        "structural_search",
    )
    matched = any(
        row.get("address") == addresses[1] and row.get("structural") is not None
        for row in structural.get("matches", [])
    )
    require(matched, "Explicit structural search did not find the second fixture function")
    result["acceptance"]["structural_candidate_present"] = matched
    return addresses


def run_comparison(session: FeatureSession, baseline: Path) -> None:
    comparison = session.call(
        "compare_versions", {"baseline": str(baseline)}, "compare_versions"
    )
    require(
        comparison.get("summary", {}).get("matches") == FIXTURE_FUNCTIONS,
        "Version comparison did not return candidates for every fixture function",
    )
    page = session.call(
        "version_matches", {"kind": "matches", "offset": 0}, "version_matches"
    )
    require(bool(page.get("rows")), "Version match paging returned no candidates")


def run_verification(session: FeatureSession, address: str) -> None:
    captured = session.call(
        "verification_evidence",
        {"action": "capture", "view": "pseudocode", "address": address},
        "verification_capture",
    )
    evidence_id = captured.get("prism_evidence", {}).get("id")
    require(isinstance(evidence_id, str), "Pseudocode capture returned no evidence identifier")
    session.call(
        "verification_evidence",
        {"action": "get", "evidence": evidence_id},
        "verification_evidence_get",
    )
    draft = {
        "statement": "The saved pseudocode contains a profile function.",
        "scope": "predicate",
        "checks": [
            {
                "evidence": evidence_id,
                "pointer": "/text",
                "predicate": {"operation": "contains", "text": "profile"},
            }
        ],
    }
    claim = session.call(
        "verification_claim", {"action": "propose", "draft": draft}, "verification_claim"
    )
    claim_id = claim.get("id")
    require(isinstance(claim_id, int), "Claim proposal returned no identifier")
    verified = session.call(
        "verification_claim",
        {"action": "verify", "claim": claim_id},
        "verification_claim_verify",
    )
    status = verified.get("assessment", {}).get("status")
    require(status in {"verified", "corroborated"}, f"Claim was not corroborated: {status}")
    session.result["acceptance"]["claim_status"] = status
    session.call(
        "verification_claim",
        {"action": "get", "claim": claim_id},
        "verification_claim_get",
    )
    session.call(
        "verification_claim",
        {"action": "list", "offset": 0},
        "verification_claim_list",
    )
    draft["statement"] = "Reviewed saved pseudocode contains a profile function."
    session.call(
        "verification_claim",
        {"action": "revise", "claim": claim_id, "draft": draft},
        "verification_claim_revise",
    )
    session.expect_error(
        "verification_evidence",
        {"action": "get", "evidence": "0" * 64},
        "unknown evidence digest",
        "Query returned no rows",
    )


def run_investigation(session: FeatureSession, addresses: list[str]) -> None:
    started = session.call(
        "investigation",
        {"action": "start", "objective": "Bounded feature profile of the compiled fixture"},
        "investigation",
    )
    investigation_id = started.get("graph", {}).get("id")
    require(isinstance(investigation_id, str), "Investigation returned no graph identifier")
    base = {"investigation": investigation_id}
    claimed = session.call(
        "investigation", {"action": "next", **base}, "investigation_next"
    ).get("next", {})
    task = claimed.get("task")
    attempt = claimed.get("attempt")
    require(
        isinstance(task, int) and isinstance(attempt, int),
        "Investigation did not claim a task",
    )
    session.call(
        "investigation",
        {
            "action": "block", **base, "task": task, "attempt": attempt,
            "reason": "Harness exercises task recovery; behavior remains unverified.",
        },
        "investigation_block",
    )
    session.call(
        "investigation", {"action": "retry", **base, "task": task}, "investigation_retry"
    )
    paused = session.call(
        "investigation", {"action": "pause", **base}, "investigation_pause"
    )
    require(paused.get("graph", {}).get("paused") is True, "Investigation did not pause")
    session.call("investigation", {"action": "status", **base}, "investigation_status")
    session.call("investigation", {"action": "resume", **base}, "investigation_resume")
    recovery = session.call(
        "investigation", {"action": "next", **base}, "investigation_next_recovery"
    ).get("next", {})
    require(
        recovery.get("task") == task and recovery.get("attempt") == attempt + 1,
        "Investigation retry lost its task or attempt identity",
    )
    completed = session.call(
        "investigation",
        {
            "action": "complete", **base, "task": task, "attempt": recovery["attempt"],
            "report": {
                "findings": (
                    "Fixture inventory and saved pseudocode profiled; "
                    "behavior remains unverified."
                ),
                "functions": addresses,
                "unresolved": [],
            },
        },
        "investigation_complete",
    )

    session.snapshot("hotpath_before_restart")
    session.close("restart_close_errors")
    require(not session.result["errors"], "Prism cleanup failed before restart")
    session.start()
    restored = session.call(
        "investigation", {"action": "status", **base}, "investigation_after_restart"
    )
    require(
        restored.get("graph") == completed.get("graph"),
        "Investigation graph, task state, usage or revision changed across restart",
    )
    session.result["acceptance"]["investigation_persisted"] = True
    session.snapshot("hotpath_after_restart")


def run_execution(session: FeatureSession, address: str) -> None:
    trace = {
        "version": 1,
        "id": "synthetic-profile-trace",
        "workflow": "profile-features synthetic import",
        "source": "imported",
        "runtime_version": "synthetic-fixture",
        "captured_at": "profile-run",
        "target_sha256": digest(session.target),
        "mapping": {
            "runtime_base": "0x100000000",
            "static_base": "0x100000000",
            "size": session.target.stat().st_size,
        },
        "complete": False,
        "events": [
            {"kind": kind, "address": address, "thread_id": 1, "count": 1}
            for kind in ("function", "sample")
        ],
    }
    actions = (
        ("import", {"trace": trace}),
        ("list", {}),
        ("read", {"trace_id": trace["id"]}),
        ("functions", {"trace_id": trace["id"], "offset": 0}),
        ("graph", {"address": address}),
    )
    for action, arguments in actions:
        session.call(
            "execution_evidence", {"action": action, **arguments}, f"execution_{action}"
        )
    read = session.result["execution_read"]
    require(read.get("events") == trace["events"], "Imported trace observations changed")
    functions = session.result["execution_functions"].get("functions", [])
    require(
        any(row.get("address") == address for row in functions),
        "Imported function observation is absent from the function list",
    )
    graph = session.result["execution_graph"]
    dynamic = graph.get("dynamic", [])
    observations = [row for row in dynamic if row.get("trace_id") == trace["id"]]
    require(
        len(observations) == 1 and observations[0].get("source") == "imported",
        "Graph does not retain the imported trace provenance",
    )
    # This provider can return an empty CFG. A trace's presence then establishes
    # provenance, while the separate indexed-function query establishes the hit.
    has_blocks = bool(graph.get("blocks"))
    require(
        observations[0].get("function_observed") is has_blocks,
        "Graph observation correlation is inconsistent with its static blocks",
    )
    session.result["acceptance"]["graph_correlation"] = (
        "observed" if has_blocks else "unavailable_without_static_blocks"
    )
    session.result["acceptance"]["synthetic_function_observation_present"] = True
    session.expect_error(
        "execution_evidence", {"action": "import", "trace": trace},
        "duplicate trace id", "UNIQUE constraint failed",
    )
    invalid_hash = copy.deepcopy(trace)
    invalid_hash.update(id="synthetic-invalid-hash", target_sha256="0" * 64)
    session.expect_error(
        "execution_evidence", {"action": "import", "trace": invalid_hash},
        "trace target hash mismatch", "SHA-256 does not match",
    )
    invalid_mapping = copy.deepcopy(trace)
    invalid_mapping.update(
        id="synthetic-invalid-mapping",
        mapping={"runtime_base": "0x1", "static_base": "0x1", "size": 0},
    )
    session.expect_error(
        "execution_evidence", {"action": "import", "trace": invalid_mapping},
        "zero mapping size", "Invalid or overflowing module address range",
    )
    session.call(
        "execution_evidence",
        {"action": "remove", "trace_id": trace["id"]},
        "execution_remove",
    )
    session.expect_error(
        "execution_evidence", {"action": "read", "trace_id": trace["id"]},
        "removed trace", "Unknown trace",
    )


def run_repair(session: FeatureSession, address: str) -> None:
    scanned = session.call(
        "analysis_repair", {"action": "scan", "address": address}, "analysis_repair_scan"
    )
    repair_id = scanned.get("repair")
    require(isinstance(repair_id, str), "Analysis scan returned no repair identifier")
    plan = {
        "specialist": "profile-features harness",
        "diagnosis": "Supplied test plan exercises the API; no provider correction is asserted.",
        "adjustment": {"kind": "inspect_control_flow"},
    }
    session.call(
        "analysis_repair",
        {"action": "propose", "repair": repair_id, "plan": plan},
        "analysis_repair_propose",
    )
    actions = (
        ("status", "analysis_repair_status"),
        ("evidence", "analysis_repair_evidence"),
        ("rerun", "analysis_repair_rerun"),
        ("status", "analysis_repair_status_after"),
        ("discard", "analysis_repair_discard"),
    )
    for action, key in actions:
        session.call("analysis_repair", {"action": action, "repair": repair_id}, key)
    rerun = session.result["analysis_repair_rerun"]
    require(
        rerun.get("state") == "compared" and rerun.get("behavior_verified") is False,
        "Analysis rerun did not preserve its unverified comparison state",
    )
    require(
        session.result["analysis_repair_discard"].get("state") == "discarded",
        "Analysis repair was not discarded",
    )
    session.result["acceptance"]["repair_comparison_unverified"] = True


def run_type_recovery(session: FeatureSession, address: str) -> None:
    project = digest(session.target)
    evidence = {
        "pointer_size": 8,
        "prototypes": [
            {
                "function": address,
                "convention": "system_v64",
                "parameters": [{"kind": "integer", "width": 4}],
                "returns": {"kind": "integer", "width": 4},
                "site": "synthetic fixture signature",
            }
        ],
        "accesses": [], "aliases": [], "names": [],
    }
    proposed = session.call(
        "recover_types",
        {"project": project, "action": "propose", "evidence": evidence},
        "type_recovery",
    )
    proposal_id = proposed.get("proposal")
    require(isinstance(proposal_id, str), "Type proposal returned no identifier")
    for action in ("check", "accept"):
        session.call(
            "recover_types",
            {"project": project, "action": action, "proposal": proposal_id},
            f"type_recovery_{action}",
        )
    accepted = session.result["type_recovery_accept"]
    require(accepted.get("accepted") is True, "Type proposal was not accepted")
    session.call(
        "recover_types",
        {"project": project, "action": "read", "address": address},
        "type_recovery_read",
    )
    session.result["acceptance"]["type_accepted"] = True


def run(prism: Path, output: Path, timeout: float) -> dict[str, Any]:
    profile = load_module(ROOT / "scripts" / "profile-app.py", "profile_app_features")
    smoke = load_module(ROOT / "scripts" / "smoke-rea.py", "smoke_rea_features")
    output.mkdir(parents=True, exist_ok=True)
    support = output / "support"
    support.mkdir(exist_ok=True)
    result: dict[str, Any] = {
        "status": "error", "errors": [], "acceptance": {}, "synthetic_evidence": True
    }
    session = None
    try:
        baseline, target = build_fixtures(output)
        result["fixtures"] = {
            "baseline": str(baseline), "target": str(target), "target_sha256": digest(target)
        }
        result["binary"] = {"path": str(prism), "sha256": digest(prism)}
        port = profile.free_port()
        token = secrets.token_hex(32)
        session = FeatureSession(
            prism, target, support, timeout, result, profile, smoke, port, token
        )
        with profile.profiling_environment(support, port, token):
            session.start()
            result["catalog"] = session.client.request("tools/list", {}, timeout)
            session.call("analysis_tools", {}, "provider_fingerprint")
            session.snapshot("hotpath_before")
            addresses = run_search(session)
            run_comparison(session, baseline)
            run_verification(session, addresses[0])
            run_investigation(session, addresses)
            run_execution(session, addresses[0])
            run_repair(session, addresses[0])
            run_type_recovery(session, addresses[0])
            session.snapshot("hotpath_after")
        result["status"] = "ok"
    except Exception as error:
        result["errors"].append(str(error))
    finally:
        if session is not None:
            try:
                session.close()
            except Exception as error:
                result["errors"].append(f"MCP cleanup failed: {error}")
        if result["errors"]:
            result["status"] = "error"
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prism", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--timeout", type=float, default=180.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    prism = args.prism.expanduser().resolve()
    output = args.output.expanduser().resolve()
    report = run(prism, output, args.timeout)
    report_path = output / "features.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": report["status"], "path": str(report_path), "errors": report["errors"]
    }))
    return 0 if report["status"] == "ok" else 1


if __name__ == "__main__":
    raise SystemExit(main())
