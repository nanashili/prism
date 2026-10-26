#!/usr/bin/env python3
"""Run serial release benchmarks against the original implementation.

UI benchmarks run on a detached baseline with only the test harness overlaid.
Other benchmarks include a paired copy of the original algorithm/schema.
Results are observations, not timing assertions suitable for shared CI runners.
"""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
BASELINE = "4c0040dacea5dc22fe61e0ff1cf3fd5a6a380e34"


def output(*command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def run_logged(command, cwd, env, path):
    print(f"Running in {cwd}: {' '.join(command)}", flush=True)
    with path.open("w") as log:
        result = subprocess.run(command, cwd=cwd, env=env, stdout=log, stderr=log)
    print(path.read_text(), flush=True)
    result.check_returncode()


def clean_package(cwd, env):
    # Worktrees have the same package name and test-binary paths. Force Prism
    # itself to rebuild when switching source trees; retain compiled dependencies.
    subprocess.run(
        ["cargo", "clean", "--release", "--package", "prism"],
        cwd=cwd,
        env=env,
        check=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-ref", default=BASELINE)
    parser.add_argument("--output", type=Path, default=ROOT / "target/performance")
    args = parser.parse_args()
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(
        Path(env.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
    )
    sources = hashlib.sha256()
    for path in sorted((ROOT / "src").rglob("*.rs")):
        sources.update(str(path.relative_to(ROOT)).encode() + b"\0")
        sources.update(path.read_bytes() + b"\0")
    metadata = {
        "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "platform": platform.platform(),
        "logical_cpus": os.cpu_count(),
        "rustc": output("rustc", "-Vv"),
        "baseline": output("git", "rev-parse", args.baseline_ref),
        "candidate_head": output("git", "rev-parse", "HEAD"),
        "candidate_status": output("git", "status", "--short"),
        "candidate_src_sha256": sources.hexdigest(),
        "profile": "repository release defaults; benchmarks run serially",
    }
    (destination / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    command = ["cargo", "test", "--release", "--locked"]
    test_args = ["benchmark_", "--", "--ignored", "--nocapture", "--test-threads=1"]
    with tempfile.TemporaryDirectory(prefix="prism-performance-") as temporary:
        baseline = Path(temporary) / "baseline"
        subprocess.run(
            ["git", "worktree", "add", "--detach", str(baseline), args.baseline_ref],
            cwd=ROOT,
            check=True,
        )
        try:
            for module in ["src/ui/syntax", "src/ui/message/markdown"]:
                tests = Path(module) / "tests.rs"
                (baseline / tests).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / tests, baseline / tests)
                with (baseline / f"{module}.rs").open("a") as source:
                    source.write("\n#[cfg(test)]\nmod tests;\n")
            clean_package(baseline, env)
            run_logged(
                command + ["--bin", "prism"] + test_args,
                baseline,
                env,
                destination / "baseline.log",
            )
        finally:
            subprocess.run(
                ["git", "worktree", "remove", "--force", str(baseline)],
                cwd=ROOT,
                check=True,
            )
    clean_package(ROOT, env)
    run_logged(
        command + ["--all-targets"] + test_args,
        ROOT,
        env,
        destination / "candidate.log",
    )
    candidate_log = (destination / "candidate.log").read_text()
    for benchmark in [
        "benchmark_graph_lines",
        "benchmark_mcp_receive",
        "benchmark_project_index",
        "benchmark_reconstruction_invalidation",
        "benchmark_exact_function_lookup",
        "benchmark_without_syntax_hints",
        "benchmark_syntax_highlighting",
    ]:
        if benchmark not in candidate_log:
            raise RuntimeError(f"Candidate output is missing {benchmark}")
    print(f"Benchmark logs: {destination}")


if __name__ == "__main__":
    main()
