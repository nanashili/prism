#!/usr/bin/env python3
"""Compare desktop RSS using fresh profiles and a running X display.

Linux only. Requires a running X server (for example Xvfb :99) and two release
desktop binaries. The same MCP helper observes both builds. Cases cover idle
Home and local source views; they do not invoke an external analysis engine.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import statistics
import subprocess
import tempfile
import time


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def probe(binary, helper, display, case):
    with tempfile.TemporaryDirectory(prefix="prism-desktop-memory-") as temporary:
        root = Path(temporary)
        # Local source views do not invoke Rizin. Selecting it avoids the default
        # REA backend's executable fingerprint requirement when opening any file.
        (root / "settings.json").write_text(json.dumps({"provider": "rizin", "desktop": {"intro_seen": True, "show_whats_new": False}, "research": {"setup_complete": True}}))
        target = root / "fixture.c"
        if case != "home":
            target.write_text("x\n" * 524288 if case == "source_short" else ("x" * 79 + "\n") * 13107)
        env = dict(os.environ, PRISM_SUPPORT=temporary, DISPLAY=display, LIBGL_ALWAYS_SOFTWARE="1", XDG_RUNTIME_DIR=temporary)
        with (root / "app.log").open("w") as log:
            command = [str(binary)] + ([str(target)] if case != "home" else [])
            app = subprocess.Popen(command, env=env, stdout=log, stderr=log, start_new_session=True)
            client = None
            try:
                started = time.monotonic()
                while not (root / "window.json").exists():
                    if app.poll() is not None:
                        raise RuntimeError((root / "app.log").read_text())
                    if time.monotonic() - started > 30:
                        raise TimeoutError("Desktop endpoint did not start")
                    time.sleep(0.05)
                client = subprocess.Popen([str(helper), "--window"], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log, text=True)

                def call(message):
                    client.stdin.write(json.dumps(message) + "\n")
                    client.stdin.flush()
                    if not select.select([client.stdout], [], [], 20)[0]:
                        raise TimeoutError("MCP response")
                    response = json.loads(client.stdout.readline())
                    if "error" in response:
                        raise RuntimeError(response)
                    return response["result"]

                call({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25"}})
                request_id = 1

                def get_state():
                    nonlocal request_id
                    request_id += 1
                    response = call({"jsonrpc": "2.0", "id": request_id, "method": "tools/call", "params": {"name": "get_state", "arguments": {}}})
                    if response.get("isError"):
                        raise RuntimeError(response)
                    return response.get("structuredContent") or json.loads(response["content"][0]["text"])

                def ready(state):
                    if state.get("error"):
                        raise RuntimeError(f"Desktop error: {state['error']}")
                    if case == "home":
                        return state.get("screen") == "Home"
                    return state.get("screen") == "Workbench" and state.get("target", {}).get("kind") == "source" and state.get("visible_text", "").startswith("x") and not state.get("pending")

                deadline = time.monotonic() + 30
                while not ready(get_state()):
                    if time.monotonic() > deadline:
                        raise TimeoutError("Desktop case did not become ready")
                    time.sleep(0.05)
                samples = []
                for _ in range(30):
                    if not ready(get_state()):
                        raise RuntimeError("Desktop left the expected state")
                    status = Path(f"/proc/{app.pid}/status").read_text()
                    samples.append({key: int(value.split()[0]) for key, value in (line.split(":", 1) for line in status.splitlines() if line.startswith(("VmRSS:", "VmHWM:")))})
                    time.sleep(0.1)
                return {"samples_kib": samples, "peak_rss_kib": max(sample["VmHWM"] for sample in samples), "settled_rss_kib": statistics.median(sample["VmRSS"] for sample in samples[-10:])}
            finally:
                if client is not None:
                    client.stdin.close()
                    try:
                        client.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        client.kill()
                        client.wait()
                if app.poll() is None:
                    os.killpg(app.pid, signal.SIGTERM)
                try:
                    app.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(app.pid, signal.SIGKILL)
                    app.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--helper", type=Path, required=True)
    parser.add_argument("--display", default=os.environ.get("DISPLAY", ":99"))
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--cases", nargs="+", choices=["home", "source_typical", "source_short"], default=["home", "source_typical", "source_short"])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binaries = {"baseline": args.baseline.resolve(), "candidate": args.candidate.resolve()}
    result = {"method": "Fresh profiles, provider=rizin (no engine invoked), X11 software rendering, 30 MCP polls at 100ms after expected state; desktop PID only; settled RSS median of final 10 samples", "binaries": {mode: sha256(binary) for mode, binary in binaries.items()}, "helper_sha256": sha256(args.helper), "cases": {}}
    for case in args.cases:
        samples = {mode: [] for mode in binaries}
        for run in range(args.runs):
            for mode in (["baseline", "candidate"] if run % 2 == 0 else ["candidate", "baseline"]):
                sample = probe(binaries[mode], args.helper.resolve(), args.display, case)
                samples[mode].append(sample)
                print(f"{case} {mode} {run + 1}: settled={sample['settled_rss_kib'] / 1024:.2f} MiB peak={sample['peak_rss_kib'] / 1024:.2f} MiB", flush=True)
        result["cases"][case] = {"samples": samples, "medians": {mode: {key: statistics.median(sample[key] for sample in rows) for key in ["peak_rss_kib", "settled_rss_kib"]} for mode, rows in samples.items()}}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
