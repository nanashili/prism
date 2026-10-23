# Decompilation throughput and durable export research

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Status: historical design research with a later source comparison, inspected
2026-10-19. The initial executive findings, local-facts table, and proposed
durable-job design below describe the pre-durable implementation. Durable jobs
have since shipped; the four-result limit applies to the interactive endpoint,
not to the durable runner's client-call count. Use
[the current performance comparison](REFERENCE_PERFORMANCE_COMPARISON.md) and
[the native critical-path audit](REFERENCE_DECOMP_CRITICAL_PATH.md) for the current
implementation, measured results, and remaining work. This note covers Prism's
decompilation path and the cost of exporting large function inventories. It
uses local Prism source and public primary documentation. It does not use a
target binary, proprietary decompiler output, or copied implementation code.

## Executive findings

Prism's current `decompile_batch` is a pagination envelope around a sequential
loop. A request may contain 1 to 64 addresses, but each reply contains at most
four results. Each result calls `read_view(View::Pseudocode, ...)` on the same
worker before the next address is processed. This reduces MCP call overhead;
it does not create decompiler parallelism.

The native path has one mutable Ghidra `Program` per image and one synchronous
RPC channel. `Program::decompile` mutates the program's current function state,
and Prism releases that function state after each call. Parallel calls against
one `Program` would violate this ownership model. Actual parallelism therefore
requires multiple isolated native sessions, each with its own program state,
or a future upstream API that explicitly supports concurrent decompilation.

The analysis checkpoint stores a serialized program snapshot, discovery data,
call data, and seeded addresses. Pseudocode and assembly are also persisted by
the analysis reply cache in SQLite when they fit the configured cache budget.
That cache survives process restarts, but it is an evictable LRU cache rather
than a durable export manifest. The highest confidence throughput improvement
is a durable, resumable export job that records completion and artifact
identity explicitly, while reusing existing cached views whenever available.

The one-hour target for 200,000 successful unique pseudocode results is
55.56 results per second. With the current four-result page limit, that is at
least 50,000 MCP replies per hour, or 13.89 replies per second, before worker
time, serialization, retries, and model interaction. No local measurement
establishes that the target is achievable for an arbitrary corpus.

## Local facts and limits

The following observations are from the current source tree.

| Area | Current behavior | Consequence |
| --- | --- | --- |
| MCP batch input | 1 to 64 validated addresses | Large jobs must retain a cursor outside the request |
| Batch page | Four items and a 1008 KiB reply budget | A large function can reduce the page below four |
| Decomp execution | Sequential `read_view` calls | More addresses in one request do not increase CPU concurrency |
| Worker ownership | One state thread and one engine | The mutable native session is serialized |
| Native transport | Synchronous `Rpc::call` | Each native request waits for completion |
| Native pseudocode cache | Stored by `analysis_call` and read by `read_view` | Warm requests can avoid native decomp; eviction or key mismatch causes a miss |
| Checkpoint | Zstandard JSON, up to 512 MiB, restored as one `Saved` value | Restore requires a large decode and allocation phase |
| Checkpoint lock | Exclusive per image | Two sessions cannot safely restore the same checkpoint today |
| Function inventory | Native function pages of 256, requested serially | Inventory RPC overhead is separate from decomp cost |
| Caller index | Full entry scan is sequential | Caller analysis must not be hidden in every function export |

The concrete batch implementation is
[`src/worker/decompile_batch.rs`](../src/worker/decompile_batch.rs), lines
8-58. The cache and native routing are in
[`src/worker/analysis.rs`](../src/worker/analysis.rs), lines 53-210. The
single native session is defined in
`src/analysis/native_ghidra/session.rs`,
and RPC calls are made by
`src/analysis/native_ghidra/client.rs`.
The checkpoint format and lock are in
`src/analysis/native_ghidra/checkpoint.rs`,
lines 12-155.

The Rust wrapper's `Program::decompile` implementation is in
[`vendor/ghidra-decompiler/src/program.rs`](../vendor/ghidra-decompiler/src/program.rs),
lines 779-805. It resolves a function, runs the current decompiler action,
prints the result, and clears the current analysis state. Prism's
`function.rs` releases the
function object after the call. These are implementation facts, not an
assumption that a Rust thread pool can safely share a `Program`.

The full call chain is important. `read_view` checks the provider view cache
first. On a miss, `engine_view` calls `analysis_call`; for Ghidra,
`analysis_call` checks the same cache before the native RPC and stores the
normalized pseudocode or assembly reply after a successful call.
`read_view`'s `native_code` branch only avoids writing the same value twice; it
does not disable persistence. The cache uses `(project image SHA-256,
normalized address, provider:view)` as its SQLite primary key, compresses the
payload, limits an individual entry to 2 MiB, and trims by access time to the
configured budget. It is therefore restart-persistent but evictable and does
not include a separate options digest in the key.

