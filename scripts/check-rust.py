#!/usr/bin/env python3
"""Check every maintained Rust crate and record source coverage and command evidence."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
import shlex
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VENDOR = Path("vendor/egui_commonmark_extended")


@dataclass(frozen=True)
class Check:
    name: str
    arguments: tuple[str, ...]
    environment: dict[str, str] | None = None


def source_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    paths = {Path(os.fsdecode(value)) for value in result.stdout.split(b"\0") if value}
    return sorted(path for path in paths if (ROOT / path).is_file())


def shared_lint_arguments() -> tuple[str, ...]:
    """Use the root manifest as the single lint policy for both crates."""
    with (ROOT / "Cargo.toml").open("rb") as manifest:
        policy = tomllib.load(manifest)["lints"]

    arguments = ["-D", "warnings"]
    levels = {"allow": "-A", "warn": "-W", "deny": "-D", "forbid": "-F"}
    for namespace, settings in policy.items():
        ordered = sorted(
            settings.items(),
            key=lambda item: item[1].get("priority", 0) if isinstance(item[1], dict) else 0,
        )
        for name, setting in ordered:
            level = setting["level"] if isinstance(setting, dict) else setting
            lint = f"clippy::{name}" if namespace == "clippy" else name
            arguments.extend((levels[level], lint))
    return tuple(arguments)


def crate_checks(label: str, manifest: Path, features: tuple[str, ...]) -> list[Check]:
    common = ("--manifest-path", str(manifest), "--locked")
    lints = shared_lint_arguments()
    return [
        Check(f"{label}-fmt", ("cargo", "fmt", "--manifest-path", str(manifest), "--check")),
        Check(f"{label}-clippy", ("cargo", "clippy", *common, "--all-targets", *features, "--", *lints)),
        Check(f"{label}-test", ("cargo", "test", *common, *features)),
        Check(
            f"{label}-doc",
            ("cargo", "doc", *common, "--no-deps", *features),
            {"RUSTDOCFLAGS": "-D warnings"},
        ),
    ]


def checks() -> list[Check]:
    root = Path("Cargo.toml")
    vendor_manifest = VENDOR / "Cargo.toml"
    result = crate_checks("prism", root, ("--all-features",))
    result += crate_checks("mcp", root, ("--no-default-features",))
    result += crate_checks("desktop", root, ("--no-default-features", "--features", "desktop"))
    # Allocation profiling changes hotpath's implementation. Check the timing-only
    # integrations separately from --all-features, including the standalone MCP.
    for label, features in (
        ("profiling", ("--features", "profiling-mcp")),
        ("mcp-profiling", ("--no-default-features", "--features", "profiling-mcp")),
    ):
        result += [
            check
            for check in crate_checks(label, root, features)
            if check.name.endswith(("-clippy", "-test"))
        ]
    result += crate_checks(
        "markdown", vendor_manifest, ("--no-default-features", "--features", "pulldown_cmark"),
    )

    # The upstream `nightly` macro mode deliberately requires a nightly compiler.
    # Validate every stable additive feature with the repository's pinned toolchain.
    with (ROOT / vendor_manifest).open("rb") as manifest:
        vendor_features = tomllib.load(manifest)["features"]
    stable = ",".join(sorted(set(vendor_features) - {"default", "nightly", "dump-macro"}))
    result += crate_checks(
        "markdown-stable", vendor_manifest,
        ("--no-default-features", "--features", stable),
    )
    return result


def validate_inventory(paths: list[Path]) -> None:
    for path in paths:
        if path.suffix == ".rs" and not (
            path == Path("build.rs") or path.is_relative_to("src") or path.is_relative_to(VENDOR / "src")
        ):
            raise ValueError(f"Rust source has no configured crate check: {path}")
        if path.suffix == ".py":
            ast.parse((ROOT / path).read_text(encoding="utf-8"), filename=str(path))

    manifests = {path for path in paths if path.name == "Cargo.toml"}
    expected = {Path("Cargo.toml"), VENDOR / "Cargo.toml"}
    if manifests != expected:
        raise ValueError(f"Update the crate checks for these manifests: {sorted(manifests ^ expected)}")


def run_check(check: Check, directory: Path) -> dict[str, object]:
    log = directory / f"{check.name}.log"
    print(f"Checking {check.name}…", flush=True)
    environment = os.environ.copy()
    environment.update(check.environment or {})
    with log.open("w", encoding="utf-8") as output:
        output.write(f"$ {shlex.join(check.arguments)}\n")
        output.flush()
        result = subprocess.run(
            check.arguments, cwd=ROOT, env=environment,
            stdout=output, stderr=subprocess.STDOUT, check=False,
        )
    if result.returncode:
        print("\n".join(log.read_text(encoding="utf-8").splitlines()[-35:]), file=sys.stderr)
    status = "passed" if result.returncode == 0 else "failed"
    print(f"{check.name}: {status} ({log})", flush=True)
    return {"name": check.name, "command": check.arguments, "status": status, "log": str(log)}


def write_coverage(paths: list[Path], results: list[dict[str, object]], directory: Path) -> None:
    sources = []
    for path in paths:
        if path.suffix != ".rs":
            continue
        content = (ROOT / path).read_bytes()
        sources.append({
            "path": path.as_posix(),
            "sha256": hashlib.sha256(content).hexdigest(),
            "lines": len(content.splitlines()),
            "crate": "markdown" if path.is_relative_to(VENDOR) else "prism",
        })
    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "coverage_kind": "automated crate checks; manual review is recorded separately",
        "project_files": len(paths),
        "rust_files": sources,
        "checks": results,
    }
    (directory / "coverage.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report-dir", type=Path, default=ROOT / "build/rust-review")
    arguments = parser.parse_args()
    directory = arguments.report_dir.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    paths = source_files()
    validate_inventory(paths)
    results = [run_check(check, directory) for check in checks()]
    write_coverage(paths, results, directory)
    return int(any(result["status"] != "passed" for result in results))


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Rust checks could not complete: {error}", file=sys.stderr)
        sys.exit(1)
