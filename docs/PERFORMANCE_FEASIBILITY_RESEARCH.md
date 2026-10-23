# Performance feasibility for large decompilation and reconstruction

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Status: design research for Prism, based on the current worktree and primary
sources accessed 2026-10-19. This note is an independent feasibility audit. It
does not claim a measured tenfold speedup and it does not use proprietary
decompiler output or raw binary analysis.

## Decision summary

The request to export 200,000 successful function decompilations in one hour
is a measurable engineering target, not a consequence of changing the MCP
batch size. It requires at least 55.56 successful functions per second at the
end-to-end boundary. The current interface accepts 1-64 addresses but returns
at most four results per page, so a complete export would require at least
50,000 responses and 13.89 responses per second before retries, persistence,
agent scheduling, or startup time ([decompile_batch.rs](../src/worker/decompile_batch.rs#L8)).

The target is feasible only for a stated corpus and hardware after measuring
the native decompiler's service time, memory per worker, and long tail. It is
not honest to promise it for every binary. Tenfold reconstruction is even less
direct: source generation, dependency context, model rate limits, compilation,
repair, and behavioral checks are additional stages. A fast pseudocode export
does not imply fast or correct source reconstruction.

The current architecture has useful foundations: batch decompilation, bounded
leases (1-8 workers), content-addressed compressed objects, and a persistent
SQLite state database ([RECONSTRUCTION.md](RECONSTRUCTION.md), “Workflow and
ownership” and “Reuse and storage”). The native checkpoint is still a single
exclusive per image session and restoration reconstructs the Rust program from
compressed JSON (checkpoint.rs,
open.rs). More workers cannot safely be
added until mutable program ownership and checkpoint publication are separated.

## What can be claimed from primary sources

Ghidra's own Java parallel-decompiler API pools decompiler interfaces,
processes each function through a callback, supports a timeout, and requires
explicit disposal. The API is a useful ownership precedent, not evidence that
Prism's Rust wrapper is thread-safe or exposes the same pool:
[DecompilerCallback](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/DecompilerCallback.html),
[parallel package](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/package-summary.html).
Prism reports a Rust engine revision and explicitly lists “No Java Ghidra
project analyzers” in its limitations
(session.rs).

Rayon's official API permits an application-owned pool with an explicit thread
count; leaving it at zero selects a runtime-dependent default based on logical
CPUs. A bounded pool is therefore preferable for a low-resource desktop
application, and its count must be recorded with each benchmark:
[ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html).
This is a proposal for a future isolated extraction pool, not a recommendation
to parallelize the current mutable `Program` in place.

SQLite WAL allows readers and a writer to overlap, but there is still only one
writer; checkpoints can become slower and a long reader can prevent WAL reset.
Prism already enables WAL and sets `synchronous=FULL`, a 2 MiB cache target and
a three-second busy timeout for reconstruction
([database.rs](../src/reconstruction/database.rs#L37)).
Any export coordinator should keep transactions short and checkpoint during
idle gaps. See the primary SQLite explanation:
[SQLite WAL](https://www.sqlite.org/wal.html).

Amdahl's original paper establishes that a serial fraction limits parallel
speedup: [Validity of the Single Processor Approach](https://ieeexplore.ieee.org/document/4785615/).
For an end-to-end serial fraction `s`, the ideal unlimited-worker ceiling is
`1/s`. A finite worker count `P` has speedup `1 / (s + (1-s)/P)`. Therefore a
finite-worker tenfold result requires `s < 0.10`; at exactly `s = 0.10`, only
the unlimited-worker limit reaches 10x. Queueing, I/O, retries and rate limits
lower the achievable result further.

## Throughput model for 200,000 functions

The required rate is:

```
200,000 / 3,600 = 55.56 accepted functions/second
```

For a mean native decompile service time `m`, ideal worker demand is at least
`ceil(55.56 * m)`, before utilization loss. Examples:

| Mean service time | Serial wall time | Ideal workers for one hour |
| --- | ---: | ---: |
| 50 ms | 2 h 47 min | 3 |
| 100 ms | 5 h 33 min | 6 |
| 500 ms | 27 h 47 min | 28 |
| 1 s | 55 h 33 min | 56 |

These are arithmetic bounds, not Prism measurements. At 100 ms, eight perfect
workers still need 41 min 40 s, leaving less than 19 minutes for image loading,
queueing, result serialization, SQLite commits, retries, and failures. A
20-30% overhead reserve is a planning hypothesis to validate on the target
corpus, not a measured Prism property. Mean service time predicts average
capacity; p95 and p99 times determine whether a bounded job finishes on time.

Use the following stage-accounting model for each run. It is an accounting
decomposition, not an exact sum when stages overlap:

```
T = T_open + T_inventory + T_graph + T_native_serial
  + T_native_parallel/P_eff + T_encode + T_store
  + T_model + T_compile + T_feedback + T_publish
```

The one-hour claim is valid only when all terms are included and the accepted
count is exactly 200,000 unique function keys. Assembly-only outputs,
duplicates, timed-out functions, and error records must be reported separately.
Define `accepted` as a durable artifact whose address, analysis identity,
content hash, and status are committed before progress advances.

## Reconstruction feasibility and quality gates

Reconstruction should be measured as a separate pipeline. A practical unit is
an immutable module or strongly connected component (SCC), with a compact
language-neutral contract, dependency hashes, target-language adapter, and
bounded diagnostics. Schedule independent SCCs concurrently, but serialize
publication of conflicting files. The existing queue orders by module and
address and does not yet use dependency edges or SCCs
([queue.rs](../src/reconstruction/queue.rs#L88)).

The following outcomes must not be conflated:

| Outcome | What it proves | What it does not prove |
| --- | --- | --- |
| Pseudocode exported | Native view was produced and stored | Recoverable source or behavior |
| Target parses | Syntax is accepted | Correct types, ABI, or runtime behavior |
| Target compiles | Toolchain accepted this build | Equivalence for untested inputs |
| Differential check passes | Observed oracle agrees for tested cases | Universal equivalence |
| Module contract accepted | Interfaces and ownership are recorded | Original source recovery |

The acceptance report should include per-stage elapsed time, p50/p95/p99,
accepted/blocked/timeout counts, bytes and tokens, cache-hit ratios, model and
tool fingerprints, peak RSS, CPU time, and compile and behavioral-check rates.
For any-language claims, list the adapter and toolchain explicitly. “Any
language” means an adapter was available and its build/check policy passed; it
does not mean a universal compiler or runtime exists.

## Low-resource design constraints

1. Keep one coordinator responsible for the checkpoint lock and durable
   publication. A future extraction boundary may share immutable image bytes,
   but each worker would need private mutable analysis state. The current
   implementation does not provide that boundary, so it cannot claim that
   workers avoid duplicating `Program` state.
2. Make the export job durable and cursor-driven. The MCP call should start or
   resume a job and return bounded progress; an agent should not hold 50,000
   interactive pages in context. Keep the existing four-result page for
   focused inspection.
3. Apply back-pressure at every boundary: native result queue, compression,
   SQLite transaction, model requests, and source publication. Bound queued
   pseudocode bytes as well as item counts.
4. Use dynamic chunks with a per-function deadline. Retry only unresolved
   functions, record the original failure, and prevent a single expensive
   function from stranding a worker partition.
5. Keep compressed immutable objects, but do not assume compression reduces
   peak RAM: decode one object at a time and release its input buffer after
   publication. Measure allocator high-water RSS, not only database size.
6. Use WAL deliberately. Short read transactions allow checkpoint progress;
   long inventory scans should page and close their transaction before native
   work. A larger cache is not automatically a speedup under a RAM budget.
7. Separate cold and warm restore benchmarks. The current measured warm saved
   restore after the property-range fix was 18.536 seconds with 1,062 MiB peak
   RSS; the earlier baseline did not finish, so no speedup ratio is established.
   Treat this as a measurement to improve, not as an SLO already met.

## Go/no-go benchmark plan

Run only against an authorized, independently authored fixture or a user-owned
project, using existing Prism commands and checks. Do not create a new test
harness. Record a manifest containing binary SHA-256, architecture, function
count, Prism commit/worktree hash, decompiler revision, OS, CPU/RAM, storage,
configuration, model/provider, language adapter, and cache state.

Run the currently available workflows separately:

* cold open and first analysis;
* warm open with a valid saved analysis;
* the existing paginated batch pseudocode export;
* reconstruction generation and build for one supported language;
* restart and resume after a clean shutdown and after a cancelled job.

The durable export-job abstraction and bounded native extraction pool described
above do not exist yet. Once implemented, they become additional acceptance
workflows, rather than baseline measurements of the present code.

For each run, capture wall time, CPU time, peak RSS, bytes read/written,
SQLite WAL size, queue wait, native service-time histogram, result size,
retries, and final status counts. Compare a sequential configuration with each
bounded worker configuration on the same input. Stop increasing workers when
throughput no longer improves, peak RSS exceeds the declared budget, or error
rate rises. A one-hour go decision requires 200,000 unique accepted outputs,
zero silent omissions, durable restart/resume, and a published resource report.
Otherwise report the largest completed count and the bottleneck stage.

For reconstruction, a go decision additionally requires a declared module
coverage denominator, per-language compile rate, reproducible source
fingerprints, and a behavioral oracle for any equivalence claim. A generated
source tree that merely compiles is a useful artifact but not evidence of full
reconstruction.

## Recommended priority

First, add a durable export-job abstraction behind the existing batch MCP
command, with cursor, bounded queues, per-function status and resumable output.
Second, profile and isolate native decompiler state so a measured, bounded pool
can be introduced without violating the exclusive checkpoint lock. Third,
replace module/address ordering with dependency fingerprints and SCC scheduling
for reconstruction. Fourth, add stable-prefix request batching and compiler
feedback only where provider limits and a declared quality gate permit it.

Do not advertise “instant restore,” “200k in one hour,” “10x,” or “any language”
until the corresponding cold/warm, corpus, hardware, resource, coverage, and
quality conditions are published. These are useful targets; they are not
properties established by the current code or the cited sources.
