# Hotpath profiling report — 2026-10-26

This report records the completed timing and allocation profiling pass,
the MCP contract checks, and the source corrections exercised by those runs.
The final timing/allocation artifacts and verification checks are recorded
below. This report does not claim universal coverage or that every bug in the
application has been found.

## Provenance

| Item | Value |
| --- | --- |
| Timing report | `build/profile/timing-before/profile.json` |
| Timing-after report | `build/profile/timing-after/profile.json` |
| Complete timing report | `build/profile/timing-complete/profile.json` |
| Desktop report | `build/profile/desktop-complete-before/profile.json` |
| MCP scenario report | `build/profile/scenarios-before/scenarios.json` |
| Binary manifest | `build/profile/bin/timing-before/binaries.json` |
| Timing-after binary manifest | `build/profile/bin/timing-after/binaries.json` |
| Complete timing binary manifest | `build/profile/bin/timing-complete/binaries.json` |
| Allocation report | `build/profile/allocation-complete/profile.json` |
| Allocation desktop report | `build/profile/desktop-allocation-complete/profile.json` |
| Optimized allocation report | `build/profile/allocation-optimized/profile.json` |
| Optimized allocation desktop report | `build/profile/desktop-allocation-optimized/profile.json` |
| Corrected chat fixture UI acceptance | `build/profile/desktop-allocation-ui-verified/profile.json` |
| Optimized allocation binary manifest | `build/profile/bin/allocation-optimized/binaries.json` |
| Hotpath HTTP auth contract | `build/profile/hotpath-contract/hotpath-auth.json` |
| Prism stdio MCP contract | `build/profile/hotpath-contract/stdio.json` and `build/profile/hotpath-contract/results.json` |
| REA runtime metadata | `build/profile/smoke-final/runtime.json` |
| Optimized smoke report | `build/profile/smoke-optimized/report.json` and `runtime.json` |
| Optimized MCP scenarios | `build/profile/scenarios-optimized/scenarios.json` |
| Final Rust review | `build/rust-review/final-profile-confirmed/` |
| Timing `prism` SHA-256 | `d6d9cd67b96e02411d72b27c9e9695aa7b75208720f1c8f74dc657c68eb32ced` |
| Timing `prism-mcp` SHA-256 | `2f29fb1a80d29e26c2f4b80d2a736863a030453905925c811018ff167ddf8a41` |
| Timing-after `prism` SHA-256 | `4417bb9cbfa30667a0fe48b72e54452119447fafacb8bafef6bd2a46a87d2130` |
| Timing-after `prism-mcp` SHA-256 | `9c3f5d77c4d050d8e19c16e55e8884cfbbeed5d38b6b0355edcaa5100f882035` |
| Timing-after compiler | `rustc 1.93.1 (01f6ddf75 2026-02-11)` |
| Complete timing `prism` SHA-256 | `235439735827fa328c3b5775f9702d134818ef70defda83d9ae7b6bc6b7eefdb` |
| Complete timing `prism-mcp` SHA-256 | `294331adcae0d4d26e95e7fd6bc1dfcfcd35172a28389f2d19e332ff479dccba` |
| Optimized allocation `prism` SHA-256 | `560a6c28bfa08f1bc93b2669a15e4ee83073dd47b113089a708a8b6bff94a425` |
| Optimized allocation `prism-mcp` SHA-256 | `dcf18f241d698f3d3b46a6479ddf3918f3fa6974045fc6139b96df20278ad505` |
| Host | macOS; reports were written 2026-10-26 Africa/Johannesburg time |
| Native provider | REA Rust 6.7.0 (`189f3474bfc5dfd6f3ed2689dfa765c7a4e9f770`) with the matching Ghidra Rust worker |
| Provider binaries | `rea` SHA-256 `22d71d01fca7ee33159ea5625706428d4da515a701d1191c866b2d39efe77188`; worker SHA-256 `175dc42ac921ee6f090231bb891a46a8a0fb40e976c5243195d5039cf07bd463` |
| Timing feature set | `profiling-mcp` (timing/thread instrumentation; allocation queries correctly reported unavailable) |

The timing executable was kept in the manifest before subsequent source or
binary changes. Hotpath MCP advertised 30 tools in these runs, including
`profiler_status`, `functions_timing`, `function_timing_logs`, `threads`, and
the allocation endpoints. The allocation endpoints returned the expected
“enable hotpath-alloc feature” error in this timing build.

## Headless baseline

The harness exercised Prism's stdio MCP initialization and tool discovery,
paged and filtered function listing, analysis-tool discovery, bounded byte
reads, and durable decompilation. It selected up to three functions per target
and recorded request latency alongside before/after hotpath snapshots.

