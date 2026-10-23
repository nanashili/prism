# Performance audit and reproducible benchmarks

Audit date: 2026-10-23. Baseline: `4c0040dacea5dc22fe61e0ff1cf3fd5a6a380e34`.

This review covers first-party inventory/storage, reconstruction, graph previews,
UI text preparation, import, and MCP transport paths. Seven groups of changes
remove measured unnecessary work without changing the release optimization profile
or adding dependencies. The remaining findings below are part of the audit, not
claims that every performance problem has been fixed.

The older performance documents contain explicitly historical measurements, often
from macOS and an earlier native worker. Their timings and memory figures do not
validate this checkout. In particular, the current default Cargo features select
Java Ghidra through REA; the dated Rust-default notices in older reports are not a
description of the current default build.

## Measurement conditions

- Linux x86_64 cloud VM, AMD EPYC 7763, five exposed logical CPUs, approximately
  17 GiB RAM; Rust 1.93.1 and the committed lockfile.
- Repository release profile: `opt-level="s"`, thin LTO, one codegen unit.
- Serial execution, with no concurrent builds during timed sections. Generated
  fixtures, warm filesystem/SQLite caches, and medians of 7–21 samples.
- MCP measurements include loopback socket receiving and a sender thread.
  Lookup measurements use file-backed SQLite with the production store settings.
  Reconstruction uses the real schema in an in-memory SQLite database, rolling
  back between samples. Graph measurements include the row-vector copy, order
  detection, optional index construction, and preview extraction; they exclude
  unchanged dossier/assembly parsing and external-engine execution.
- The graph speedup fixture uses disjoint eight-instruction blocks in reversed
  address order. The reconstruction fixture is a dependency chain with one unit
  per module and overlapping changed roots. These are repeatable scaling cases,
  not a sample of real projects.
- The UI harness is overlaid on the baseline checkout without changing its
  production code. Other benchmarks retain the old algorithm or schema in test
  code and compare it with the production implementation on equivalent fixtures.
  Paired benchmarks alternate order. UI measurements use separate baseline and
  candidate builds, so their small differences are more sensitive to VM noise.
- These are operation-level measurements. No GUI frame rate, whole-application
  memory, real-engine decompilation throughput, macOS/Windows behavior, or cold
  disk-start improvement is established.

## Results

All times are medians. The table shows the workload for each result; speedups
apply only to that operation and fixture.

| Operation | Before | After | Ratio |
| --- | ---: | ---: | ---: |
| Exact address lookup, 200,000 functions | 31.699 ms | 2.939 µs | 10,786× |
| Exact unique-name lookup, same inventory | 31.038 ms | 3.181 µs | 9,757× |
| Missing function, same inventory | 30.656 ms | 2.705 µs | 11,333× |
| Ambiguous name shared by 100,000 functions | 5.180 µs | 3.201 µs | 1.62× |
| MCP receive, 1 MiB | 10.054 ms | 0.729 ms | 13.79× |
| MCP receive, 8 MiB | 588.411 ms | 2.633 ms | 223.49× |
| Syntax preparation, 64 visible rows / 3,544 bytes | 95.859 µs | 41.221 µs | 2.33× |
| Syntax preparation, 63,488-byte generated expression | 1.356 ms | 0.629 ms | 2.15× |
| Reverse-order graph, 128 blocks / 1,024 instructions | 133.015 µs | 69.553 µs | 1.91× |
| Reverse-order graph, 4,096 blocks / 32,768 instructions | 90.818 ms | 2.858 ms | 31.78× |
| Reconstruction invalidation, 128 modules / 32 changed roots | 136.545 ms | 5.761 ms | 23.70× |
| Reconstruction invalidation, 512 modules / 32 changed roots | 2,025.049 ms | 68.157 ms | 29.71× |
| Project-index JSON, 2,000 projects / 344,671 bytes | 64.398 ms | 3.643 ms | 17.68× |
| Remove language hints, 4,096 fences / 895,914 bytes | 34.779 ms | 2.224 ms | 15.64× |

The exact-lookup ratios are large because the old query scans all 200,000 rows to
establish uniqueness or absence; the replacement performs two bounded index
seeks. They are not a general database or application speedup.

**Controls and tradeoffs:** small or already-efficient cases show little change,
including small regressions in their medians. A 16-KiB MCP reply moved from 78.598
to 80.963 µs; the large sorted graph from 1.722 to 1.778 ms; one wide block from
27.469 to 27.942 µs; single-root invalidation from 5.460 to 5.543 ms; and eight
Markdown fences from 4.665 to 4.726 µs. No improvement is claimed for those cases.
Wide overlapping graphs moved from 103.913 to 99.172 µs, and duplicate-heavy ones
from 101.834 to 94.776 µs. Their purpose is to guard against the much larger
regression an eager/unbounded address-index implementation would introduce.

