# Hotpath profiling, second pass — 2026-10-26

The second pass added profiling boundaries for the recently implemented feature
workflows, reproduced and fixed three issues, and passed the final Rust and
macOS acceptance checks. The final feature workload completed 51 MCP calls,
including five specifically checked rejections, with all 321 fixture functions
indexed and six valid hotpath snapshots. This supplements the first pass in
`docs/HOTPATH_PROFILE_20261026.md`.

The initial snapshot contained 508 Rust files and matched the preceding pass's
final source inventory (`build/profile/round2/source-delta.json`). The expanded
workload covers features already present in that snapshot; it is not evidence
of source changes between those two snapshots.

## Findings and fixes

All three findings are P2. Each correction was reviewed for ownership, error
handling, resource safety, and structural readability under the project's Rust
skills and AGENTS.md requirements.

| Location | Rule and reproduced consequence | Correction |
| --- | --- | --- |
| `src/worker/execution.rs:82`, `src/storage/execution.rs:71` | Preserve evidence on partial failure. Debugger capture invalidated prior runtime evidence before capture/persistence succeeded. Simply moving invalidation after a committed insert also left a saved trace and current prior evidence when invalidation failed, preventing retry with the same immutable trace ID. | Capture first, then insert the trace and invalidate prior runtime evidence in one SQLite transaction. A failed insert or invalidation preserves earlier evidence and leaves the trace ID retryable. Imported traces retain their separate behavior. |
| `src/worker/function_search.rs:11` | Borrow stable inputs for inspection. Indexing rebuilt an owned decompilation context for every function and saved view: 965 context constructions over two pages containing 321 functions. | Construct one context and cache namespace per page request and borrow them while assembling documents. Cache identity, cancellation, and configuration scope remain intact. |
| `src/comparison/capture.rs:310` | Validate complete file-backed ranges without rejecting valid input. Listing hashing required the span to fit inside one object section, so a contiguous listing crossing adjacent sections became unknown. | Hash the length-prefixed span across adjacent file-backed sections and reject actual gaps. The final instruction length was already decoded before this fix; this is not a missing-tail fix. Single-section digest format and cache namespace remain unchanged. |

Regression evidence under `build/profile/round2/`:

* `comparison-regression-red.log` reproduces the adjacent-section rejection
  using the previous span algorithm in a standalone test.
  `comparison-regression-green.log` and `comparison-tests-clean.log` record the
  correction. Repository tests also cover changed tail bytes and unbacked gaps.
* The final failing test in `execution-regression-red.log` uses the previous
  ordering in an isolated repository copy with real SQLite storage. It observes
  current evidence becoming stale after a duplicate trace write fails. Earlier
  compile failures in that log are setup failures, not regression evidence.
* `execution-atomic-red.log` reproduces the intermediate implementation's second
  partial failure: a real SQLite trigger aborts evidence invalidation after the
  trace had committed. `execution-atomic-green.log` records both repository
  regressions passing after the transaction fix, including retry with the same
  trace ID after removing the failing trigger.

Modified methods and their immediate dependencies were reread after formatting,
including the shared connection used for invalidation, search context/cache
identity, and comparison instruction boundaries. Manual review focused on these
changes and the new feature workflows. The automated 508-file inventory does
not represent line-by-line manual review of every Rust file or establish
universal correctness.

## Instrumentation and workload

Twenty-eight opt-in `hotpath::measure` scopes were added across 15 Rust files:
worker search, comparison, verification, investigation, execution, and repair;
search, verification, execution, and type-recovery storage; investigation API
dispatch; comparison capture/matching; and type inference. Default builds keep
profiling disabled.

`scripts/profile-features.py` builds two local native fixtures, owns isolated
application sessions, reconnects to a new hotpath HTTP session after restart,
and records cleanup failures. Expected rejections must contain their specific
error message; arbitrary failures do not count as successful negative cases.

