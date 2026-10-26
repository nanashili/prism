#!/usr/bin/env python3
"""Capture one already running thread in an explicitly authorized isolated environment.

Requires the optional Frida Python package. Never spawns or resumes an executable.
Import the resulting JSON object using Prism's execution_evidence action=import.
"""

import argparse
import hashlib
import json
import math
from pathlib import Path
import sys
import time
from datetime import datetime, timezone
from uuid import uuid4


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--authorized-isolated", action="store_true", required=True,
                        help="Acknowledge explicit authorization and an isolated target environment")
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--thread", type=int, required=True)
    parser.add_argument("--target", type=Path, required=True,
                        help="Exact loaded module file on this machine; its SHA-256 binds the trace")
    parser.add_argument("--static-base", required=True,
                        help="Preferred module base in Prism's static address space, in hexadecimal")
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--seconds", type=float, default=10)
    parser.add_argument("--mode", choices=("calls", "blocks", "instructions"), default="calls")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.pid <= 0 or args.thread <= 0 or args.thread > 2**53 - 1:
        parser.error("PID and thread must be positive; thread must fit a JavaScript safe integer")
    if not math.isfinite(args.seconds) or not 0 < args.seconds <= 300:
        parser.error("Capture duration must be between 0 and 300 seconds")
    try:
        base = int(args.static_base, 16)
    except ValueError:
        parser.error("--static-base must be hexadecimal")
    if not 0 <= base < 2**64:
        parser.error("--static-base must fit an unsigned 64-bit address")
    args.static_base = hex(base)
    if not args.workflow.strip() or len(args.workflow.encode()) > 1024:
        parser.error("--workflow must contain 1 to 1024 UTF-8 bytes")
    args.target = args.target.resolve(strict=True)
    args.output = args.output.resolve()
    if args.output.exists():
        parser.error("--output must be a new file; existing capture data is never overwritten")
    if not args.output.parent.is_dir():
        parser.error("--output parent directory must exist")
    return args


def digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def write_trace(args: argparse.Namespace, metadata: dict, observations: dict, target_hash: str) -> None:
    trace = {
        "version": 1,
        "id": str(uuid4()),
        "workflow": args.workflow,
        "source": "frida_stalker",
        "runtime_version": metadata["version"],
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "target_sha256": target_hash,
        "mapping": {
            "runtime_base": metadata["runtime_base"],
            "static_base": args.static_base,
            "size": metadata["size"],
        },
        **observations,
    }
    if not trace["events"]:
        raise RuntimeError("No target-module observations were captured; no trace was written")
    data = json.dumps(trace, separators=(",", ":")) + "\n"
    if len(data.encode()) > 2 * 1024 * 1024:
        raise RuntimeError("Capture exceeds Prism's 2 MiB trace limit")
    with args.output.open("x", encoding="utf-8") as output:
        output.write(data)
        output.flush()
    print(f"Saved {len(trace['events'])} observation records to {args.output}", flush=True)


def close_capture(script, session) -> None:
    failures = []
    for resource, operation in ((script, "unload"), (session, "detach")):
        if resource is None:
            continue
        try:
            getattr(resource, operation)()
        except Exception as error:
            error.add_note(f"Frida cleanup: {operation}")
            failures.append((operation, error))
    if failures:
        active = sys.exception()
        if active is not None:
            for operation, failure in failures:
                active.add_note(f"Frida cleanup {operation} failed: {failure!r}")
        else:
            raise ExceptionGroup("Frida cleanup failed; any saved trace was retained",
                                 [failure for _, failure in failures])


def capture(frida, args: argparse.Namespace) -> None:
    target_hash = digest(args.target)
    source = Path(__file__).with_name("frida-stalker.js").read_text(encoding="utf-8")
    session = frida.attach(args.pid)
    script = None
    try:
        script = session.create_script(source)
        script.load()
        metadata = script.exports_sync.start(str(args.target), args.thread, args.mode)
        print(f"Frida {metadata['version']}: capturing thread {args.thread} for {args.seconds:g}s", flush=True)
        interrupted = False
        try:
            time.sleep(args.seconds)
        except KeyboardInterrupt:
            interrupted = True

        observations = script.exports_sync.stop()
        if error := observations.pop("error", None):
            print(f"Partial capture: {error}", file=sys.stderr, flush=True)
        if interrupted or digest(args.target) != target_hash:
            observations["complete"] = False
        write_trace(args, metadata, observations, target_hash)
    finally:
        close_capture(script, session)


def main() -> None:
    args = arguments()
    # Import only after parsing authorization; --help and rejected invocations have no runtime effects.
    import frida

    capture(frida, args)


if __name__ == "__main__":
    main()