The function index increased the 200,000-row fixture database from 17,006,592 to
26,570,752 bytes (**+9.12 MiB**). Fixture import took 353.343 ms without the index
and 582.285 ms with it. Those import figures are single setup observations,
including generated-row allocation, inserts, and commit, not repeated throughput
benchmarks. Existing databases also pay a one-time index-building cost on open;
that migration latency was not timed. The index is created idempotently and is
covered by an existing-database reopen test.

For the 64-row syntax workload, layout sections fell from 1,552 to 384 and their
aggregate backing capacity from 212,992 to 53,248 bytes (**75% less**). For the long
expression, sections fell from 24,576 to 1 and backing capacity from 3,407,872 to
416 bytes. These figures count only `LayoutSection` vector capacity; they are not
process RSS or total rendering-memory measurements.

When built, the graph address index adds one `usize` per instruction, approximately
256 KiB for 32,768 instructions on this host. It is local to graph construction;
sorted input and the broad-prefix/single-block fast paths do not allocate it.

Raw output: [baseline](performance/2026-10-23-baseline.txt),
[candidate](performance/2026-10-23-candidate.txt), and
[environment metadata](performance/2026-10-23-environment.json).
The recorded candidate HEAD is the pre-commit baseline; `candidate_src_sha256`
identifies the tested working-tree Rust sources, including the added harnesses.


## Changes and preserved behavior

1. **Exact function navigation:** the old `(address=? OR name=?)` query scanned
   the project inventory. A covering `(project,name,address)` index and two
   disjoint `UNION ALL` seeks stop after the two matches needed for ambiguity
   detection. Excluding the matching address from the name branch preserves
   self-match deduplication without sorting every duplicate name. Project scope,
   exact case matching, and malformed-row errors remain covered. Existing stores
   create the index once on opening. This trades storage and import work for
   interactive lookup speed; it does not optimize substring searches.
2. **MCP replies:** each read now searches only newly received bytes for newline.
   Previously, a near-8-MiB frame received in 4-KiB chunks repeatedly searched
   roughly 8 GiB in aggregate. Cancellation checks, deadlines, EOF errors, and
   the existing byte limit remain unchanged.
3. **Syntax highlighting:** the source string is copied once, and adjacent tokens
   with the same formatting share a layout section. Tests cover all color schemes,
   light/dark themes, Unicode boundaries, quotes, escapes, and comments. This
   measures layout-job preparation; egui glyph layout and painting are separate.
4. **Unordered graph assembly:** narrow block previews use a lazily built address
   index. Sorted input retains binary search; single-block queries, wide ranges,
   and duplicate-heavy prefixes retain early exits. Index selection examines at
   most 64 candidates per block before falling back to the original scan. Tests
   preserve provider order, duplicate addresses, exclusive end bounds, and the
   twelve-line preview limit. Wide overlapping ranges were added as regression
   workloads after review exposed a slower initial implementation.
5. **Reconstruction invalidation:** three recursive statements start from the
   union of changed roots instead of repeating all three traversals per root.
   Shared consumers and cycles are visited once per statement. Generation scope,
   state filters, action stages, leases, and transaction rollback behavior remain
   equivalent. No reconstruction schema migration is needed.
6. **Metadata import and endpoint JSON:** file decoding uses buffered readers.
   Chat import lazily builds one exact-path project lookup shared across loose
   threads and archived chats, rather than rereading and parsing the index for
   every inferred association. Explicit project IDs still need no index;
   duplicate paths retain the first association. The timing below measures index
   decoding once; single-read reuse is separately verified by a regression test.
7. **Markdown without code colors:** fenced-language hints are removed by copying
   retained spans once, replacing repeated suffix shifts. Unicode, CRLF, nested
   quoting, indentation, tilde/backtick fences, and unterminated fences preserve
   the baseline output. Text requiring no change remains borrowed.

## Remaining findings

These source-confirmed costs are not included in the improvement percentages.
Priority describes follow-up value for large workloads; real-user latency and
engine-dependent effects still need measurement.

