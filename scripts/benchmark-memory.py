#!/usr/bin/env python3
"""Measure baseline/candidate workloads in fresh Linux processes, serially.

Baseline implementations are test-only copies of cc2c4f6. Fixtures are generated
before measurement. Peak RSS includes fixture loading and correctness checks;
elapsed_ms in workload output covers the operation described in MEMORY_AUDIT.md.
"""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
CASES = {
    "project_undo": ("ui::agent_dock::inspector::notes::tests::memory_project_undo", "undo.json"),
    "project_save": ("project::document_save::tests::memory_project_save", "project.json"),
    "archive_json": ("project::archive::tests::memory_archive_json", "archive.json"),
    "archive_chats": ("migration::chats::tests::memory_archive_chat_import", "chats.json"),
    "android_inventory": ("analysis::android::tests::memory_android_inventory", "android.json"),
    "android_source": ("analysis::android::source::tests::memory_android_source", "android.json"),
    "rpc_queue": ("protocol::incoming::tests::memory_rpc_queue", "frames.jsonl"),
    "lines_typical": ("ui::view_tab::lines::tests::memory_line_index", "typical.c"),
    "lines_short": ("ui::view_tab::lines::tests::memory_line_index", "short.c"),
    "analysis_view": ("worker::analysis::view::tests::memory_bounded_analysis_view", "view.json"),
    "analysis_view_nested": ("worker::analysis::view::tests::memory_bounded_analysis_view", "nested-view.json"),
}


def command_output(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def source_hash():
    digest = hashlib.sha256()
    for path in sorted((ROOT / "src").rglob("*.rs")):
        digest.update(str(path.relative_to(ROOT)).encode() + b"\0")
        digest.update(path.read_bytes() + b"\0")
    return digest.hexdigest()


def build(destination):
    command = ["cargo", "test", "--release", "--locked", "--bin", "prism", "--no-run", "--message-format=json"]
    with (destination / "build.jsonl").open("w") as log:
        subprocess.run(command, cwd=ROOT, stdout=log, check=True)
    for line in (destination / "build.jsonl").read_text().splitlines():
        item = json.loads(line)
        if item.get("reason") == "compiler-artifact" and item.get("executable") and item["target"]["name"] == "prism" and item["profile"]["test"]:
            return Path(item["executable"])
    raise RuntimeError("Release test executable not found")


def fixtures(directory, binary):
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / "undo.json").open("w") as stream:
        json.dump({"source": "x" * (12 * 1024 * 1024), "unknown": [1, None, "é"]}, stream)
    with (directory / "project.json").open("w") as stream:
        stream.write('{"records":[')
        for index in range(60000):
            if index:
                stream.write(",")
            json.dump({"address": hex(index * 16), "metadata": {"name": f"function_{index}", "flags": [True, False], "description": "record " * 6}}, stream)
        stream.write("]}")
    with (directory / "archive.json").open("w") as stream:
        json.dump({"chunks": ["a" * 65536] * 384}, stream)
    with (directory / "chats.json").open("w") as stream:
        stream.write("[")
        for index in range(128):
            if index:
                stream.write(",")
            json.dump({"id": str(index), "title": f"Thread {index}", "project": "project", "updatedAt": 1000, "messages": [{"kind": "user", "text": "m" * (512 * 1024)}]}, stream)
        stream.write("]")
    with (directory / "frames.jsonl").open("w") as stream:
        for _ in range(32):
            stream.write("[" + "0," * 524287 + "0]\n")
    (directory / "typical.c").write_text(("x" * 79 + "\n") * 13107)
    (directory / "short.c").write_text("x\n" * 524288)
    # Ordinary repeated records, plus a separate indentation-expansion stress case.
    with (directory / "view.json").open("w") as stream:
        json.dump({"normalized_result": [{"name": "entry", "data": {"values": list(range(32))}}] * 40000}, stream)
    env = dict(os.environ, PRISM_MEMORY_FIXTURE=str(directory / "android.json"))
    subprocess.run([str(binary), "analysis::android::tests::generate_android_memory_fixture", "--exact", "--ignored", "--nocapture"], env=env, cwd=ROOT, check=True)
    env["PRISM_MEMORY_FIXTURE"] = str(directory / "nested-view.json")
    subprocess.run([str(binary), "worker::analysis::view::tests::generate_analysis_view_memory_fixture", "--exact", "--ignored", "--nocapture"], env=env, cwd=ROOT, check=True)


