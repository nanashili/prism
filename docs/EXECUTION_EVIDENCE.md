# Static and dynamic evidence correlation

Prism retains workflow observations alongside its static analysis. `execution_evidence`
is available in the analysis MCP catalog. It supports `import`, `list`, `read`,
`remove`, `functions`, `graph`, and `capture_debugger` actions. Evidence is stored in
the local profile database, keyed by target SHA-256; it survives restarts and view
cache eviction. It is not currently embedded in exported `.prism` project documents.

## Import and inspect

Import the JSON object below as the tool's `trace` argument with `action: "import"`.
Use a fresh immutable ID for each capture. Replace the hash and mapping with those
of the actual analyzed native module.

```json
{
  "version": 1,
  "id": "open-document-1",
  "workflow": "Open a document through the file picker",
  "source": "imported",
  "runtime_version": "collector version",
  "captured_at": "2026-10-26T10:00:00Z",
  "target_sha256": "<SHA-256 of the open native target>",
  "mapping": {
    "runtime_base": "0x700000000",
    "static_base": "0x100000000",
    "size": 65536
  },
  "complete": false,
  "events": [
    {
      "kind": "call",
      "address": "0x700001010",
      "target": "0x700002000",
      "thread_id": 1,
      "count": 2
    }
  ]
}
```

The mapping describes one module: `static = static_base + (runtime - runtime_base)`.
Bases and observation addresses are hexadecimal strings, preserving all 64 address
bits across JSON clients. `size` is an unsigned byte count, and the module's upper
bound is exclusive. The static base is the module base used by the analysis
provider, not the entry point or the first executable section. Overflowing ranges,
unknown fields, invalid event shapes, and mismatched hashes are rejected before
persistence. Imported mappings and collector metadata are supplied evidence; a
matching hash field does not independently attest the running process's memory.

Kinds are `call`, `block`, `instruction`, `function`, and `sample`. A call requires
`target`; other kinds must omit it. Every observation includes `thread_id` and a
positive `count` of at most one billion. Each trace is limited to 10,000 records
and 2 MiB; each target retains at most 16 traces. Remove an old trace explicitly
before importing beyond that limit. An import never replaces an existing ID.

Use `action: "functions", trace_id: "..."` to page executed indexed entries, and
follow `next_offset` until null. Only exact observed entry addresses match; a PC
is never assigned to the nearest symbol. Debugger samples do not establish entry
execution. Functions reached only in the middle may be absent from this entry
list, while their function graph still reports observations inside its blocks.

Use `action: "graph", address: "0x..."` for a saved indexed function. Its `dynamic`
array retains per-trace provenance, observed blocks, observed call sites and
targets, and the number of records whose source address lies outside the target
module. Calls entering from other modules still establish an observed target.
Calls leaving the target retain their runtime address and a null static target.
Static CFG successors, callers, and callees are preserved separately.

The Graph view highlights observed blocks and lists workflow evidence and call
targets. Importing or removing evidence refreshes open graph tabs; each subsequent
graph read reapplies current evidence to the static cache.

## Existing debugger sessions

Launch or attach through Prism's debugger UI only after explicitly authorizing
debugging in an appropriately isolated environment. Prism does not provision a
sandbox. The isolation acknowledgment gates both Launch and Attach.

`capture_debugger` accepts `trace_id`, `workflow`, `runtime_version`, `thread_id`,
and `mapping`. The debugger must have launched the open project with a matching
file hash, and the selected thread must be stopped. Attached processes are refused
because their target identity has not been established. This command requests threads and the top stack
frame; it never launches, attaches, resumes, steps, or evaluates code. The frame's
instruction address becomes a single `sample` observation. The resulting trace is
always partial. Unwound frames and return addresses are not recorded as executed
calls. The runtime version and mapping are supplied by the caller.

## Optional Frida Stalker capture

The optional collector uses the [Frida Stalker API](https://frida.re/docs/javascript-api/#stalker).
It requires Python 3.11 or later and the Frida Python package in the operator's environment. No Frida
package is added to Prism's build or started automatically.

Run or instrument executable targets only with explicit authorization in an
appropriately isolated environment. The collector requires `--authorized-isolated`
to acknowledge both conditions; that flag does not create isolation. The operator
must launch the target separately, select its process and thread, and perform the
workflow during the capture interval. The collector attaches locally and never
spawns or resumes a target.

```bash
python3 scripts/trace-frida.py \
  --authorized-isolated \
  --pid 1234 --thread 5678 \
  --target /isolated/path/to/executable \
  --static-base 0x100000000 \
  --workflow "Open a document" \
  --seconds 10 --mode blocks \
  --output /isolated/results/open-document.json
```

The loaded module path must match the local target file. The collector hashes
that file, records `Frida.version`, resolves the live module base and size, and
collects call events plus optional block or instruction events. The default
`calls` mode reduces tracing overhead. Addresses are aggregated without converting
pointers through JavaScript numbers. Output files are created exclusively and
never overwrite a prior capture. Cleanup attempts script unload and session detach
independently, retaining cleanup errors and any saved output.

Import the saved JSON object with `execution_evidence`. A successful interval sets
`complete` true only if the collector's own record/count limits were not exceeded.
Interrupting the interval produces partial output. `complete` does not attest that
Stalker's event queue lost no events, that other threads were covered, or that the
workflow exercised every path. A process exit or transport failure before capture
retrieval can prevent output; live instrumentation acceptance requires a separately
authorized environment and has not been inferred from deterministic tests.

Offline collector checks run with `node --test scripts/frida-stalker.test.cjs` and
`python3 scripts/test-trace-frida.py`. These cover address precision, filtering,
buffer draining, partial evidence, and independent cleanup using supplied events
and protocol doubles; they do not exercise Frida injection or target execution.

## Interpretation

Observation counts mix block visits, instruction events, calls, and samples; they
are not exact invocation counts. An observed call target can resolve where a
statically indirect call went in this run, but the observation alone does not
classify that instruction as direct or indirect. Compare its site with static
assembly. An unobserved function or block is not proven unreachable. No observation
replaces static evidence or proves the collector's provenance independently.
