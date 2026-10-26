#!/usr/bin/env python3
"""Measure MCP payload tokens and validate analysis-tool catalog compatibility."""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import importlib.util
import json
import secrets
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PROTOCOL_VERSION = "2025-11-25"


def load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load {path}")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def json_bytes(value: Any, *, pretty: bool = False) -> bytes:
    if pretty:
        return json.dumps(value, ensure_ascii=False, indent=2).encode()
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


def measure_json(value: Any, tokenizer: Any) -> dict[str, int]:
    payload = json_bytes(value)
    pretty_payload = json_bytes(value, pretty=True)
    text = payload.decode()
    pretty_text = pretty_payload.decode()
    return {
        "bytes": len(payload),
        "chars": len(text),
        "tokens": len(tokenizer.encode(text, disallowed_special=())),
        "pretty_bytes": len(pretty_payload),
        "pretty_tokens": len(tokenizer.encode(pretty_text, disallowed_special=())),
    }


def call_payloads(
    report: dict[str, Any], tokenizer: Any
) -> tuple[list[dict[str, Any]], dict[str, int]]:
    rows: list[dict[str, Any]] = []
    request_tokens = 0
    response_tokens = 0

    for index, call in enumerate(report.get("calls", [])):
        request = {
            "method": "tools/call",
            "params": {
                "name": call.get("tool"),
                "arguments": call.get("arguments", {}),
            },
        }
        response = (
            call.get("response")
            if "response" in call
            else {"error": call.get("error")}
        )
        request_size = measure_json(request, tokenizer)
        response_size = measure_json(response, tokenizer)
        request_tokens += request_size["tokens"]
        response_tokens += response_size["tokens"]
        rows.append(
            {
                "index": index,
                "tool": call.get("tool"),
                "status": call.get("status"),
                "request": request_size,
                "response": response_size,
            }
        )

    return rows, {
        "request_tokens": request_tokens,
        "response_tokens": response_tokens,
    }


def tool_names(catalog: dict[str, Any]) -> list[str]:
    return sorted(
        item["name"]
        for item in catalog.get("tools", [])
        if isinstance(item, dict) and isinstance(item.get("name"), str)
    )


def catalog_by_name(catalog: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        item["name"]: item
        for item in catalog.get("tools", [])
        if isinstance(item, dict) and isinstance(item.get("name"), str)
    }


def default_catalog_checks(
    actual: dict[str, Any], baseline: dict[str, Any]
) -> dict[str, bool]:
    actual_tools = catalog_by_name(actual)
    baseline_tools = catalog_by_name(baseline)
    names_match = tool_names(actual) == tool_names(baseline)

    descriptions_match = names_match and all(
        actual_tools[name].get("description") == baseline_tools[name].get("description")
        for name in actual_tools
    )
    schemas_absent = all(
        "inputSchema" not in item and "outputSchema" not in item
        for item in actual.get("tools", [])
        if isinstance(item, dict)
    )
    required_fields_match = names_match and all(
        item.get("required", [])
        == baseline_tools[name].get("inputSchema", {}).get("required", [])
        for name, item in actual_tools.items()
    )

    return {
        "default_summary": names_match and descriptions_match and schemas_absent,
        "default_required_fields": required_fields_match,
    }


def live_tool(
    client: Any,
    smoke: Any,
    tokenizer: Any,
    name: str,
    arguments: dict[str, Any],
) -> tuple[Any, dict[str, int], dict[str, int]]:
    raw = client.request(
        "tools/call",
        {"name": name, "arguments": arguments},
        60,
    )
    wire_measurement = measure_json(raw, tokenizer)
    if raw.get("isError"):
        raise smoke.RpcError(json.dumps(raw, ensure_ascii=False))

    structured = raw.get("structuredContent")
    if structured is not None:
        return structured, measure_json(structured, tokenizer), wire_measurement

    text = "".join(
        block.get("text", "")
        for block in raw.get("content", [])
        if isinstance(block, dict)
    )
    parsed = json.loads(text)
    return parsed, measure_json(parsed, tokenizer), wire_measurement