def measure(launcher, binary, case, mode, fixture, log):
    env = dict(os.environ, PRISM_MEMORY_IMPL=mode, PRISM_MEMORY_FIXTURE=str(fixture))
    usage_path = log.with_suffix(".rss.json")
    command = [str(launcher), str(usage_path), str(binary), case, "--exact", "--ignored", "--nocapture", "--test-threads=1"]
    started = time.monotonic()
    with log.open("w") as stream:
        process = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=stream)
    output = log.read_text()
    if process.returncode:
        raise RuntimeError(f"{case}/{mode} failed ({process.returncode}):\n{output}")
    if "1 passed" not in output:
        raise RuntimeError(f"Workload did not run: {case}\n{output}")
    elapsed = re.search(r"elapsed_ms=([0-9.]+)", output)
    if not elapsed:
        raise RuntimeError(f"Missing operation timing: {output}")
    return {**json.loads(usage_path.read_text()), "operation_ms": float(elapsed[1]), "process_ms": (time.monotonic() - started) * 1000, "output": output}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/memory")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--binary", type=Path, help="Use an already built release test executable")
    parser.add_argument("--cases", nargs="+", choices=CASES, default=list(CASES))
    args = parser.parse_args()
    if platform.system() != "Linux" or args.runs < 1:
        parser.error("Requires Linux wait4 RSS units and a positive run count")
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    binary = args.binary.resolve() if args.binary else build(destination)
    launcher = destination / "measure-rss"
    subprocess.run(["cc", "-O2", "-Wall", "-Wextra", "-Werror", str(ROOT / "scripts/measure-rss.c"), "-o", str(launcher)], check=True)
    fixture_dir = destination / "fixtures"
    fixtures(fixture_dir, binary)
    result = {
        "environment": {"utc": datetime.datetime.now(datetime.timezone.utc).isoformat(), "platform": platform.platform(), "logical_cpus": os.cpu_count(), "rustc": command_output("rustc", "-Vv"), "baseline": "cc2c4f6638c3c691de70ad5ab3a973859c247f3d", "candidate_head": command_output("git", "rev-parse", "HEAD"), "candidate_src_sha256": source_hash(), "binary_sha256": hashlib.file_digest(binary.open("rb"), "sha256").hexdigest(), "method": "fresh process per sample via small C launcher; Linux wait4 ru_maxrss; serial alternating order; release defaults"},
        "fixtures": {path.name: {"bytes": path.stat().st_size, "sha256": hashlib.file_digest(path.open("rb"), "sha256").hexdigest()} for path in sorted(fixture_dir.iterdir())},
        "cases": {},
    }
    result["environment"]["launcher_src_sha256"] = hashlib.sha256((ROOT / "scripts/measure-rss.c").read_bytes()).hexdigest()
    result["environment"]["cc"] = command_output("cc", "--version")
    for name in args.cases:
        test, fixture = CASES[name]
        samples = {"baseline": [], "candidate": []}
        for run in range(args.runs):
            for mode in (["baseline", "candidate"] if run % 2 == 0 else ["candidate", "baseline"]):
                sample = measure(launcher, binary, test, mode, fixture_dir / fixture, destination / f"{name}-{mode}-{run + 1}.log")
                samples[mode].append(sample)
                print(f"{name} {mode} {run + 1}: {sample['peak_rss_kib'] / 1024:.2f} MiB, {sample['operation_ms']:.3f} ms", flush=True)
        # Hashes emitted by workloads validate retained/imported data on every run.
        digests = {digest for rows in samples.values() for row in rows for digest in re.findall(r"(?:checksum|digest|semantic_sha256)=([0-9a-f]{64})", row["output"])}
        if len(digests) > 1:
            raise RuntimeError(f"Semantic digest mismatch for {name}: {digests}")
        result["cases"][name] = {"test": test, "fixture": fixture, "samples": samples, "medians": {mode: {key: statistics.median(row[key] for row in rows) for key in ["peak_rss_kib", "operation_ms", "process_ms"]} for mode, rows in samples.items()}}
        (destination / "results.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"Results: {destination / 'results.json'}", flush=True)


if __name__ == "__main__":
    main()
