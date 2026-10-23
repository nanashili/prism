# Resource efficiency research for Prism

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Status: design research for Prism, based on the current worktree and primary
sources read on 2026-10-19. This note contains no proprietary binary analysis,
production edits, tests, or new benchmarks. Local observations are labelled as
measured or source inspection; proposed changes are not measured savings.

## Current resource picture

The recent warm restore measurement is the most useful baseline: restoring the
saved Unity analysis completed in **18.536 seconds** and reached **1,062 MiB peak
RSS**. This is one completed run, not a release-wide benchmark. The user also
observed a second Prism process around 500 MiB; that observation needs a process
ownership trace before assigning it to a particular worker or cache.

The relevant allocation boundaries are visible in the source:

* `image::open` reads the
  selected executable slice into an owned `Vec<u8>` before parsing it. The
  `Vec` is passed into `Session::open`
  and then into `Program::from_snapshot`.
* The pinned vendor restore path creates an `ObjectArchitecture` with those
  bytes, then its loader construction clones the image buffer into
  `ObjectLoadImage` ([`with_bytes`](../vendor/ghidra-decompiler/src/object_arch.rs#L801-L805),
  [`build_loader`](../vendor/ghidra-decompiler/src/object_arch.rs#L842-L867)).
  During restore, the decoded `Saved` value owns its fields while the
  architecture XML string is parsed into `DocumentStorage`; the XML string is
  explicitly dropped after parsing ([`from_snapshot`](../vendor/ghidra-decompiler/src/program/snapshot.rs#L87-L109)).
  The checkpoint decoder is a streaming Zstandard reader feeding a 64 KiB
  `BufReader` and `serde_json::from_reader`, so this source does not establish a
  whole compressed buffer or whole JSON text buffer. The measurable overlap is
  the owned decoded fields, XML/DOM, and restored program maps. The builder is
  already cleared immediately after native initialization, before XML parsing.
  Its image clone is therefore a separate transient initialization cost, not a
  proven duplicate allocation throughout the XML/index restoration peak.
* [`ProgramSnapshot`](../vendor/ghidra-decompiler/src/program/snapshot.rs#L15-L23)
  stores architecture XML plus `BTreeMap` records. It is a durable state
  representation, but it is not an mmap-able index and it has no lazy record
  loading. [`restore_records`](../vendor/ghidra-decompiler/src/program/snapshot.rs#L113-L143)
  inserts the complete saved function and data maps before the session is
  available.
* Checkpoint restore is bounded to 512 MiB of decompressed JSON and uses a
  64 KiB reader (`checkpoint.rs`).
  The bound protects against oversized state but does not cap the allocations
  needed after parsing. Checkpoint writes call `program.snapshot()`, serialize
  JSON, then compress it (`checkpoint.rs`).
* The worker intentionally retains its analysis engine until 60 seconds idle and
  drops it on timeout or failed work ([`analysis/engine.rs`](../src/analysis/engine.rs#L10-L46),
  [`worker/thread.rs`](../src/worker/thread.rs#L86-L117)). This saves startup work
  for nearby requests but explains why an apparently idle Prism process can keep
  hundreds of MiB. `State::open` drops the previous engine before opening another
  project ([`worker/open.rs`](../src/worker/open.rs#L9-L39)).
* Reconstruction objects are read by selecting a compressed BLOB into a
  `Vec<u8>`, then decoding into a second `Vec<u8>` ([`objects.rs`](../src/reconstruction/objects.rs#L31-L51)).
  Objects are capped at 1 MiB, so this is bounded per request but keeps both
  representations live during decoding; the ratio depends on compression.
  Reconstruction keeps a 2 MiB SQLite page
  cache target and WAL with `synchronous=FULL`
  ([`reconstruction/database.rs`](../src/reconstruction/database.rs#L37-L45)).
* The external REA worker is currently configured with one Ghidra decompiler
  thread and an explicit Java heap limit ([`rea.rs`](../src/analysis/rea.rs#L29-L46)).
  This is a deliberate resource bound, so parallelism should be introduced only
  with measured per-worker RSS and queue back-pressure rather than by raising
  this value globally.
* The normal release profile already uses `opt-level="s"`, thin LTO, one
  codegen unit, symbol stripping and `panic="abort"`; the decompiler dependency
  is raised to `opt-level=3` ([`Cargo.toml`](../Cargo.toml#L66-L85)). A current
  worktree artifact was measured with `du -sk build/Prism.app` as 24,184 KiB
  (allocated size, rounded by `du`), while `stat` reported logical file sizes of
  19,414,576 bytes for `build/Prism.app/Contents/MacOS/prism` and 534,224 bytes
  for `prism-mcp`. The corresponding SHA-256 hashes were
  `8a6497acf2ce5b460212a7d52e684ec5a6797ee4a78b324ae45c294562dbb6fe` and
  `7a9734830696d9e1f41db104fc64ef014f11e0535ad49774e79bdee20e205926`.
  These identify a particular pre-rebuild worktree bundle. Signing status was
  not established by the size commands, and allocated size must not be compared
  directly to a logical-byte bundle total.

## What primary sources establish

Ghidra's Java API has a packed database abstraction. `PackedDatabase` can open a
packed database and says that unpacking is cached when possible
([Ghidra `PackedDatabase`](https://ghidra.re/ghidra_docs/api/ghidra/framework/store/db/PackedDatabase.html)).
`GhidraProject.saveAsPackedFile` publishes a program as a packed file
([Ghidra `GhidraProject`](https://ghidra.re/ghidra_docs/api/ghidra/base/project/GhidraProject.html)).
This supports the original Reference approach of reusing a native Ghidra database,
but does not prove that a packed Java database can be opened by Prism's pinned
Rust wrapper. It is a compatibility investigation, not permission to copy Java
implementation code into the Rust backend.

SQLite documents that a negative `PRAGMA cache_size` is a suggested KiB limit,
not a hard process-wide memory cap, and that `PRAGMA shrink_memory` asks the
connection to release as much page-cache memory as possible
([SQLite pragmas](https://www.sqlite.org/pragma.html)). The C API equivalent,
`sqlite3_db_release_memory`, attempts to free heap memory for one connection
([SQLite C interface](https://www.sqlite.org/c3ref/db_release_memory.html)).
Therefore Prism should measure cache RSS and explicitly shrink after large
inventory/export operations; changing the pragma alone cannot guarantee a RAM
ceiling.

Rust's `Arc` provides shared ownership of immutable data across threads without
copying the inner value when the handle is cloned
([`std::sync::Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html)). It is a
valid design tool for one read-only executable image shared by bounded workers,
but it does not make the underlying decompiler database thread-safe. Mutable
program state must remain owned by one worker or use a reviewed synchronization
boundary.

Cargo documents that `opt-level="s"` and `"z"` optimize for size, while `"z"`
also disables loop vectorization; thin LTO generally costs less link time than
fat LTO while retaining substantial optimization
([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)).
The project therefore needs measured profile variants, not a blanket switch to
`opt-level="z"`: the decompiler hot path is deliberately at level 3.

The `zstd` Rust stream API exposes pull-based readers and writers, so callers
can process compressed data incrementally instead of requiring a whole input or
output buffer ([`zstd::stream`](https://docs.rs/zstd/latest/zstd/stream/)).
The pinned API must still be checked for reusable decoder contexts before adding
a dependency or changing the on-disk format.

## Recommended design, ordered by leverage

### P0: remove restore-time duplication before adding workers

Make restore a staged operation with explicit ownership phases:

1. Read and authenticate the checkpoint header and image identity.
2. Decode one bounded representation into the destination structure.
3. Release decoded snapshot fields and XML/DOM state at the earliest safe
   ownership boundary; the current decoder already streams compressed input and
   JSON rather than materializing either whole representation.
4. Remove or avoid the image clone in `ObjectArchitecture::build_loader` when
   the wrapper permits it. `Program::from_snapshot` moves the image buffer into
   `ObjectArchitecture`, and the confirmed duplicate is the later `bytes.clone()`
   at loader setup.

The owner should be `analysis/native_ghidra/checkpoint.rs` for framing and
`vendor/ghidra-decompiler/src/program/snapshot.rs` for the destination API.
Avoid an unsafe mmap implementation until the vendor API has a lifetime-safe
borrowed image representation; mapping itself does not make the API safe. A
safe intermediate is a temporary file-backed mapping or shared `Arc<[u8]>` only
if the architecture object accepts a shared immutable slice. `Arc` is
process-local and does not share bytes with another worker process. Measure peak
RSS, allocations and restore wall time separately
for compressed input, XML parse, record restore and post-restore indexing.

For a larger redesign, persist a compact binary snapshot or a Ghidra-compatible
packed database and keep a small metadata index in SQLite. A packed database is
the source-supported precedent, while a new Rust binary format is Prism-owned
and must have a version, checksum, image hash, engine hash and migration path.
Do not claim “instant restore” until a cold process and warm process are both
measured. Lazy loading can make the first view fast while moving work to the
first uncached function; both timings must be reported.

### P0: make engine lifetime a resource policy

Replace the fixed 60-second retention with an explicit policy derived from the
configured memory budget. Keep the current engine for interactive bursts, and
drop it at an idle, checkpointed boundary, on project switch, or when the user
requests “release”. A memory watermark may trigger a release only after the
current dirty operation reaches a safe checkpointed boundary; do not tear down
an engine during dirty work. The owner is `src/worker/thread.rs`, while the
engine implementation is `src/analysis/engine.rs`; `Engine` should expose a
narrow `memory_hint` or `release_idle` operation rather than backend internals.

After releasing the SQLite cache, issue `PRAGMA shrink_memory` on both the main
and view connections and measure the result. This is best-effort according to
SQLite's contract. Ensure every release path preserves the original operation
error and reports an independent cleanup error, instead of silently ignoring
it. Do not run several native engines for the same image until checkpoint
ownership and write ordering are redesigned; the current per-image checkpoint
lock is exclusive.

### P1: stream reconstruction objects and bound queues

Give `objects.rs` two APIs: a bounded reader for file export and the existing
small `Vec` API for UI-sized values. The reader should decompress into the caller
provided sink with an exact declared-size limit and verify the digest after the
stream completes. Keep the one-MiB object bound, but avoid materializing both
compressed and decompressed copies when writing to disk or sending a response.
Use reusable zstd decoder/encoder contexts only after profiling context reuse
against the current `encode_all` path.

Put explicit byte and item budgets on reconstruction claims, evidence reads,
model-request queues and progress previews. The existing eight-item worker
channel is a useful scheduling back-pressure point, but it is not a memory
guarantee unless every queued item has a measured or enforced byte budget. The
reconstruction worker limit must be enforced as a real semaphore, with
cancellation and lease expiry releasing all buffers. Report both queued item
count and reserved bytes; an arbitrary worker count is not a resource bound.

### P1: share immutable analysis, isolate mutable decompilers

Separate immutable image bytes, immutable symbol/index snapshots, and mutable
per-function decompiler state. Share only immutable layers within one process
with `Arc`, or use a lifetime-safe file-backed mapping; give each bounded worker
a private mutable decompiler context. Keep snapshot publication in one coordinator
and preserve the current exclusive checkpoint lock until a multi-reader/multi-writer design is proven.

The practical first milestone is not “N Ghidra processes”. It is one durable
export job with dynamic chunks, reused pseudocode records, a fixed worker budget,
and progress stored outside the chat. This avoids repeated UI/MCP payloads while
allowing future parallel extraction. Compare one worker, two workers and four
workers under a hard memory budget; stop increasing workers when RSS, page faults
or queue time rises.

### P2: reduce binary and package footprint with evidence

Produce three reproducible artifact reports: `cargo bloat` or equivalent symbol
breakdown, `cargo tree -e features` for enabled dependency features, and signed
app bundle size by file. Keep UI dependencies behind `desktop`; build the MCP
binary without desktop features; verify whether optional image, accesskit and
platform window features are actually needed by the shipped target. Do not
remove a feature solely because it is large: prove that the behavior is unused
and run the existing build checks.

Benchmark a size profile (`opt-level="z"`, with and without thin LTO) against
the current release profile on the analysis corpus. Cargo explicitly warns that
size levels can have surprising speed/size tradeoffs. Keep the decompiler at
level 3 unless the benchmark shows an acceptable decompilation regression. Use
`strip="symbols"` for release artifacts and keep debug information in a
separate development artifact; do not strip information needed for crash
diagnostics without retaining symbol files.

## Concrete owners and acceptance criteria

| Area | Owning files/API | First measurable acceptance |
| --- | --- | --- |
| Restore allocations | `checkpoint.rs`, vendor `snapshot.rs` | peak RSS and allocation count for the same checkpoint; no correctness regression; cold and warm wall time reported |
| Engine release | `analysis/engine.rs`, `worker/thread.rs`, storage connections | idle release lowers RSS or allocator footprint within a documented interval; all cleanup errors preserved |
| Streaming objects | `reconstruction/objects.rs`, export callers | no simultaneous full compressed/decompressed buffers for streamed export; digest and byte limits remain enforced |
| Immutable sharing | `native_ghidra` session/architecture boundary | one image allocation per process; mutable contexts remain isolated; no checkpoint lock races |
| Bounded jobs | decomp batch worker and reconstruction queue | cancellation, queue bytes, worker count, and checkpoint progress survive restart |
| Binary size | `Cargo.toml`, bundle script | report size and startup/decompile throughput for each profile; no unmeasured profile swap |

For every comparison, record binary hash, engine revision, OS/architecture,
input size and function count, cold versus warm state, peak RSS, CPU time, wall
time, page faults, bytes read/written, cache hits, completed functions and
failures. A lower RSS number with more repeated analysis is not an improvement.
Likewise, a faster partial export is not a faster complete decompilation.

## Limits

No source establishes that Prism can reconstruct arbitrary binaries “in any
language” with behavioral equivalence. Missing symbols, optimized or indirect
control flow and target-language runtime differences remain correctness limits.
No source or current measurement establishes a 10x end-to-end speedup or a
one-hour result for a 200,000-function image. The highest-confidence resource
path is to remove restore-time copies, release idle engines and stream bounded
objects before increasing native parallelism. Those changes are measurable and
compatible with the current batch-only decompilation contract.