def expected_rejection(
    client: Any,
    smoke: Any,
    tokenizer: Any,
    arguments: dict[str, Any],
) -> str:
    try:
        live_tool(client, smoke, tokenizer, "analysis_tools", arguments)
    except smoke.RpcError as error:
        message = str(error)
        requested_name = arguments["names"][0]
        if f"Unknown analysis tool: {requested_name}" not in message:
            raise AssertionError(f"Unexpected rejection: {message}") from error
        return message

    raise AssertionError(f"analysis_tools unexpectedly accepted {arguments!r}")


def run_catalog_acceptance(
    client: Any,
    smoke: Any,
    tokenizer: Any,
    baseline_report: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    baseline_catalog = baseline_report.get("provider_fingerprint", {})
    result: dict[str, Any] = {"checks": {}, "errors": [], "rejections": {}}
    measurements: dict[str, Any] = {}
    default: dict[str, Any] | None = None
    full: dict[str, Any] | None = None

    try:
        default, value_measurement, wire_measurement = live_tool(
            client, smoke, tokenizer, "analysis_tools", {}
        )
        result["default_names"] = tool_names(default)
        measurements["summary"] = {
            "value": value_measurement,
            "wire": wire_measurement,
        }
        result["checks"].update(default_catalog_checks(default, baseline_catalog))
    except Exception as error:
        result["errors"].append(f"summary catalog: {error}")
        result["checks"].update(
            {"default_summary": False, "default_required_fields": False}
        )

    try:
        full, value_measurement, wire_measurement = live_tool(
            client,
            smoke,
            tokenizer,
            "analysis_tools",
            {"detail": "full"},
        )
        result["full_names"] = tool_names(full)
        measurements["full"] = {
            "value": value_measurement,
            "wire": wire_measurement,
        }
        result["checks"]["full_catalog_exact"] = full == baseline_catalog
    except Exception as error:
        result["errors"].append(f"full catalog: {error}")
        result["checks"]["full_catalog_exact"] = False

    baseline_tools = catalog_by_name(baseline_catalog)
    full_tools = catalog_by_name(full) if full is not None else {}
    available_tools = full_tools or baseline_tools
    selected_name = (
        "binary_overview"
        if "binary_overview" in available_tools
        else next(iter(available_tools), "")
    )
    try:
        selected, value_measurement, wire_measurement = live_tool(
            client,
            smoke,
            tokenizer,
            "analysis_tools",
            {"detail": "full", "names": [selected_name]},
        )
        expected = full_tools.get(selected_name, baseline_tools.get(selected_name))
        expected_catalog = dict(full if full is not None else baseline_catalog)
        expected_catalog["tools"] = [expected]
        result["checks"]["named_full_exact"] = selected == expected_catalog
        measurements["named"] = {
            "name": selected_name,
            "value": value_measurement,
            "wire": wire_measurement,
        }
    except Exception as error:
        result["errors"].append(f"named catalog: {error}")
        result["checks"]["named_full_exact"] = False

    rejection_cases = {
        "unknown_name": {"detail": "full", "names": ["__unknown__"]},
        "blocked_provider": {"detail": "full", "names": ["set_comment"]},
        "batch_only_name": {
            "detail": "full",
            "names": ["procedure_pseudo_code"],
        },
    }
    for label, arguments in rejection_cases.items():
        try:
            error_text = expected_rejection(client, smoke, tokenizer, arguments)
        except Exception as error:
            result["errors"].append(f"{label} rejection: {error}")
            result["checks"][f"{label}_rejected"] = False
        else:
            result["checks"][f"{label}_rejected"] = True
            result["rejections"][label] = error_text

    if "binary_overview" in baseline_tools:
        try:
            reply, value_measurement, wire_measurement = live_tool(
                client,
                smoke,
                tokenizer,
                "analysis_call",
                {"name": "binary_overview", "arguments": {}},
            )
            result["checks"]["analysis_call_evidence_receipt"] = (
                "result" in reply and "prism_evidence" in reply
            )
            measurements["analysis_call"] = {
                "name": "binary_overview",
                "value": value_measurement,
                "wire": wire_measurement,
            }
        except Exception as error:
            result["errors"].append(f"analysis call: {error}")
            result["checks"]["analysis_call_evidence_receipt"] = False

    return result, measurements


def run_live_acceptance(
    prism: Path,
    target: Path,
    support: Path,
    baseline_report: dict[str, Any],
    tokenizer: Any,
) -> dict[str, Any]:
    profile = load_module(ROOT / "scripts" / "profile-app.py", "profile_tokens_profile")
    smoke = load_module(ROOT / "scripts" / "smoke-rea.py", "profile_tokens_smoke")
    support.mkdir(parents=True, exist_ok=True)
    result: dict[str, Any] = {
        "binary": str(prism),
        "status": "red",
        "checks": {},
        "errors": [],
    }

    port = profile.free_port()
    token = secrets.token_hex(32)
    with profile.profiling_environment(support, port, token):
        client = smoke.McpClient(prism, target, support)
        try:
            client.request(
                "initialize",
                {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "profile-tokens", "version": "1"},
                },
                60,
            )
            client.request("notifications/initialized", {}, 60, notify=True)
            acceptance, measurements = run_catalog_acceptance(
                client, smoke, tokenizer, baseline_report
            )
            result["checks"] = acceptance["checks"]
            result["errors"].extend(acceptance["errors"])
            result["rejections"] = acceptance["rejections"]
            result["catalog_measurements"] = measurements
        except Exception as error:
            result["errors"].append(str(error))
        finally:
            try:
                result["cleanup_errors"] = client.close()
            except Exception as error:
                result["cleanup_errors"] = [f"MCP cleanup failed: {error}"]
            result["errors"].extend(result["cleanup_errors"])

    result["status"] = (
        "green"
        if not result["errors"] and all(result["checks"].values())
        else "red"
    )
    return result