The checkpoint remains an analysis-state cache, not a completion manifest. It
does not record which addresses a job requested, whether each export was
published, or which exact action and output digest produced an artifact.

## Reference versus Prism: actual decompilation paths

This comparison is limited to the application code that is present in the two
trees. Reference's REA service implementation is external to the application
repository, so the app source establishes its request contract, caching, and
worker ownership, but not the service's internal decompiler algorithm.

### Reference's bulk path

Reference's `Rea.batchDecompile` uses an outer
chunk size of 256 addresses. Each chunk is sent to REA through one
`batch_decompile` call with `detail: "summary"`. When REA returns retained
evidence, the app retrieves it with `inspect_analysis_view` pages of 64
addresses. The app queue still serializes MCP calls, but this contract reduces
one-function request overhead and leaves batching inside the REA service. The
app checks the returned count and yields individual items without placing the
whole export in one response. The same file enforces the service contract at
`batchSize` values of at least 64 and multiples of 64.

Reference also configures the external service with a bounded Ghidra decompiler
thread count derived from host memory and available processors. In
`analysisResources.ts`, the
default is 4 threads on hosts below 32 GiB and 8 threads on larger hosts,
clamped by available parallelism. Its broker creates one worker per target and
shares that worker between clients, so opening the same binary does not imply a
second import. These facts explain lower app-side round-trip cost and fewer
duplicate sessions. They do not prove that REA internally parallelizes the
256-function request, because that implementation is not in this repository.

The app's restart reuse is split across two file caches. Procedure inventories
are stored as JSON by `procedures.ts`,
while pseudocode and assembly are written as compressed, atomically renamed
files by `analysisCache.ts`. The cache
write occurs after each individual `code` call. This is durable across app
restarts, but its identity is a cache path derived from the target and view;
the app source does not expose an engine revision and options digest equivalent
to Prism's durable job identity.

### Prism's current paths

Prism's external REA adapter starts a separate service, but it explicitly sets
`REA_GHIDRA_DECOMPILER_THREADS=1` in
[`src/analysis/rea.rs`](../src/analysis/rea.rs). This is a materially different
default from Reference's 4 or 8 bounded threads. The adapter also performs one
synchronous MCP call at a time. For the external provider, the first ranked
throughput change is therefore to make the thread count an explicit, bounded
configuration derived from measured CPU and memory limits, rather than to add
more callers around a one-thread service.

For the native provider, `session.rs`
owns one mutable Rust Ghidra `Program`. Each pseudocode request calls
`Program::decompile` for one address and
`function.rs` releases that
function state before returning. This release is important for resident memory,
but it means that a Rust thread pool cannot safely share this `Program`.
Parallel native decompilation requires isolated sessions and independent
checkpoint workspaces, or a new upstream API with an explicit concurrency
contract.

The interactive Prism batch envelope currently accepts up to 64 addresses but
returns at most four results and a roughly 1 MiB serialized page. Its loop is
still one `read_view_strict` call per address in
[`decompile_batch.rs`](../src/worker/decompile_batch.rs). The durable runner is
even narrower: [`decompilation_run.rs`](../src/worker/decompilation_run.rs)
advances one address, reads one pseudocode result, and records it before the
next address. Therefore the current batch command is a pagination and failure
boundary, not a native bulk-decompiler operation.

Prism has stronger explicit job durability than the Reference app path. A durable
job snapshots its ordered address set in SQLite, records a cursor and status,
compresses each artifact, hashes its uncompressed text, and commits the result
before advancing the cursor. [`results.rs`](../src/storage/decompilation/results.rs)
can reuse a successful result by project, provider, configuration, and address
after restart. This durable result layer is separate from the evictable
analysis reply cache. The current implementation pays one compression and one
SQLite transaction per function in
[`results/record.rs`](../src/storage/decompilation/results/record.rs), which
is a likely coordination cost for a 200,000-function export even when native
decompilation is warm.

Checkpoint behavior is another difference. Prism's native service may force a
checkpoint after a state-changing call, and the open path calls
`checkpoint(true)` after restoring or creating a session in
`native_ghidra/open.rs`. The
checkpoint guard skips that call when a restored snapshot is clean, while a
newly created session is dirty and is written. The service also uses a
ten-second checkpoint interval. Thus restore startup cost is snapshot decode
and program reconstruction, not an unconditional second snapshot write. Any
optimization should preserve the dirty-state and release invariants and measure
decode, reconstruction, and subsequent interval checkpoints separately.

### Ranked implementation implications

1. **External REA:** expose a bounded decompiler-thread setting and default it
   from the same memory and CPU inputs used by Reference. Measure RSS before
   increasing it. This is the clearest configuration-level throughput gap.