| Surface | Observed final acceptance |
| --- | --- |
| Search and comparison | 321 functions indexed over two pages; text search and an explicit structural exemplar candidate; two-image comparison and version matches. |
| Verification | Pseudocode evidence capture/retrieval; claim proposal, verification, retrieval, listing, and revision. The text predicate is corroborated; unknown evidence digest is rejected. |
| Investigation | Start, next task, blocked task/retry, pause/status/resume, recovery of the same task with an incremented attempt, and completion. The complete saved graph matches after restart. |
| Execution evidence | Synthetic function-entry and sample events imported, listed, read exactly, and associated with an indexed function. Duplicate IDs, wrong target hash, and invalid mapping are rejected. Removal is followed by an unknown-trace rejection. |
| Analysis repair | Scan, proposal with explicit harness evidence, status/evidence, rerun, status, and discard. The comparison remains behavior-unverified. |
| Type recovery | A synthetic prototype-shaped proposal is checked, accepted, and read back. Arrays and aliases are outside this runtime workload. |
| Profiler | Six snapshots provide status, timing, allocation, and threads. HTTP authentication and Prism stdio contracts are checked separately. |

`build/profile/round2/features-atomic-final/features.json` records `status: ok`,
no errors, 46 successful calls and five checked expected rejections. Native REA
returned no static CFG blocks for this fixture, so graph provenance was checked
but execution-to-block correlation is explicitly
`unavailable_without_static_blocks`. The separate executed-function result
contains the function-entry observation. No live debugger or Frida trace was
recorded.

## Measurements

These rows come from allocation-enabled builds. Timings include instrumentation
and wall-clock provider waits; they are not CPU profiles. Nested timing totals
overlap and must not be summed. Allocation rows report exclusive gross allocated
bytes, not retained memory or RSS. Restart creates a separate process/session.

The index comparison uses `hotpath_after_index` from
`features-instrumented-confirmed/features.json` before context reuse and
`features-atomic-final/features.json` afterward. Both indexed 321 functions over
two pages:

| Scope | Before | Final |
| --- | --- | --- |
| Search context calls | 965 | 2 |
| Search context exclusive allocation | 1.4 MB | 2.9 KB |
| Indexing inclusive total | 24.05 ms | 19.71 ms |
| Search-page inclusive total | 13.95 ms | 10.37 ms |
| Search document storage inclusive total | 9.44 ms | 8.95 ms |

The context allocation reduction is approximately 99.8% within that scope,
not application memory overall. These are bounded workload observations, not a
repeated benchmark establishing general speedups.

Selected final rows before restart (`hotpath_before_restart`):

| Scope | Calls | Inclusive total | Exclusive allocation |
| --- | --- | --- | --- |
| Worker comparison | 1 | 2.16 s | 929 B |
| Comparison capture | 2 | 1.69 s | 3.3 MB |
| Comparison matching | 1 | 645.79 µs | 526.1 KB |
| Semantic search | 2 | 9.05 ms | 13.0 KB |
| Investigation API | 9 | 3.86 ms | 588.3 KB |
| Verification claim worker | 5 | 1.49 ms | 45.3 KB |
| Verification evidence worker | 3 | 611.54 µs | 12.0 KB |

Selected final rows after restart (`hotpath_after`):

| Scope | Calls | Inclusive total | Exclusive allocation |
| --- | --- | --- | --- |
| Execution evidence worker | 10 | 342.57 ms | 39.4 KB |
| Analysis repair worker | 7 | 37.61 ms | 462.0 KB |
| Repair rerun, nested above | 1 | 32.12 ms | 333.4 KB |
| Type-recovery storage | 4 | 915.25 µs | 46.7 KB |
| Type inference, nested above | 3 | 21.00 µs | 6.8 KB |

Comparison capture and the provider-backed execution graph request account for
most of these feature timings. Execution worker p95 was 341.57 ms; local trace
save totaled 389.71 µs over four import attempts. This separates provider waits
from local persistence and does not measure live debugger capture.

Before restart, hotpath reported 160.9 MB RSS and 78 registered thread rows, 17
not marked exited. After the restarted workload it reported 52.4 MB RSS and 60
registered rows, again 17 not marked exited. Registered rows include completed
request threads and are not simultaneous live threads. Different startup/catalog
work makes these RSS snapshots unsuitable for claiming a reduction across restart.

