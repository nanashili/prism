# Durable provider batch windows

> Historical implementation report, superseded on 2026-10-20. The
> `reference-ghidra` feature name below became `prism-ghidra-rea`; Rust REA is
> now the default Ghidra provider on macOS/Linux and uses windows of at most
> 64 saved addresses. See
> [current durable-batch behavior](REA_RUST_PROVIDER.md#saved-inventory-and-durable-batches).
> The 256-selector retained-page description belongs to Java REA and is not
> the Rust worker protocol. The former Prism-hosted worker was removed.

Updated 2026-10-19. This report covers the durable-runner integration for the
feature-gated `ghidra-rea` provider. Backend preparation, protocol parsing,
configuration, and UI belong to the Reference provider implementation. No runtime
throughput, provider concurrency, or memory improvement is claimed here.

## Ownership and execution

With `reference-ghidra` enabled and provider `ghidra-rea` selected, `ExportRunner`
owns one volatile `BatchWindow`. Its handle owns at most 256 selectors and a
retained-evidence reference. The runner additionally owns at most 256 frozen
manifest positions and one page of at most 64 results. It consumes the page one
function per worker turn, retaining no whole-job output or inventory copy.

A worker turn performs one bounded phase: begin a provider batch, poll its pending
request, accept one validated page, skip one verified saved success, persist one
result, or release a finished window. The agreed engine API is:

- `begin_decompilation_batch(Vec<String>) -> Result<DecompilationBatch>`
- `poll_decompilation_batch(&mut DecompilationBatch)` returns `Pending`,
  `Page(Vec<DecompilationItem>)`, or `Complete`.
- `abandon_decompilation_batch(DecompilationBatch) -> Result<()>`

The backend submits `batch_decompile` with `detail: "summary"`, validates retained
pages in selector order, adapts page size on resource constraints, and validates
aggregate counts before returning the final page. A pending poll is nonblocking.
The scheduler uses its existing 25 ms receive wait when that poll makes no
progress and no foreground work ran, avoiding a polling spin while retaining
control-lane fairness.

The runner uses only the existing `State.engine`. It does not invoke the lazy
engine opener, analyze, list, or discovery calls. A missing or unusable prepared
session pauses the job with guidance to open the project and explicitly Analyze.
The backend separately enforces its prepared-session requirement. Native `ghidra`
and other existing providers keep their single-function execution path.

## Frozen selection and integrity

The storage query selects only rows in `decompilation_items` at or after the
durable cursor, ordered by frozen manifest position, with SQL `LIMIT 256`. It
always includes the current cursor and otherwise excludes results marked
successful. The selection is never expanded using the provider inventory.

That status filter is an optimization, not an integrity decision. As the cursor
walks the manifest, each saved success still passes the existing digest/decode
check and cursor validation inside the cursor transaction. If an excluded success
is corrupt or missing its artifact, the runner abandons the volatile window and
starts a new window that includes that current address. Already committed results
remain intact; uncommitted provider results may be recomputed. No cursor skips an
unverified artifact, and normal saved successes are not decoded again merely to
collect a window.

Each returned item must match both the next selected manifest position and its
current address. Successful text and per-function errors go through the existing
`record_result` size checks, bounded error conversion, quota accounting, and atomic
artifact/result-counter/cursor transaction. The runner does not change the
database format or identity calculation. The provider's separate configuration
work adds its backend identity to the prepared context; the runner compares that
exact context before every step.

## Pause, cancellation, and cleanup

Pause, cancel, global Stop, context change, project switch, shutdown, provider
failure, and completion release an owned active window through the backend's
explicit abandon operation before dropping the runner. A pending call transfers
to the backend's abandoned-call slot, where its correlated response can be drained
without reopening or discovering functions. While that call still computes,
resume or ordinary provider work may report busy; the durable cursor remains at
its last committed artifact.

Page-only cleanup releases the active handle immediately. If the entire engine
has already been dropped, its provider connection was closed, so dropping the
remaining handle is sufficient. Runtime active-job cleanup is attempted even if
batch release fails; both failures are retained in the returned error. A durable
pause-write failure leaves the owned runner in place on explicit pause, rather
than discarding its recovery state.

The legacy no-action `decompile_batch` selection path returns actionable
`action=start` guidance for `ghidra-rea`. It does not run its old four single
function calls for this provider. Other providers retain the existing behavior.

## Changed files

- `src/worker/decompilation_run.rs`: provider dispatch, progress result, and error
  recovery through owned runner release.
- `src/worker/decompilation_run/reference.rs`: stepwise begin/poll/record/release flow.
- `src/worker/durable_decompilation.rs`: lifecycle release and independent runtime
  cleanup after successful control/pause persistence.
- `src/worker/decompile_batch.rs`: actionable durable-only guidance for Reference.
- `src/decompilation/job.rs`, `mod.rs`, and `window.rs`: feature-gated window
  ownership and bounded page/manifest position state.
- `src/storage/decompilation/inventory.rs` and `inventory/window.rs`: bounded
  frozen-manifest candidate query.
- `src/worker/scheduling.rs`: coordinated Pending backoff, implemented by the MCP
  responsiveness owner.

No dependencies, tests, fixtures, or harnesses were added. No live job, GUI,
proprietary target, release build, staging, or commit was used for this work.
Existing per-function limits and transaction durability remain unchanged. An
extreme full page can still contain up to 64 bounded function texts; actual
resident memory and provider behavior require coordinator measurement.

## Verification

The complete modified runner, storage query, lifecycle methods, and immediate
dependencies were reread after formatting, including partial-failure ownership
and cursor behavior. Checks actually executed for this slice:

- Scoped `rustfmt --edition 2024 --check` passed for the runner, window, inventory,
  and legacy batch source files after the final review.
- `cargo check --locked --offline --bin prism` passed at the earlier integration
  checkpoint, with a temporary unused backend-method warning while provider work
  was still in progress.
- `cargo clippy --locked --offline --features reference-ghidra --bin prism --bin
  prism-mcp -- -D warnings` was attempted. At that checkpoint it failed on the
  concurrently incomplete composer-cache fields/imports and unused backend
  identity export. Those owners subsequently completed their wiring. This report
  does not claim that attempt passed.
- A physical-line scan of all 415 first-party Rust source files under `src` passed
  the 250-line ordinary, 100-line root, and 120-line main limits after the final
  review.

The coordinator is running the final shared build, formatting, and lint gates
centrally. No repository `tools/check_architecture.py` or
`tools/check_source_sizes.py` exists in this Prism checkout. No tests were written
or run. Runtime page retention, pause/resume, and the 2,251-address provider run
remain coordinator validation; they are not claimed as exercised by this
implementation task.