2. **Native export:** add a bounded multi-address native operation or streamed
   result protocol. Keep per-address artifact and digest state, but amortize
   RPC framing, transaction setup, and compression scheduling over a small
   chunk. A shared mutable `Program` must remain single-owner.
3. **Durable storage:** separate artifact preparation from the commit path and
   evaluate grouped SQLite transactions with crash recovery. Preserve the
   invariant that a published artifact is durable before its cursor advances.
4. **Checkpoint startup:** measure snapshot decode and program reconstruction,
   then measure interval checkpoints separately. Do not add a second restore
   write; the current clean-state guard already skips it.
5. **Reuse ordering:** resolve the durable result and analysis reply caches
   before entering native decompilation. Include engine revision and options in
   any new content-addressed artifact key. Do not treat the existing evictable
   cache key as an options-aware identity.

The source comparison supports these as implementable bottleneck reductions. It
does not establish that Reference can reconstruct an entire 200,000-function
binary in one hour, nor that a native Rust session can match Java Ghidra's
parallel package. Those claims require measurements on a declared corpus,
engine revision, options, concurrency, and cache state.

## What upstream sources establish

Ghidra's official Java parallel package uses a pool of `DecompInterface`
instances. `DecompilerCallback` obtains an interface for each function,
decompiles it, processes a `DecompileResults` value, and returns the interface
to the pool. It also exposes per-function timeout and disposal behavior.
`ChunkingParallelDecompiler` groups functions into chunks and waits for the
callback results. See the official [parallel package
summary](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/package-summary.html),
[DecompilerCallback API](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/DecompilerCallback.html),
and [ChunkingParallelDecompiler source](https://raw.githubusercontent.com/NationalSecurityAgency/ghidra/master/Ghidra/Features/Decompiler/src/main/java/ghidra/app/decompiler/parallel/ChunkingParallelDecompiler.java).

This is useful precedent for a pool of independent decompiler contexts. It is
not evidence that Prism's current Rust `Program` can be shared across threads.
The official [Ghidra developer guide](https://github.com/NationalSecurityAgency/ghidra/blob/master/DevGuide.md)
identifies Java as the primary extension language, with C++ and Sleigh used in
other layers. Prism's pinned Rust wrapper is a distinct interface and exposes
only the capabilities currently mapped in its native service.

If Prism adds a Rust pool for independent non-native stages, Rayon permits an
explicit thread count through [ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html).
An explicit bound is required: its default is based on available parallelism,
which can exceed the memory budget. Adding Rayon would be a dependency and
implementation decision, not a description of the current tree.

SQLite's official [WAL documentation](https://sqlite.org/wal.html) says that
readers and a writer can overlap, while only one writer exists. It also warns
that long readers can delay checkpoints and that the WAL grows until a
checkpoint completes. WAL can help a result browser read while one coordinator
writes, but it does not remove the native per-image lock or make a mutable
Ghidra session concurrent.

## Recommended design: durable batch export

Keep `decompile_batch` as the only public decompilation command. Add an
internal export-job mode behind that command, or an internal job lifecycle
whose execution always calls the same native batch operation. The interactive
UI should receive a job identifier and poll or subscribe to compact progress;
it should not carry 200,000 pseudocode strings through the agent conversation.

The job should contain:

1. An immutable manifest: image hash, engine identifier and revision, language,
   view, decompiler options, ordered address set, and manifest hash.
2. A durable cursor and per-address state: `pending`, `running`, `succeeded`,
   `timed_out`, or `blocked`, with an error and attempt count.
3. A canonical action key such as `(image_hash, address, view,
   engine_revision, options_hash)`, plus a separate digest of the serialized
   output artifact. The current analysis cache key is only
   `(project, address, provider:view)` and must not be treated as an
   options-aware content address.
4. An output record that is committed before the cursor advances. Replaying a
   job after a crash is then idempotent and skips committed artifacts.
5. A bounded output queue. The native worker must apply backpressure when the
   artifact writer or SQLite transaction is full.

The native worker should accept a batch of addresses and stream compact result
records to the coordinator. One RPC can then carry multiple functions while
the MCP surface still follows the existing batch rule. The coordinator should
write each record, release native function state, and report progress at a
bounded cadence. Do not build an in-memory vector containing the entire export.

Pseudocode and assembly artifacts should be stored separately from the
analysis checkpoint. The checkpoint can remain a program-state snapshot; the
artifact store answers whether a requested view already exists. This makes a
restart reuse both analysis and completed output without keeping a native
worker alive.

## Safe concurrency and straggler handling

The first implementation should use one native session with durable output.
It has the smallest memory footprint and makes the restart behavior correct.
Only after measuring that one session is the bottleneck should Prism add a
pool of native sessions. Every pooled worker must own an isolated program and
checkpoint workspace. The current exclusive image lock means simply spawning
several workers against one saved checkpoint is not valid.

If isolated workers are implemented, use a global resource budget rather than
one unrestricted pool per project. A worker's configured analysis memory limit
is a per-worker limit, so N workers can multiply the resident set. Start with a
small explicit count, record RSS and failure rates, and stop increasing the
pool when throughput no longer improves.

Schedule addresses through a bounded queue. Use short initial chunks, measure
recent p50 and p95 decompile duration, and split or retry only unresolved
addresses. A timeout must produce a durable `timed_out` result, not block the
whole export. Long functions should be isolated so one straggler does not hold
an otherwise completed chunk. Cancellation should stop new work and let the
current native call finish or return its explicit cancellation error.

Keep whole-program caller-index construction as a separate job. The current
`CallIndex::scan` walks every entry and stores edges in one sequential pass.
Including it implicitly in each function's export makes a supposedly local
request pay a global cost. Export a function's direct references first and
request caller data only when the selected workflow needs it.

## Throughput budget and acceptance metrics

The one-hour requirement is an end-to-end target, not a decompiler-only rate.
For 200,000 unique successful results:

* Required rate: `200000 / 3600 = 55.56` results per second.
* Current four-result page floor: `200000 / 4 = 50000` replies.
* At 100 ms mean native time, a serial worker needs 5 h 33 min. Six ideal
  workers are the arithmetic minimum before coordination overhead; eight would
  take 41 min 40 s in a no-overhead model.

These are arithmetic bounds, not measurements. Record cold and warm runs
separately and report:

* wall time from job creation to the last committed artifact;
* native CPU time, peak RSS per worker, total RSS, and checkpoint size;
* p50, p95, and p99 function duration, queue wait, and retry count;
* completed, cached, timed out, blocked, and failed functions;
* artifact reuse after process restart;
* reply count and bytes transferred through the MCP boundary; and
* whether all successful artifacts match the manifest and are readable.

The acceptance claim should state corpus, architecture, engine revision,
options, concurrency, and cache state. A one-hour result on one corpus is not
a general guarantee. Reconstruction adds model latency and validation work,
so its rate must be measured separately from pseudocode export.

## Migration sequence

1. Measure the current sequential batch path with existing checks and manual
   timing. Capture page sizes, native duration, RSS, and checkpoint restore
   time without adding a new test harness.
2. Add manifest, artifact, cursor, and status records. Implement a single
   worker durable export that resumes after restart and serves cache hits.
3. Add native multi-address RPC and bounded streaming so one batch request does
   not require one RPC per function. Preserve the 1 MiB reply limit for
   interactive MCP responses; job artifacts need not be returned in one reply.
4. Add bounded queues, cancellation, per-function timeouts, and straggler
   accounting. Keep publication in one coordinator transaction path.
5. Profile warm and cold runs. Optimize serialization, inventory page size,
   artifact compression, and SQLite access only where measurements identify a
   bottleneck. WAL is optional and must be evaluated with checkpoint behavior.
6. Prototype multiple isolated native sessions only if the single-worker
   export misses the measured target. Resolve checkpoint copy, lock, and memory
   ownership before enabling concurrency.
7. Make reconstruction consume immutable decompilation artifacts by manifest
   key. Language generation and validation remain separate stages and must not
   silently rerun native analysis.

## Risks and non-goals

* Rayon or another pool cannot make one mutable `Program` thread-safe.
* More native workers can reduce wall time while increasing peak memory and
  checkpoint contention.
* Larger pages reduce protocol overhead but increase reply size, latency, and
  failure blast radius; they do not replace durable jobs.
* Compression saves disk and transfer bytes but can increase CPU time. Measure
  artifact compression separately from native decompilation.
* A cached pseudocode artifact is reusable only when image hash, engine
  revision, view, and options match. Do not serve stale output after an engine
  change.
* Successful pseudocode generation is not proof of source equivalence or
  compilability. Preserve explicit blocked and uncertain states.

## Sources

Local source paths are linked above. External primary sources used here:

* [Ghidra parallel decompiler package](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/package-summary.html)
* [Ghidra DecompilerCallback](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/DecompilerCallback.html)
* [Ghidra ChunkingParallelDecompiler source](https://raw.githubusercontent.com/NationalSecurityAgency/ghidra/master/Ghidra/Features/Decompiler/src/main/java/ghidra/app/decompiler/parallel/ChunkingParallelDecompiler.java)
* [Ghidra developer guide](https://github.com/NationalSecurityAgency/ghidra/blob/master/DevGuide.md)
* [Rayon ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html)
* [SQLite write-ahead logging](https://sqlite.org/wal.html)