Full request latencies, function logs, allocations, and thread data remain in the
feature artifact. A compact extraction and source freshness checks are recorded
in `build/profile/round2/final-verification.json`.

## Final verification and provenance

The build used `cargo build --profile profiling --features
profiling-alloc,profiling-mcp --bins`, rustc 1.93.1
(`01f6ddf7588f42ae2d7eb0a2f21d44e8e96674cf`), LLVM 21.1.8, and hotpath 0.29.2
from the lockfile on `aarch64-apple-darwin`. All final acceptance runs used the
copied binaries in `build/profile/bin/round2-atomic-final/binaries.json`:

| Binary | SHA-256 |
| --- | --- |
| `prism` | `1e9bd8fff9b581be8b18651ed9a5ace282c4b8a1973d8c87c6cd24ec663db1dc` |
| `prism-mcp` | `dcf18f241d698f3d3b46a6479ddf3918f3fa6974045fc6139b96df20278ad505` |

After the final atomicity fix:

```sh
RUSTC_WRAPPER= python3 scripts/check-rust.py \
  --report-dir build/rust-review/round2-atomic-confirmed
```

All 24 checks passed. The report was generated at
`2026-10-26T08:27:06.597231+00:00`, inventorying 508 Rust files and 874 project
files. Every Rust hash was checked against the working tree afterward and
matched. The configured main, MCP-only, desktop, profiling, and vendored-markdown
combinations cover formatting, clippy, tests, and docs where configured. Logs and
hashes are in `build/rust-review/round2-atomic-confirmed/coverage.json`.

An earlier matrix failure used a stale compiled markdown dependency lacking the
current `options.rs` module. Cleaning only that dependency's build artifacts
with `cargo clean -p egui_commonmark_extended` and bypassing the compiler wrapper
resolved it. Failed logs were retained; no source workaround was introduced.

Final acceptance against the hashes above:

* `build/profile/round2/desktop-atomic-final/profile.json`: no errors; 13 views,
  15 settings pages, project save/reopen, and target reopen. The isolated fixture
  retained one chat and 500 messages, with intro disabled. No transient failure
  occurred in this final run. This checks navigation/persistence, not every
  rendered row or interactive state. Diff had no selected baseline, Debugger no
  attached live session, and Reconstruct did not run a paid agent.
* `build/profile/round2/native-atomic-final/output.json`: The Unarchiver and
  CodexBar each decompiled three selected functions; Chrome's launcher decompiled
  its one indexed function. All seven succeeded, jobs completed, and all three
  sessions used MCP protocol `2025-11-25` without errors. The reported 300-row
  inventories for larger applications are first-page counts, not total counts.
* `build/profile/round2/hotpath-contract-atomic/results.json`: missing and wrong
  HTTP authorization returned 401; authorized initialize, tool listing, and
  profiler status returned 200. Prism stdio initialize/tool listing and clean
  EOF passed without cleanup errors. Credentials and raw headers are excluded.

Native runtime `--version` reports `6.7.0`; its installation is
`/Users/tihan-nico/.local/share/rea-rust/versions/6.7.0+189f3474bfc5/aarch64-apple-darwin/bin`.
Hashes below were read directly from those files:

| Runtime file | SHA-256 |
| --- | --- |
| `rea` | `22d71d01fca7ee33159ea5625706428d4da515a701d1191c866b2d39efe77188` |
| `rea-ghidra-rust-worker` | `175dc42ac921ee6f090231bb891a46a8a0fb40e976c5243195d5039cf07bd463` |

Intermediate artifacts, including the earlier baseline's invalid post-restart
snapshot, are retained for diagnosis and are not combined with final data. The
first pass's stdout-routing fix remains protected by the contract check; it is
not a new second-pass finding.

This establishes behavior for the recorded macOS workloads. Windows, Linux,
live debugger/Frida capture, missing static graph support, broader type forms,
paid-agent workflows, and universal FFI soundness remain outside the observed
acceptance coverage.