def measure_report(
    report_path: Path,
    output: Path,
    tokenizer: Any,
    live: dict[str, Any] | None,
) -> dict[str, Any]:
    report = json.loads(report_path.read_text())
    rows, aggregate = call_payloads(report, tokenizer)
    catalog = report.get("catalog", {})
    result = {
        "source": str(report_path.resolve()),
        "source_sha256": hashlib.sha256(report_path.read_bytes()).hexdigest(),
        "tokenizer": {
            "library": "tiktoken",
            "version": importlib.metadata.version("tiktoken"),
            "encoding": "o200k_base",
            "exact": True,
            "billing_claim": False,
        },
        "wire_note": (
            "Reports store parsed values; measured JSON is a deterministic text "
            "surrogate, not captured framing."
        ),
        "call_summary": {
            "count": len(rows),
            **aggregate,
            "expected_errors": len(report.get("expected_errors", [])),
            "unexpected_successes": len(report.get("unexpected_successes", [])),
        },
        "catalog_json": measure_json(catalog, tokenizer),
        "largest_responses": sorted(
            rows, key=lambda row: row["response"]["tokens"], reverse=True
        )[:20],
        "observed_tools": tool_names(catalog),
    }
    if live is not None:
        result["live_acceptance"] = live

    output.mkdir(parents=True, exist_ok=True)
    (output / "metrics.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--live-prism", type=Path)
    parser.add_argument("--live-target", type=Path)
    parser.add_argument("--baseline-report", type=Path)
    args = parser.parse_args()
    if bool(args.live_prism) != bool(args.live_target):
        parser.error("--live-prism and --live-target must be supplied together")
    return args


def main() -> int:
    args = parse_args()
    import tiktoken

    tokenizer = tiktoken.get_encoding("o200k_base")
    report = json.loads(args.report.read_text())
    baseline = (
        json.loads(args.baseline_report.read_text())
        if args.baseline_report
        else report
    )
    live = None
    if args.live_prism:
        live = run_live_acceptance(
            args.live_prism.resolve(),
            args.live_target.resolve(),
            args.output.resolve() / "live-support",
            baseline,
            tokenizer,
        )

    result = measure_report(args.report.resolve(), args.output.resolve(), tokenizer, live)
    print(
        json.dumps(
            {
                "calls": result["call_summary"]["count"],
                "request_tokens": result["call_summary"]["request_tokens"],
                "response_tokens": result["call_summary"]["response_tokens"],
                "live_status": live["status"] if live else None,
                "path": str(args.output.resolve() / "metrics.json"),
            }
        )
    )
    return 0 if live is None or live["status"] == "green" else 1


if __name__ == "__main__":
    raise SystemExit(main())
