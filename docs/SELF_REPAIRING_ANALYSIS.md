# Self-repairing analysis

Prism exposes `analysis_repair` through its desktop agent tools and headless
MCP endpoint. Suspicious batch pseudocode receives `analysis_warnings` with
bounded evidence excerpts. These heuristic leads cover decompiler warnings,
unresolved jump tables, function boundaries, calling conventions and type
assumptions. Scanning also checks basic-block bounds, overlaps and entry points.
Structured native dossiers contribute missing jump-table cases, unresolved calling
conventions and conflicting declarations for identical parameter storage.
Signals request investigation; they do not establish that analysis is wrong.

## Workflow

1. Run a saved-inventory `decompile_batch` and inspect suspicious results.
2. Call `analysis_repair` with `action: "scan"` and one indexed `address`.
   The returned `repair` identifies an immutable baseline and specialist task.
3. Delegate that task using the calling agent's specialist facility. The
   specialist reads `action: "evidence"`, diagnoses the failure against the
   assembly and control flow, and proposes exactly one bounded adjustment.
   If delegation is unavailable, report that limitation. Prism cannot spawn
   or authenticate specialist sessions; `specialist` is a caller-supplied
   reference and `specialist_reference_verified` remains false.
4. Submit `action: "propose"`, `repair`, and a `plan` containing `specialist`,
   `diagnosis`, and `adjustment`. For example, a justified transient-failure
   retry uses `adjustment: {"kind": "refresh_decompilation"}`.
5. Call `action: "rerun"`. Inspect `action: "status"` for comparisons and
   `action: "evidence", attempt: 1` for the complete bounded capture.
6. Use `action: "discard"` when rejecting the overlay. Discard deactivates it
   and retains its baseline, diagnosis, attempts and comparisons.

## Supported adjustments and limits

`refresh_decompilation` retries the one selected function and reads its assembly
and dossier. `inspect_control_flow` reads assembly and control flow before the
same one-function decompilation request. Both bypass Prism view caches and
saved-success reuse. The provider can still reuse prepared analysis or its own
caches: a new request does not prove a new analysis pass.

Calling-convention, boundary, type and jump-table override proposals are retained
as interpretations with a capability blocker. The current provider integration
has no reversible override API, so Prism does not apply those proposals or
pretend that a retry applied them. Implementing native semantic overrides requires
a provider transaction or reversible overlay API first.

Each overlay permits two attempts, including interrupted work. Each capture
component is limited to 192 KiB; oversized captures retain a digest and explicit
error instead of silently truncating evidence. The complete stored capture is
limited to 768 KiB. Each project has a 32 MiB overlay budget, with capacity for
both captures and the proposal reserved when the baseline is created. This is
an application quota, not a guarantee against filesystem exhaustion.

Provider requests use the configured timeout and memory limits. A rerun makes
one function-decompilation request and two independent metadata reads. Independent
reads are attempted after a failure so missing evidence is visible. Cancellation
and provider failure become observations; interrupted process work consumes an
attempt and is marked interrupted on the next ownership-locked recovery.
Partial captures are recorded as `incomplete` and remain available for inspection;
another attempt still counts toward the same limit. `complete_capture` means the
three requests returned bounded observations, not that the provider supplied every
analysis facet. Unsupported or empty CFG evidence remains unknown in comparisons.

## Evidence integrity

The project identity is the binary SHA-256. Prism checks the current file hash
before and after capture, matches provider and complete analysis configuration,
and verifies baseline and candidate digests when reading stored evidence.
The configuration includes runtime provenance for the prepared native provider.
If the post-capture identity check fails, the candidate remains recorded as
`invalidated`, with the failure reason, and consumes one attempt. It cannot be
treated as a comparison of the indexed binary. Another attempt requires the
current binary to match the indexed identity again.
A project OS lock prevents overlap with other repair and durable batch runners;
process exit releases the lock. State changes and result commits use SQLite
transactions, and discarded overlays cannot start further attempts.