| Target | Indexed functions | Open/index timing | Request latency (min / mean / max) | Before RSS | After RSS |
| --- | ---: | --- | ---: | ---: | ---: |
| The Unarchiver | 300 returned (2,269 indexed in UI) | `execute` 689.97 ms; `index` 528.84 ms; `prepare_engine` 488.27 ms | 24.942 / 82.497 / 512.069 ms | 22.8 MB | 121.3 MB (133.9 MB max) |
| Google Chrome | 1 | `execute` 369.35 ms; `index` 310.04 ms; `prepare_engine` 307.98 ms | 23.988 / 87.270 / 564.097 ms | 20.8 MB | 124.3 MB (135.0 MB max) |
| CodexBar | 300 returned | `execute` 72.43 s; `index` 72.30 s; `REA::call_tool` 71.34 s across 443 calls | 3.550 / 71.096 / 508.993 ms | 19.2 MB | 121.4 MB (130.8 MB max) |

The long CodexBar open is dominated by the external provider's 443 tool calls,
not by a measured Prism-only function. Hotpath request latency itself ranged
from 0.268 to 9.395 ms across these targets. These values are workload samples,
not performance budgets.

## Desktop baseline

The desktop run launched the native window, opened and analyzed The Unarchiver,
checked the workbench, rendered assembly, pseudocode, references, strings,
imports, segments, hex, resources, notes, bookmarks, graph, diff, debugger, and
reconstruction views, visited all listed settings pages, ran a bounded
decompilation, saved a project, reopened it, reopened the target, and captured a
window image. The UI state reported 2,269 indexed functions and a successful
round trip with the original target SHA-256 preserved.

Before the UI workload, identity construction was the largest measured path:
`decompilation::identity::new` 223.07 ms (48.66%),
`backend_identity` 223.06 ms (48.65%), and three fingerprint calls totalling
222.47 ms (48.53%). After the workload, repeated worker execution and external
analysis dominated: `worker::execute::execute` 1.86 s across 25 calls,
`worker::open::open` 1.44 s across three calls, and `analysis::rea::call_tool`
1.10 s across 12 calls. Desktop MCP request latency was 0.037 / 27.790 /
189.038 ms (minimum / mean / maximum), and hotpath requests were 0.370 /
3.073 / 17.983 ms.

RSS was 135.8 MB at the initial desktop snapshot and 184.8 MB after the full
workload. The run observed 15 threads initially and 14 after the workload.

## MCP contract and lifecycle scenarios

`build/profile/scenarios-before/scenarios.json` used `/bin/ls` and completed with
19 Prism tools and 50 indexed functions. It deliberately exercised invalid
argument handling (`read_bytes` rejected an invalid negative offset, `-1`),
the direct native decompile guard, unknown-tool rejection, durable
decompilation pause/resume,
two-function completion, paged results, reconstruction generation setup,
module/dependency inspection, evidence and submit no-ops, reporting, and build
availability checks. The decompilation job completed 2/2 with 385 artifact
bytes; reconstruction remained pending with 51 units and no behavior-verified
claim. This verifies observable boundaries and state transitions, not source
equivalence.

## Corrections made before the completed runs

Findings are grouped by severity. P1 means the profiling MCP path could break a
protocol contract; P2 means a correctness, resource-safety, or material
performance defect with a narrower trigger.

| Severity and rule / location | Consequence observed or reproduced | Fix |
| --- | --- | --- |
| P1 — Profiling MCP protocol contract, `src/profiling.rs`, `src/main.rs`, `src/bin/prism-mcp.rs` | Profiling shutdown/report output could corrupt the JSON-RPC stdio stream when profiling integration was enabled. | Keep the profiler guard alive through command completion and route profiling MCP output away from stdout. This safeguard applies to the profiling integration path; it does not imply the default unprofiled application emitted profiler output. |
| P2 — Bridge framing, `src/bridge/incoming.rs:read_frame` | A request and trailing bytes delivered in one TCP read were truncated at the newline, so unexpected data was accepted outside the request frame. | Preserve the complete read and reject bytes after the request frame. Regression test: `rejects_data_after_the_request_delivered_in_the_same_read`. |
| P2 — Storage accounting, `src/storage/usage.rs:Usage::add_file_bytes` | A `u64` overflow could leave category and total accounting inconsistent. | Check category and aggregate sums before mutating state; return an error on overflow. `Usage::total` uses saturating aggregation for reporting. |
| P2 — Storage scan ownership, `src/ui/settings_storage/measurement.rs` | Invalidating a running scan could drop its receiver and lose its completion result while the detached worker continued. | Keep the receiver owned until the worker reports; retain cancellation and mark a restart so the next scan starts after completion. |
| P2 — Backend identity, `src/analysis/rea/identity.rs:backend_identity` | The worker fingerprint was computed once for components and again for the `worker` field, doubling digest work. | Reuse the computed worker identity in both fields. |
| P2 — Inventory page size, `src/analysis/inventory.rs` | Retained procedure pages were limited to 256 rows, causing avoidable provider pagination for larger inventories. | Raise the retained page limit to 4,096; the completed runs verified the reduced provider-call path without a response failure. |
| P2 — Catalog validation, `src/mcp/catalog.rs` | `Incoming::read` rebuilt the full mode catalog on every request: 99 calls consumed about 101 MB, roughly 1 MB per request. | Cache each mode catalog in a `OnceLock` and validate through borrowed definitions while preserving the public owned API. |
| P2 — Profiler collection setup and live MCP cap | The global counting allocator was not installed for every profiling path, and the live MCP view used the old default 15-entry cap. | Install the global counting allocator and return every measured function in live snapshots; detailed invocation logs remain bounded. |

