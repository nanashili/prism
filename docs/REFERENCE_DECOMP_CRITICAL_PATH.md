# Reference and Prism: decompilation critical path

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Source audit, 2026-10-19. Scope: the Reference app in `../app` and Prism's current
working tree. The enclosing repository HEAD was
`85a9be336ecf442cd8a8aa27a8a98060cee0646a`; Prism and other files were dirty or
untracked, so that commit alone does not reproduce this source snapshot. Critical
file hashes appear at the end. This report is source evidence, not a benchmark or
a Unity compatibility assessment. No proprietary analysis outputs or checkpoints
were inspected. No tests, fixtures, harnesses, heavy builds, or profiling runs were
created or executed for this audit.

## Conclusions and evidence strength

1. **Confirmed:** default Prism Ghidra decompilation uses one native Rust worker
   and one mutable `Program`. Its durable runner advances one address, waits for
   native analysis and any checkpoint, writes persistence, then advances again.
   Raising external REA thread settings cannot accelerate this path.
2. **Confirmed:** with nonempty string sections, native pseudocode performs a
   raw-flow pass to find string constants, clears its analysis, then runs the full
   decompiler action pipeline. Its precise fraction of elapsed time is unmeasured.
3. **Confirmed:** decompilation dirties analysis state and can synchronously save
   a whole-program checkpoint before returning the function. The normal gate is
   ten seconds since the last completed save, not one forced save per function.
   Whole-program serialization can therefore make small-function throughput
   depend on inventory size. Its current cost is unmeasured.
4. **Confirmed:** a cold successful durable function normally gets compressed and
   committed twice: once to the interactive view cache and once to durable job
   results. The durable connection explicitly uses WAL and `synchronous=FULL`.
   These operations run serially with native computation.
5. **Confirmed:** Reference requests 256 addresses per provider batch and reads
   retained pages of 64. It configures a default thread capacity of up to four or
   eight, subject to CPU count. **Unverified:** the external provider actually
   uses concurrent decompilers for those calls. App source alone cannot establish
   its execution strategy, internal caching, durable writes, or measured speed.
6. **Confirmed:** Reference's bulk writer consumes provider batch pages directly and
   syncs its output file once at the end. It does not call its single-function
   `code()` cache writer. Attributing the separate level-19 cache compressor to
   every bulk-exported function is incorrect.

No source observation proves that a particular item is the largest wall-time
cost. Optimize verified duplicated work first, then use measured stage costs to
choose between checkpoint changes and bounded native concurrency.

## Actual execution paths

### Reference bulk export

`exportAllPseudocode` obtains the inventory addresses and streams
`rea.batchDecompile` into a partial output file. The generator sends
`batch_decompile({addresses, detail:"summary"})` with 256 addresses, then, when the
summary references retained evidence, retrieves `decompilations` pages of 64 in
selector order. Calls and writes are awaited. A serialized client connection does
not disprove provider-internal parallelism, but it does not demonstrate it either.

Sources: batch generator and provider request, rea.ts:414-498,
stream and final sync, export.ts:160-220,
thread capacity, analysisResources.ts:39-50,
environment handoff, rea.ts:122-125.

For 200,000 functions, this shape requires 782 summary calls and 3,125 page calls
if every summary uses retained evidence and every page contains 64 results:
3,907 provider calls. Inline complete results reduce that count. These are
arithmetic capacity figures, not an observed run. The provider may perform
additional work or persistence that is outside the inspected app source.

The writer calls `handle.sync()` after the complete stream. This differs from
Prism's per-function durable result/cursor transaction; the guarantees must be
compared before using final-file-only persistence as a speed target. The bulk
generator does not invoke `code()`, whose separate cache writes use zstd level 19.
Sources: individual code cache, rea.ts:529-543,
cache encoding and atomic write, analysisCache.ts:72-103.

The app resolves REA from a packaged resources script or source-relative sibling
checkout. No external REA implementation was located in the standard sibling
locations checked during this audit. This is a local evidence gap, not a claim
that provider internals cannot be inspected or are necessarily unavailable.
Source: toolchain.ts:114-117.

### Prism native and durable export