Baseline pseudocode is read from a digest-checked durable artifact. Baseline
assembly and control flow are separate provider observations captured at scan
time, not a claim that they came from the original decompilation invocation.
Candidates never enter Prism's accepted pseudocode cache or durable batch results.
Provider internal caches may change during a request, but binary bytes and the
accepted Prism artifacts remain selected.

Comparisons record digests and availability for pseudocode, assembly, dossier and
basic blocks, plus added/removed diagnostic categories. Comparisons remain
`unverified` (or `invalidated` after an identity failure), with
`behavior_verified: false`, including when warnings disappear.
AI diagnoses remain separate from raw observations. This workflow cannot establish
semantic equivalence, complete jump-table recovery, correct ABI or correct types.

## Verification

Deterministic tests exercise all diagnostic categories, CFG inconsistencies,
missing evidence, evidence comparisons, failed transactions, attempt exhaustion,
interruption recovery, discard retention, corruption detection and context
mismatches. Run `cargo test --locked --bin prism analysis_repair` for these tests,
and `python3 scripts/check-rust.py` for the complete repository check matrix.

The final 2026-10-26 run passed all 24 matrix checks, including all 15 repair
regressions. The command used `RUSTC_WRAPPER=` and
`CARGO_TARGET_DIR=build/analysis-repair-target` to avoid an unavailable shared
compiler-cache daemon, with
`--report-dir build/rust-review/self-repair-final`. Individual logs and the
hashed source inventory are in that directory's `coverage.json` report.

### Native acceptance, 2026-10-26

Tested on macOS 27.2 arm64 with REA 6.7.0, installed runtime
`6.7.0+189f3474bfc5/aarch64-apple-darwin`. `scripts/smoke-rea.py` used an
isolated support profile and the locally built Prism executable. Two functions
each from The Unarchiver and CodexBar completed successfully, with no reported
protocol or shutdown errors. The results are saved in
`build/analysis-repair-smoke/final-provider-results.json`.

A separate isolated MCP exercise captured The Unarchiver function `0x1000054ac`.
The actual specialist agent `/root/repair_specialist` inspected its saved
pseudocode, assembly and dossier and proposed `inspect_control_flow`. The
specialist identified the unresolved jump-table warning and the provider's
missing CFG; the available assembly did not establish the table's targets.
Both permitted attempts returned unchanged pseudocode, assembly and dossier.
CFG comparison remained unknown, warnings remained present, and both comparisons
stayed unverified. The third attempt was rejected. Discard retained the baseline
and candidate evidence, and accepted durable analysis was unchanged. Results,
plan and machine/runtime/binary hashes are saved under
`build/analysis-repair-smoke/` as `repair-results.json`, `specialist-plan.json`
and `provenance.json`.

These checks cannot establish Windows/Linux behavior, FFI soundness or
whole-application analysis correctness.

### Manual review

Reviewed the new repair model, detectors, comparison, storage, worker and catalog,
their tests, and the modified registration, ownership and batch annotation paths
against `rust-review`, `rust-best-practices` and the project readability rules.
The specialist's review identified these P2 issues, all corrected before the
final checks:

| Rule and source | Consequence | Correction |
| --- | --- | --- |
| Preserve evidence uncertainty, `src/analysis_repair/diagnostics.rs:77` | Empty or unavailable CFG could incorrectly imply a bad boundary. | Classify it as missing evidence; compare unavailable CFG as unknown. Regression covers both cases. |
| Inspect immediate evidence dependencies, `src/analysis_repair/native_signals.rs:37` | Structured unresolved-indirect-call limitations could be missed. | Inspect dossier limitations and retain an explicit missing-evidence signal. |
| Model partial failure explicitly, `src/storage/analysis_repair.rs:398` | A partial capture could be recorded as a completed comparison. | Record an incomplete attempt, preserve its capture, and allow only the remaining bounded retry. Regressions verify retention and attempt limits. |

The automated source inventory is separate from this scoped manual review and
does not establish universal correctness.