## Timing-after comparison (interim)

The interim after report is `build/profile/timing-after/profile.json`; its
binary identities are recorded above and in
`build/profile/bin/timing-after/binaries.json`. The same local macOS targets
were exercised, but the artifacts are separate runs, so the comparison is a
measurement of the recorded harness outputs rather than a universal speedup
claim.

| Target | Recorded baseline | Timing-after result | Provider call comparison |
| --- | ---: | ---: | ---: |
| The Unarchiver | 689.97 ms open/execute | 501.06 ms | Interim timing sample |
| Google Chrome | 369.35 ms open/execute | 318.73 ms | Interim timing sample |
| CodexBar | 72.43 s open/execute; 69.91 s inventory | 8.32 s open/execute; 6.34 s inventory | `REA::call_tool` 443 → 31 |

The timing-after request snapshots report Prism latency ranges of 4.337–424.089
ms for The Unarchiver, 2.985–504.204 ms for Chrome, and 6.246–419.595 ms for
CodexBar. Hotpath HTTP request ranges were 0.289–4.906 ms, 0.271–4.555 ms,
and 0.311–2.053 ms respectively. The after snapshots also include protocol
framing and MCP transport work, so those rows are retained as workload
evidence rather than reduced to a single function-level score.

## Complete timing and desktop evidence

The final timing artifact is `build/profile/timing-complete/profile.json`.
All three headless targets completed successfully, with 53 of 53 timing rows
recorded for each target. The final desktop artifact is
`build/profile/desktop-complete-after/profile.json`: 82 of 82 desktop timing
rows and 10 of 10 helper rows completed successfully. It records
`intro_disabled=true` and retains the 500-message fixture.

The final headless and desktop runs were launched in parallel, so their wall
times include build and CPU contention. The earlier sequential CodexBar result
(8.32 s open, 6.34 s inventory) remains the useful single-case comparison; the
final parallel run measured 11.73 s. The structural provider call reduction
remained stable at 31 calls. Final application-update timing was 380 calls,
1.20 ms average, and 893.95 µs p95; viewport timing was 274 calls, 138.86 µs
average, and 158.72 µs p95; usage measured two calls at 603.75 µs average.
Identity ran twice in headless and six times in desktop, compared with three
and nine in the baseline evidence.

The final timing tables include blocking protocol and provider waits. For
example, `protocol::framing::read`, `mcp::transport::read`, and response waits
describe wall-clock waiting and must not be interpreted as CPU hotspots.
Inclusive timing rows overlap nested calls, so their totals and percentages
must not be added together. The report does not convert frame CPU sums into
wall time.

## Allocation evidence

The initial allocation artifacts are complete for all three headless targets
and the desktop target: 53/53 rows per headless target, 82/82 desktop rows,
and 10/10 helper rows, all with nonzero allocation data. The initial headless
after totals were 189.0 MB for The Unarchiver, 183.2 MB for Chrome, and 359.5
MB for CodexBar. The desktop after total was 269.1 MB. These values come from
`build/profile/allocation-complete/profile.json` and
`build/profile/desktop-allocation-complete/profile.json`.

The optimized headless allocation artifact is complete. Its after rows contain
54/54 entries, with gross totals of 172.9 MB, 167.1 MB, and 339.8 MB for The
Unarchiver, Chrome, and CodexBar respectively. The optimized desktop artifact
also completed successfully with the 500-message fixture retained, `intro_disabled=true`,
83/83 main rows, and 11/11 helper rows. Main gross allocation was 174.1 MB,
versus 269.1 MB in the initial desktop allocation run; helper gross allocation
was 3.8 MB versus 104.7 MB.

