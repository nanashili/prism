# Profiling Prism

Prism has an opt-in [`hotpath-rs`](https://hotpath.rs/) integration for timing,
allocation, and thread measurements. The dependency is optional and the normal
build has no profiler instrumentation. Profiling builds are separate from
release builds so reports include symbols while retaining release optimization
settings.

## Build variants

Build the desktop binary with timing instrumentation and the profiler's HTTP
MCP server:

```sh
CARGO_TARGET_DIR=build/profiling-target \
  cargo build --locked --profile profiling --features profiling-mcp --bin prism
```

Build the stdio helper without desktop dependencies:

```sh
CARGO_TARGET_DIR=build/profiling-target \
  cargo build --locked --profile profiling --no-default-features \
    --features profiling-mcp --bin prism-mcp
```

Allocation measurements are a separate run. Add `profiling-alloc` to the same
commands (and keep `profiling-mcp` when querying allocation data):

```sh
CARGO_TARGET_DIR=build/profiling-alloc-target \
  cargo build --locked --profile profiling \
    --features profiling-alloc,profiling-mcp --bin prism
```

With those commands the binaries are under
`build/profiling-target/profiling/` (or the allocation target directory). If a
shared Cargo target directory is preferred, obtain it with
`cargo metadata --format-version 1 --no-deps` and use its `target_directory`
value when passing `--prism` and `--mcp` below.

The `profiling` profile inherits `release` and enables debug information without
stripping it. This keeps optimization, thin LTO, and one codegen unit consistent
with the shipped profile while making function locations useful in reports.
`profiling-alloc` and `profiling-mcp` are opt-in feature combinations; use
separate timing and allocation runs when investigating small changes.

## Hotpath MCP

When a profiling build starts, hotpath's streamable HTTP MCP server listens on
loopback. Its default port is `6771`; set `HOTPATH_MCP_PORT` to use another
port. Set `HOTPATH_MCP_AUTH_TOKEN` to require a token. The token is sent as the
raw value of the `Authorization` header, without a `Bearer` prefix. Keep the
server on loopback, use a fresh high-entropy token for each run, and never put a
token in a command line, checked-in file, or report.

Hotpath's default function limit is 15. For the final report written at guard
shutdown, `HOTPATH_FUNCTIONS_LIMIT=0` means unlimited, while a positive value
sets a bounded list. The live MCP tools use the limit captured when the guard
is built; set `HotpathGuardBuilder::functions_limit(0)` (or another explicit
value) in the profiling entry point when full or bounded `functions_timing`
and `functions_alloc` MCP data is required. Environment variables do not
retroactively change that live worker limit.

Depending on enabled features, its tools include `profiler_status`,
`functions_timing`, `function_timing_logs`, `functions_alloc`,
`function_alloc_logs`, and `threads`. The advertised tool schema is the
authority for a particular hotpath version.

This endpoint is separate from Prism's application MCP interfaces:

* `prism-mcp` is a stdio helper for Prism's authenticated local bridge and
  exposes project, analysis, annotation, and reconstruction tools.
* hotpath MCP is a loopback HTTP server for profiler measurements and does not
  provide Prism project operations.

The guard remains alive until the entry point finishes so queued measurements
are flushed before exit. Profiling diagnostics must not corrupt an MCP stdio
protocol stream.

## Reproducible harness

`scripts/profile-app.py` runs bounded headless or desktop workloads and writes
all output below an explicitly supplied directory:

```sh
python3 scripts/profile-app.py \
  --prism build/profiling-target/profiling/prism \
  --output build/profiles/timing \
  --mode headless \
  --samples 3 \
  /absolute/path/to/sample
```

Desktop mode additionally needs the matching helper:

```sh
python3 scripts/profile-app.py \
  --prism build/profiling-target/profiling/prism \
  --mcp build/profiling-target/profiling/prism-mcp \
  --output build/profiles/timing-desktop \
  --mode desktop \
  /absolute/path/to/sample
```

For each target the harness creates a fresh `PRISM_SUPPORT` directory, chooses a
free loopback port, generates a unique token, records Prism MCP request
latencies, discovers the hotpath tool catalog, and saves before/after snapshots
including detailed logs for leading function rows. The resulting `profile.json`
is safe to archive after checking that it contains no credentials; the support
directory also contains process diagnostics and desktop captures when available.

The harness clears and restores any inherited `HOTPATH_OUTPUT_PATH` around the
profiled process and desktop child environment, because an inherited value has
precedence over the builder's output path. When starting a manual MCP profile,
unset that variable first. Hotpath's live HTTP MCP is nonblocking from the
application's stdio protocol, but its request and provider-wait timings still
measure wall-clock waits rather than CPU time.

For desktop screenshot comparisons, keep the intro disabled and allow a 150 ms
dwell after opening each view, including settings pages, before capture. This
reduces screenshots of transitional state; it does not prove that a paginated
window rendered every item in a large fixture.

Rich-code UI acceptance also requires the generated fixture's activity text to
end at a complete paragraph boundary before the reply's fenced code begins.
The fixture generator keeps that boundary explicit so a screenshot cannot
mistake an orphaned fence fragment for an application rendering defect.

The harness reuses the bounded REA smoke-test client for headless workloads. It
does not alter a user's normal profile and stops child processes on success or
failure. Use separate output directories for timing and allocation runs.

The second-pass native MCP feature harness is a separate acceptance run. It
builds two small local fixtures and records the expanded investigation,
execution-evidence, verification, repair, type-recovery, comparison, and
restart-persistence calls:

```sh
python3 scripts/profile-features.py \
  --prism build/profiling-target/profiling/prism \
  --output build/profile/round2/features-instrumented
```

Its `features.json` records MCP responses and hotpath snapshots. Imported
execution evidence is synthetic and does not represent a live debugger trace;
the type-recovery path exercises a prototype-shaped proposal, not complete
array or alias acceptance. Use the artifact to describe the tested workload,
not to claim universal feature or provider coverage.

The recorded second-pass findings, workload measurements, binary provenance,
and acceptance limits are in `docs/HOTPATH_PROFILE_ROUND2_20261026.md`.

## MCP token measurements

`analysis_tools` returns a compact discovery list by default: every allowed
tool's name, complete description, and required argument names. Fetch a tool's
complete input and output schemas before constructing unfamiliar arguments:

```json
{"detail":"full","names":["binary_overview"]}
```

Passing `{"detail":"full"}` without `names` returns the complete provider
catalog in its original format, with Prism's existing blocked-tool filters.
Named requests accept up to 32 tools. `analysis_call` retains its existing
arguments and results.

Measure one copy of each saved feature call with an isolated tokenizer
environment. The optional live probes compare the provider catalog with a
baseline feature report and exercise summary discovery, complete schemas,
named selection, and rejected names:

```sh
python3 -m venv build/profile/tokenizer-env
build/profile/tokenizer-env/bin/pip install tiktoken==0.12.0
build/profile/tokenizer-env/bin/python scripts/profile-tokens.py \
  --report build/profile/after/features.json \
  --baseline-report build/profile/before/features.json \
  --output build/profile/token-measurements \
  --live-prism /absolute/path/to/prism \
  --live-target /absolute/path/to/sample
```

Counts use `o200k_base` over deterministic JSON serialization of parsed values.
They measure payload tokens, not provider billing or a client's handling of
MCP text and structured content. Report formatting, duplicated artifact fields,
and profiler snapshots are excluded from the call totals. The latest measured
changes and validation are in the [token profile](TOKEN_PROFILE_20261026.md).

`python3 scripts/check-rust.py` also runs dedicated timing-only profiling
clippy/test checks for the desktop and standalone MCP combinations. Allocation
instrumentation is covered by the repository's all-features checks; runtime
allocation measurements still require the separate build above.

## Interpreting results

Hotpath function timing is instrumentation data (calls, sampled calls, averages,
percentiles, totals, and source locations), while allocation data reports the
instrumented allocation counters. Thread data describes profiler-observed
threads; it is not a retained-memory or whole-system measurement. Prism request
latencies in `profile.json` include protocol and external-engine work and should
be interpreted alongside hotpath snapshots.

Results depend on the target, installed analysis provider, operating system, and
provider configuration. Current acceptance work exercises local macOS targets;
it does not establish Windows or Linux behavior, complete graph availability,
live debugger traces, all type-recovery forms, or provider authentication and
paid-agent workflows. The feature harness uses synthetic imported evidence for
execution observations. A profile report records what was observed for its
workload, not universal application performance.

## Inventory paging evidence

The retained REA procedure view accepts a positive integer `limit` with no
provider schema maximum below JavaScript's safe integer range. The accepted
`4096` page size was validated against REA `6.7.0+189f3474bfc5` on the local
macOS provider: a 112,593-row inventory returned 4,096 rows with
`next_offset=4096`. The preceding 1,024-row probe returned the same total and
advanced to `next_offset=1024`. Prism now uses 4,096 for retained procedure
pages, reducing the number of provider page requests for that inventory from
about 440 to about 28.

The captured tool schema and profiling run are retained at
`build/profile/timing-before/profile.json`. The compact retained-page probe
evidence, including response hashes and elapsed times, is at
`build/profile/inventory-probe.json`. The provider binary used for the schema
and page probes was
`/Users/tihan-nico/.local/share/rea-rust/versions/6.7.0+189f3474bfc5/aarch64-apple-darwin/bin/rea`.