Default provider `ghidra` selects `NativeGhidra`; `rizin` selects its native adapter;
other provider IDs select external REA. The default native backend is therefore
a different implementation from Reference's external Ghidra provider.
Sources: [dispatch, engine.rs:24-38](../src/analysis/engine.rs#L24),
[default and budgets, settings.rs:62-79](../src/settings.rs#L62).

The native service reads one request, dispatches it, then writes its reply before
reading another. `Session` owns one mutable program. The durable runner obtains
one manifest address, checks saved results, reads pseudocode, persists the result,
and reloads progress before the next step. This is actual serialization, not just
a missing thread-count option.
Sources: native loop, service.rs:19-33,
program ownership, session.rs:9-22,
[durable step, decompilation_run.rs:30-58](../src/worker/decompilation_run.rs#L30),
[manifest and record progression, job.rs:48-108](../src/decompilation/job.rs#L48).

Prism's interactive batch is also sequential. It accepts up to 64 selector
addresses but executes at most four per request, with an approximately 1 MiB reply
budget. For 200,000 small functions this is at least 50,000 interactive requests.
That count does **not** describe the durable runner, which runs internally after
starting the job and does not need one client batch request per four functions.
Source: [decompile_batch.rs:12-64](../src/worker/decompile_batch.rs#L12).

## Native work and state lifetime

### Repeated flow work before the full action pipeline

`Program::decompile_unguarded` calls `add_referenced_strings`, then invokes
`action.reset` and `action.perform`, prints, and clears function analysis.
`add_referenced_strings` returns early only when there are no string sections.
Otherwise `flow_facts` clears the function, follows flow, scans every alive
operation input into a `BTreeSet` of constants, also gathers call targets and body
extent, then clears analysis again. The full action's processing follows flow
again. The string path consumes constants but not the collected calls/body.

Sources: [program.rs:427-437 and 792-813](../vendor/ghidra-decompiler/src/program.rs#L427),
[flow_facts, discovery.rs:19-59](../vendor/ghidra-decompiler/src/program/discovery.rs#L19),
[flow generation, funcdata_op.rs:631-654](../vendor/ghidra-decompiler/src/funcdata/funcdata_op.rs#L631),
[processing start, funcdata/mod.rs:425-448](../vendor/ghidra-decompiler/src/funcdata/mod.rs#L425).

This proves duplicated flow construction, not a doubling of total decompilation
time: translator-level caches can still help, and later actions can dominate.
Potential improvements are a constants-only raw-flow pass, compact address-bound
facts reused from existing discovery, or a reviewed way to preserve usable raw
flow into full analysis. The last option requires checking symbol/type binding
and action-start invariants. Simply deleting string enrichment changes output:
it currently adds typed global string symbols before full decompilation.
Source: [string symbol mutation, program.rs:440-465](../vendor/ghidra-decompiler/src/program.rs#L440).

### Low-RAM cleanup is deliberate

After the operation, `with_released_function` independently attempts cleanup and
preserves both operation and cleanup failures. The symbol's `Funcdata` is removed
and destroyed. Within decompilation, `clear_analysis` already clears temporary
p-code, blocks, varnodes, high variables, call specs, and analysis state. Retaining
all analyzed functions would change this memory behavior substantially.

Sources: function.rs:48-81,
[Funcdata clear, funcdata/mod.rs:364-396](../vendor/ghidra-decompiler/src/funcdata/mod.rs#L364),
[Funcdata destroy, funcdata/mod.rs:208-218](../vendor/ghidra-decompiler/src/funcdata/mod.rs#L208).

The durable checkpoint retains architecture symbols/types/context/comments/string
knowledge, saved function/data records, discovery state, call index if present,
and seeded addresses. It excludes executable bytes and compiled translator
caches; released per-function scratch and finished pseudocode are not restored
as native hot decompilation state. Finished text survives separately in SQLite.
Sources: [snapshot model and construction, snapshot.rs:1-95](../vendor/ghidra-decompiler/src/program/snapshot.rs#L1),
checkpoint model, checkpoint.rs:26-51.

An ordinary pseudocode request does not build the whole-program callers index.
That index is an independent path which scans functions with flow analysis. Avoid
adding callers/dossiers to bulk export under the assumption that they are free.
Source: references.rs:35-70.

## Checkpoint work on the function response path

`service::execute` marks pseudocode dirty before execution. Afterwards, it calls
`checkpoint(force)` before emitting the response. A normal pseudocode call is
not forced; inventory/call-index creation, analyze, and discovery can force a
save. A dirty normal checkpoint runs once at least ten seconds have passed since
the previous successful save. The timer resets **after** completing the save.
If a single function takes more than ten seconds, each function can trigger one.
Sources: service.rs:61-86,
gate and save, checkpoint.rs:125-155.

Saving scans known function symbols, encodes the full architecture into an owned
XML string, clones function/data metadata into owned collections, streams the
resulting JSON through zstd level 3, syncs the temporary file, renames it, and
syncs the parent directory. This is whole-program work, not just the latest
function's scratch state. A save failure can turn an otherwise successful
decompilation response into an error.
Sources: [snapshot.rs:47-95](../vendor/ghidra-decompiler/src/program/snapshot.rs#L47),
compression, checkpoint.rs:141-153,
[atomic durability, atomic.rs:5-39](../src/storage/atomic.rs#L5),
error composition, service.rs:101-109.

The 512 MiB uncompressed JSON writer limit is applied after constructing the XML
and metadata snapshot. It does not cap peak allocation while capturing that
snapshot. A 1 GiB worker budget can thus become tight before the limit rejects
the write. Restore also overlaps saved structures with a newly initialized live
program, though it transfers the XML buffer into the compact decoder rather than
intentionally cloning it. Source: [snapshot.rs:110-135](../vendor/ghidra-decompiler/src/program/snapshot.rs#L110).

An illustrative model: if ten seconds of useful work are followed by a save
costing `C` seconds, the checkpoint share is approximately `C/(10+C)`. This is not
a measured cost. Inventory-dependent snapshot size and actual save frequency
must be measured on the target program before deciding how much throughput can
be recovered. Disabling dirty tracking is not correct while decompilation can
add durable string/type/symbol knowledge. Background compression alone also does
not remove capture allocation or the need to capture a consistent mutable state.

On Unix, native worker startup creates an immutable temporary copy of the
executable and memory maps that copy; opening the checkpoint hashes the image.
Other platforms retain owned executable bytes. Repeated worker
replacement therefore includes image copying/hashing, program initialization, and
checkpoint restore even when discovery is saved. Preserve the repaired reusable
worker lifecycle. A persistent pool must account for each worker's image and
architecture footprint, not only each function's temporary memory.
Sources: [image_bytes.rs:32-51](../vendor/ghidra-decompiler/src/image_bytes.rs#L32),
checkpoint open, checkpoint.rs:55-80,
native open, open.rs:27-51.

## Persistence and large-job retrieval

### Two compressions and two transactions for a cold successful function

Strict pseudocode first looks for a compatible durable artifact, then the view
cache. A cold native call checks its code cache again. On success it writes
zstd-level-3 text to the separate view-cache connection; afterwards the durable
runner again compresses the text with level 3, computes its digest, and commits
the artifact and job progress to the main DB.
Sources: [cache path, analysis_cache.rs:34-120](../src/worker/analysis_cache.rs#L34),
[cache write, cache.rs:66-90](../src/storage/cache.rs#L66),
[durable write, record.rs:10-61](../src/storage/decompilation/results/record.rs#L10).

For a newly successful result, the durable transaction performs eight SQL
statements before commit: load previous result; read cursor; verify manifest
address; read quota; update quota; upsert result; update counters; advance cursor.
It updates the job row three times. Surrounding the transaction are more manifest,
success/cache checks and a full job-row reload. Repeated statement setup and
small configuration serialization add overhead, but no timing establishes their
importance relative to full decompilation or storage sync.
Sources: [record.rs:19-43 and helper SQL](../src/storage/decompilation/results/record.rs#L19),
[progress reload, job.rs:103-108](../src/decompilation/job.rs#L103).

The main DB explicitly uses `journal_mode=WAL; synchronous=FULL`. The view cache
does not explicitly choose journal mode or synchronous policy in its open path;
its effective state should be queried rather than assumed. `auto_vacuum=FULL`
and cache eviction can add work, but that amount is unmeasured. Current cache
accounting is trigger-maintained: it does **not** sum the entire cache on every
insert. Warm cache hits can update the access timestamp after sixty seconds.
Sources: [main DB, database.rs:24-29](../src/storage/database.rs#L24),
[view-cache schema/accounting, cache.rs:6-35](../src/storage/cache.rs#L6),
[hit and eviction, cache.rs:41-63 and 109-126](../src/storage/cache.rs#L41).

Skipping the redundant interactive-cache write during durable generation is a
concrete candidate because strict reads already prefer durable text. Ordinary
interactive reads currently pass `allow_upstream=true`, so they do not consult
that durable-text branch. Extend their compatible-durable lookup when bypassing
the cache write, otherwise they can recompute code that the export saved.
Preserve configuration-sensitive identity. Current cache identity includes the configuration digest;
the older claim that native cache keys ignore options is stale.
Source: [analysis_cache_identity.rs:12-20](../src/worker/analysis_cache_identity.rs#L12).

### Retry and result pages can become a second large workload

Retry rewinds the cursor to the earliest failed manifest position, not merely to
the next failed item. Subsequent successful positions are traversed again.
`skip_saved` decodes/hashes the artifact, then `record_decompilation_result` loads
and decodes/hashes it a second time before committing the cursor. A failure near
the start of 200,000 mostly successful functions can therefore cause a long
persistence-only retry walk even though native decompilation is skipped.
Sources: [retry rewind, jobs.rs:156-175](../src/storage/decompilation/jobs.rs#L156),
[first verification, job.rs:58-70](../src/decompilation/job.rs#L58),
[second verification, record.rs:24-27 and 115-126](../src/storage/decompilation/results/record.rs#L24).

Result paging uses `LIMIT ... OFFSET ... ORDER BY i.position` from a result table
joined to manifest items. Increasing offsets require increasing skipped-result
work; a 100-item traversal of 200,000 results has about 199.9 million preceding
positions across its pages. The exact access cost and whether SQLite repeatedly
builds a temporary sort require `EXPLAIN QUERY PLAN` against the actual schema;
the joined ordering is a concern, not a measured plan. When data is included,
each returned artifact is decompressed and digest checked; the lookahead or
over-budget row may be decoded again on the next page. Per-row JSON sizing also
serializes data before the final response serializer does so again.
Source: [pages.rs:15-88](../src/storage/decompilation/pages.rs#L15).

Driving ordered reads from `(job,position)` and using a stable keyset cursor is a
bounded-memory improvement candidate. Sparse/unattempted manifest positions and
compatibility with the published offset API require explicit handling. The
existing read-only fast lane fixes worker blocking, but does not change this SQL
or the artifact decode cost.

## Prioritized feasible implementation

| Priority | Change | Why it is justified | Required lifecycle and low-RAM constraint |
| --- | --- | --- | --- |
| 1 | Bypass duplicate view-cache population during durable production; persist one compressed representation | Two encodes and two transactions are directly visible; durable text is already preferred on strict reads | Keep digest validation, configuration identity, atomic artifact/cursor persistence, and normal interactive caching |
| 2 | Reduce string-enrichment flow work while preserving output | The discarded raw-flow pass is directly visible on every eligible cold function | Start with constants-only facts or compact cached facts; do not retain full p-code for all functions or remove string semantics |
| 3 | Measure capture, XML encode, zstd, and sync separately; revise checkpoint policy from those results | Whole-program saves block replies and have inventory-dependent allocation | Keep a consistent state boundary and recoverable committed snapshot; explicit larger recovery window or incremental metadata log needs an approved contract |
| 4 | Group bounded durable results in one transaction and combine job-row updates | Serial per-result commits and repeated row updates are directly visible | Bound pending bytes and elapsed time, atomically commit artifacts plus cursor, flush on pause/end; report only committed progress and permit recomputation after crash |
| 5 | Add bounded native batch RPC; then consider a persistent independent-program worker pool | One RPC per function and one program are directly visible | Batching alone is sequential. A pool needs independent mutable programs, deterministic result ordering, one durable writer, per-worker cleanup/cancellation, and aggregate peak-RAM admission |
| 6 | Keyset result export and pending-only retry traversal | Increasing OFFSET work and repeated successful-result verification are visible | Preserve manifest order, artifact verification, resumability, sparse results, and honest progress semantics |
| 7 | Remove smaller avoidable copies/preparation after profiling | Per-row JSON size encoding, repeated config serialization, SQL preparation, printer UTF-8 copying exist | Avoid broad rewrites or caching mutable analysis objects without ownership bounds |

For priority 4, illustrative starting limits such as eight results plus a byte
and time cap are design choices, not measured optimums. Eight maximum-size raw
512 KiB artifacts alone consume 4 MiB; compressed buffers, strings, JSON, and
SQLite state add more. A time cap prevents tiny slow jobs waiting indefinitely
for a count threshold. Batch failure must leave the last committed cursor and
artifacts mutually consistent. It must not acknowledge uncommitted success.

For priority 5, changing an environment thread count or parallelizing a loop over
the existing mutable `Program` is insufficient. Prefer a small persistent pool
only when the aggregate resident/peak budget admits it. One worker may be the
correct choice on a tight machine. Each worker also has a checkpoint lock/path:
concurrent workers cannot all own the current exclusive same-image checkpoint.
A pool requires a deliberate single checkpoint owner or a read-only base plus
reviewed merge/worker-state policy, including new string/type knowledge.
Sources: exclusive checkpoint lock, checkpoint.rs:63-73,
native process budget, client.rs:24-48.

Finishing 200,000 functions in one hour requires about 55.6 successes per second.
A purely serial path has about 18 ms total per function for decompilation,
checkpoint amortization, persistence, and coordination. Eight perfectly utilized
workers would permit about 144 ms of decompiler time per function only if serial
overhead, failures, memory pressure, startup, and skew were negligible. These
figures express the target, not achievable or measured throughput.

## Verification gaps and existing repaired behavior

The coordinator owns runtime measurements. This audit does not establish native
stage timings, full-inventory memory peaks, fsync latency, REA provider pool
behavior, SQLite's effective cache pragmas/query plan, or any speedup claim.
Required comparisons should separate cold analysis, warm inventory/restoration,
warm text artifacts, and result retrieval, while checking equal inventory and
artifact digests with the existing client and authorized fixtures only.

The starting source already contains repairs for synchronized worker reuse after
domain errors, restored global symbol-counter reservation, and read-only
status/results access outside the busy analysis worker. Do not count these as
new remaining opportunities or undo them. RPC polling waits are not an automatic
100 ms charge per function: a ready response can wake the wait earlier.

No new dependencies or production source changes were made by this audit. No
format/build/test pass is claimed for this documentation-only work.

## Critical source snapshot hashes

SHA-256 at inspection time, relative to `prism/`:

```text
4849edcb0b23b484938ddd4c9265e02b486dd4b5db911d53913b178e2c09de91  src/analysis/native_ghidra/service.rs
bd69de55efbfe7f973a2743d541a9f38b1ea17a383bad7e660571b35d59d989f  src/analysis/native_ghidra/checkpoint.rs
892d219e580c19c7e101651fd39501150ccd629499c6878fd58c9964c4866048  vendor/ghidra-decompiler/src/program.rs
03b9d41b0807b326fe0db4e8c77b05a15d2e9e806c49c0edb78d5ca40ccbedf3  vendor/ghidra-decompiler/src/program/discovery.rs
368c4f74e9f9ea324790ca52f5bbce06e5f475ae891d02e2d9993236c64189af  vendor/ghidra-decompiler/src/program/snapshot.rs
2362c48606de43ce1ad01b1f016078c983692ba8a567c55633b6eee7e808ff9f  src/storage/decompilation/results/record.rs
b1af97b76e87a537f66c17b973e23971ee438526e5e426ed5edc33c516256a44  src/worker/analysis_cache.rs
f13b7945793e31ac89bc1106fbfac3d4f641d6e3ad5d24e723a74b4360f94b3d  ../app/src/engine/rea.ts
ff31e27777eefa31ada6ff786d60f51fd4e24cb610943ba31112ec9de14b8591  ../app/src/workbench/export.ts
```