| Priority | Trigger and evidence | Remaining work |
| --- | --- | --- |
| High | [Durable result pages](../src/storage/decompilation/pages.rs) join from results and order by inventory position before applying LIMIT. A small response can sort a whole job. | Benchmark an inventory-first ordered query and a cursor. Define handling of orphan legacy results before changing the existing left-join semantics. |
| High | [Whole-pseudocode export](../src/worker/exports.rs) repeatedly calls [function pages](../src/storage/database.rs) with increasing OFFSET; the empty-filter query still evaluates text predicates. | Add an internal keyset iterator for export. Verify every function is emitted once and preserve the external offset-based API. |
| High | [Snapshot export](../src/reconstruction/snapshot.rs) holds its write transaction through decompression and durable per-file writes. | Measure blocked writers on large snapshots. Shortening the transaction requires preserving revision consistency and failure receipts. |
| Medium | [Strings misses](../src/worker/analysis.rs) scan the input but do not populate the cache they consult; Imports always reparses file inventory. | Exercise repeated view navigation on large files, then populate the existing cache under explicit identity/invalidation rules. |
| Medium | [Android method reads](../src/analysis/android.rs) parse the class JSON again for each requested method; a class file can be up to 16 MiB. | Measure same-class navigation and cache bounded parsed/source results with an explicit jadx-output lifetime. |
| Medium | [Sidebar queries](../src/storage/navigation/query.rs) rebuild renamed rows per page and repeat full-inventory match/group/sort work. [Resource root pages](../src/storage/resources.rs) invalidate and recursively rebuild their index. | Separate data revisions from paging/refresh; benchmark filter changes and warm root navigation with realistic inventories. |
| Medium | Expanding “Read full message” beyond 64 KiB bypasses [Markdown caching](../src/ui/message/body_cache.rs). Default previews remain bounded to 16,000 bytes. | Profile expanded large messages and consider bounded block-level caching/virtualization rather than removing memory limits. |
| Medium | [Function](../src/ui/bench_sidebar/rows.rs) and [resource](../src/ui/bench_sidebar/resource_rows.rs) tree flattening rebuild row descriptors/parent strings per repaint, including offscreen expanded rows. | Profile deeply expanded trees; cache by tree revision or traverse only the visible range while preserving selection and expansion state. |
| Medium | [Tool validation](../src/mcp/validation.rs) reconstructs the schema catalog per request, and validation happens in both helper and bridge. | Benchmark a bounded immutable catalog per mode; preserve all validation checks and public tool discovery output. |
| Medium | [Tool previews](../src/agent/tool_updates.rs) join up to sixteen complete content strings before clipping to 16 KiB. | Append only the retained UTF-8-safe prefix; test separators and clipping around multibyte characters on large replies. |
| Lower / profile first | [Remembered permissions](../src/agent/remembered.rs) reread and parse a bounded file under a shared mutex for every check. UI chat/mention handlers also perform synchronous bounded SQLite reads. | Measure contention and UI stalls before caching or moving work; permission revocation and update visibility must remain correct. |
| External-engine dependent | [Rizin](../src/analysis/rizin.rs) repeats plugin discovery for uncached pseudocode and can duplicate assembly work for graph requests. | Benchmark with an installed Rizin/decompiler session and preserve process/session invalidation. |

Worker polling and decoded JSON queue memory also deserve profiling, but source
inspection alone does not establish an idle-CPU regression or a leak. Increasing
worker concurrency, removing bounds, switching renderers, and globally changing
release optimization levels are not supported by these measurements. The vendored
Rust decompiler is not a dependency of this checkout's Cargo application and was
not benchmarked as though it were the active engine.

## Reproduce and validate

From the repository root, with the pinned Rust toolchain installed:

```sh
python3 scripts/benchmark-performance.py
```

The driver creates a temporary detached baseline worktree, overlays only the two
UI test modules, forces the Prism package to rebuild when switching source trees,
and runs the candidate benchmarks serially. Dependencies remain cached. Results
and environment metadata go to `target/performance/`. Forcing the package rebuild
prevents same-named test binaries from being reused across the worktrees.

To run only current paired benchmarks and UI timing:

```sh
cargo test --release --locked --all-targets benchmark_ -- --ignored --nocapture --test-threads=1
```

This filter excludes the existing optional network-download test. Benchmarks use
output equality checks and repeated timings, not brittle speed thresholds.

Validation commands:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets -- --test-threads=1
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo check --locked --no-default-features --bin prism-mcp
cargo build --release --locked --bins
```

Formatting, all-target tests, strict all-target/all-feature Clippy, and the
standalone MCP check, and release binary build passed. Normal tests: **31 passed**, with seven performance
tests and the existing optional network-download test ignored. The explicit
release benchmark run passed all **seven** performance tests. Baseline UI golden
tests also passed before timing. GUI interaction and live external-engine
sessions were not run in this headless environment.
The built MCP helper passed `initialize` and `tools/list` smoke checks in combined,
window, and analysis modes without requiring a running desktop.