The catalog fix must be evaluated on the combined validation path. In the
optimized desktop run, `Incoming::read` made 102 calls and allocated 184.7 KB,
while `validate_call` made 102 calls and allocated 1.0 MB (the catalog was
cold-initialized once, with approximately 950 B p95 per validation call).
Together this is about 1.18 MB, compared with 101.1 MB across the earlier
99-request path, an approximately 99% reduction. The helper path fell from
104.7 MB to 3.8 MB (about 96%). The main update path measured 370 calls at
1.38 ms average and 914.43 µs p95; viewport measured 270 calls at 148.15 µs
average and 166.53 µs p95. Frame counts differ between runs, so these totals do
not support an RSS conclusion; observed RSS was effectively unchanged (264.7
MB versus 263.8 MB).

The optimized desktop screenshots are under
`build/profile/desktop-allocation-optimized/support/20261025T234052Z-8918df61/0/`.

Hotpath 0.29.2 reports function allocations as exclusive through thread-local
nested scopes unless `HOTPATH_ALLOC_CUMULATIVE=true` is set. `total_allocated`
is cumulative gross allocated bytes; it is not retained memory or RSS, and
deallocation is tracked separately. The desktop report shows 173 historical
registry entries after the workload: 159 exited, 13 sleeping, and 1 running.
This is not a claim that 173 OS threads remained live, nor a leak finding.

## MCP contract evidence

The hotpath HTTP contract rejected both missing and incorrect authorization
with HTTP 401, while the correct token initialized successfully and listed all
30 tools. The optimized allocation rerun passed the same contract under
`build/profile/hotpath-contract/allocation-optimized/results.json`. The exact
token and loopback port were not recorded in the artifact;
they remain intentionally undisclosed. The Prism stdio contract results are
recorded separately under `build/profile/hotpath-contract/stdio.json` and
`results.json`; profiler diagnostics were kept out of the JSON-RPC stdout
stream. These are separate contracts: Prism's stdio MCP carries application
tools, while hotpath's authenticated loopback HTTP MCP carries profiler tools.

The final REA smoke metadata records version 6.7.0 and the exact native REA
and worker hashes in `build/profile/smoke-final/runtime.json`. The exercised
provider and local macOS applications are part of the evidence boundary.

The optimized smoke run passed all three apps and the optimized scenario run
passed the 29-tool MCP contract. Their artifacts are recorded under
`build/profile/smoke-optimized/` and
`build/profile/scenarios-optimized/scenarios.json`.

`python3 scripts/check-rust.py --report-dir build/rust-review/final-profile-confirmed`
passed 24/24 checks and generated coverage at
`2026-10-25T23:42:25.493775+00:00`, covering 872 project files and 508 Rust
files. The 508-file hash inventory matched with no missing files; it is
automated coverage evidence, not proof of universal manual review.

The separate `build/profile/desktop-timing-after/profile.json` captured
preliminary desktop state checks while the intro was still present. It is not
treated as final UI screenshot acceptance. The corrected harness disables the
intro and dwells 150 ms per view before the final screenshot pass. Earlier
screenshots with the intro visible are retained only as preliminary evidence.

The complete artifact records the corrected implementation, but this report
does not attribute the observed timing changes to one source correction in
isolation; the runs include provider, filesystem, and scheduling variability.

## Scope exclusions

The final matrix is recorded in the completed timing and allocation artifacts
above. Remaining exclusions are platform and workload boundaries: no Windows or
Linux behavior, graph availability on other providers, paid-agent
authentication, or universal responsiveness guarantee follows from these
macOS-local runs. The automated 508-file inventory is not a substitute for
manual review of every source path.

The final screenshots are stored under
`build/profile/desktop-allocation-ui-verified/support/20261025T234452Z-6ff5998e/0/`.
The inspected `capture.png` and `storage.png` support the workbench chat and
storage views. A visible earlier button proves only the paginated window, not
that all 500 fixture messages were rendered in one view.

## Harness correction for rich-code acceptance

Visual inspection found that the earlier 500-message fixture generator split
activity range 0–240 inside a fenced code block (`load r` / `ecord`). That
created orphan text and an empty gray code block, so the earlier capture is not
valid evidence for rich-code rendering. `scripts/profile-chat-fixture.py` now
ends the activity text at a complete paragraph boundary (0–138), so the reply
begins with a complete fence. The prior allocation comparison remains valid as
a same-fixture measurement, but its UI capture is excluded from rich-code
acceptance. The regenerated UI-verified desktop run completed successfully under
`build/profile/desktop-allocation-ui-verified/`, with the intro disabled and all
500 messages retained. Visual inspection confirmed the complete assistant code
block, the user code block, and the storage view. This harness correction does
not change the Rust matrix or catalog findings.
