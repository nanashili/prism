# Reference Ghidra backend integration

> Historical implementation report with current setup below. The original
> `reference-ghidra` feature name and Prism-hosted Rust route were superseded on
> 2026-10-20. Retained hashes and measurements identify the earlier snapshot;
> they do not establish performance for changed sources or optional Rust REA.
> Everything from **Historical report** onward, including its obsolete build
> commands, describes that earlier snapshot.

## Current provider selection (2026-10-20)

Default desktop builds enable Java Ghidra through REA with `prism-ghidra-rea`
and select `ghidra-rea`. Rust Ghidra through REA is experimental and requires
the optional `rea-rust-ghidra` feature plus explicit `ghidra-rust-rea` selection.
Build both routes for comparison with
`cargo build --release --locked --features rea-rust-ghidra --bin prism`.

To build only the Java provider, use
`cargo build --release --locked --no-default-features --features desktop,prism-ghidra-rea --bin prism`,
then configure REA, Node, Java and Ghidra in Settings > Engines. Prism forwards canonical
`GHIDRA_INSTALL_DIR` and `JAVA_HOME` to REA's Java provider.

The default macOS bundle uses Java Ghidra through REA and relies on external
runtime installations. Rust packaging requires `--rea-rust-ghidra` and an
explicit REA package, portable Node runtime, Rust worker and worker license notices; see
[Rust provider setup](REA_RUST_PROVIDER.md#packaged-setup-and-path-resolution).
Rust and Java retain distinct provider and saved-job identities. The recorded
Rust benchmarks do not justify making it the default.

Both providers require explicit project preparation and use durable batch jobs
over Prism's saved inventory. The Rust decompilation path accepts only saved
addresses and never discovers functions while decompiling. Provider selection
does not authorize cross-provider reuse of saved pseudocode. Rust target and
operation limits are listed in [the provider guide](REA_RUST_PROVIDER.md).

## Historical report (2026-10-19)

Verified against the local sources and installed REA on 2026-10-19. The user
approved using Reference's backend behind a feature flag. `reference-ghidra` adds the
explicit `ghidra-rea` choice, displayed as **Ghidra (Java through REA)**. The existing
`ghidra` choice remains the default Rust backend with its existing identity.

The new path invokes REA's actual Ghidra provider and native decompiler pool.
Changing the Rust provider's loop size would not invoke that implementation.
This integration matches the working provider batch and retained-page path;
it does not establish identical output or performance across different programs,
analysis profiles, Ghidra installations, or inventories.

## Scope and provenance

The enclosing repository HEAD is
`85a9be336ecf442cd8a8aa27a8a98060cee0646a`. The app has local changes, and the
Prism tree is untracked in that repository. That commit alone does not reproduce
the inspected or implemented workspace. The hashes below identify the source
files actually inspected. No installed REA files were modified.

Reference app root: `/path/to/reference-checkout/app`.

| File relative to app | SHA-256 |
| --- | --- |
| `src/engine/rea.ts` | `f13b7945793e31ac89bc1106fbfac3d4f641d6e3ad5d24e723a74b4360f94b3d` |
| `src/engine/reaBatch.ts` | `acd5fa15d440fcada32a38d985d4b168052faee803b4ddaf9afea31903948028` |
| `src/engine/toolchain.ts` | `12745e04820d9b1aa3801d76d0317417734f960a44af90b94f3a7779ea002345` |
| `src/engine/analysisResources.ts` | `088fd193d893cbd26674cf39caff5bb1454f4406cb1e764f7e10c6d1d8b1c428` |
| `src/engine/reaTimeouts.ts` | `07af9d15ea9c436f6868c0d445e0d3faa36d070124e27699abf595a834ddb9e3` |
| `src/workbench/export.ts` | `ff31e27777eefa31ada6ff786d60f51fd4e24cb610943ba31112ec9de14b8591` |

Installed REA root: `/Applications/Reference.app/Contents/Resources/rea`.
The installed launcher reports REA `6.5.0` when invoked with Reference's bundled
Node and `--version`. The selected Ghidra installation's
`Ghidra/application.properties` reports `12.1.4`.

| File relative to installed REA | SHA-256 |
| --- | --- |
| `scripts/rea.mjs` | `526105418604cc6e5a202a6275b700814e27450bfaffd45e7510530ba669de8d` |
| `package.json` | `7b273825071813af72fb5cda4c0dbf169e16ca4ebf3fd20226d11fe146d1262c` |
| `dist/application/native/BatchDecompilation.js` | `a8e9d8302b18c8c7f958678689d848dc196399e95a3414777c9dcd624760a94f` |
| `dist/ghidra/GhidraAnalysisCache.js` | `1073595e776c3ba067d25337f11814e5f3ac314de92ec0f3127412f182a98a7a` |
| `dist/ghidra/GhidraHeadlessArguments.js` | `e5f2ddca9b9827313c6b8f6b93bd3f90a4e1f173404bfbaadb2309c29bca56ec` |
| `dist/ghidra/GhidraClient.js` | `ddf0b8554207823b4293c0153dbf5cf7c47346dafefd4ffa90cd0e703eb2d044` |
| `dist/ghidra/GhidraProviderClient.js` | `1f8ce0fb658e4422db6d70a17b7ad2eb58c6268168933ef6649642e6905aea72` |
| `dist/ghidra/GhidraSessionError.js` | `f4eee7f1429535a0507542761f68a42396e4f0bbbf60796e12827184c641f802` |
| `bridge/ghidra/ReaGhidraDecompilation.java` | `8f2051828ff0fe9fada1543cfd7f3b7450da5723ed03baa74824d198a991b763` |
| `bridge/ghidra/ReaGhidraBridge.java` | `ab9fcb11e77c10a5955026a75b0a6c2e9aede6f9a842ea19d6630581d560d15a` |
| `bridge/ghidra/ReaGhidraAnalysisOptions.java` | `14200fb685d88c91b38f955ad360fe2e11b9a83a53cc69f87fd229c2e784e936` |

TypeScript contract references below are in
`/path/to/reference-checkout/rea/src`. The installed compiled and Java
files listed above anchor the runtime evidence. A version string alone does not
prove that every local checkout or PATH installation has identical contents.
The actual executable used by an already-running Reference process was not probed.

## Verified Reference path and limits

- `app/src/engine/rea.ts:76` launches a selected JS/MJS REA through Node;
  `:99` supplies provider and Ghidra configuration.
- `rea.ts:414` defaults to 256 selected addresses, requests a summary, then
  reads retained `decompilations` pages of 64 in selected order. `:471` recovers
  a retained summary when inline delivery exceeds a response budget.
- `app/src/engine/reaBatch.ts` validates ordering and page coverage.
- `app/src/workbench/export.ts:160` writes results incrementally through a file
  handle. It retains partial output on failure and publishes the completed file
  through the existing rename path.
- REA `application/native/BatchDecompilation.ts` uses explicit selectors and
  internal native batches of 64. Prism never supplies a discovery pattern.
- `bridge/ghidra/ReaGhidraDecompilation.java:18` owns a native decompiler pool
  for one immutable Program revision. It configures C code on, syntax trees off,
  jump loads on, maximum instructions at `Integer.MAX_VALUE`, a 30-second
  per-function timeout, and a 32-Mi-character source cache. Worker count is
  bounded to 1 through 8 and available processors.

Prism retains at most 256 pending manifest positions, one page of at most 64
results, and one correlated RPC response. It does not accumulate the whole job's
pseudocode. REA still retains the current batch, the Java Program, its source
cache, and native decompiler processes. The heap setting is not a total process
tree RAM limit. A 1-worker profile trades throughput for lower memory use.

## Feature, selection, launch and identity

`Cargo.toml` declares `reference-ghidra = ["desktop"]`; it adds no dependencies.
Build the optional desktop backend explicitly:

```sh
cargo build --offline --features reference-ghidra --bin prism
```

For a release macOS bundle, use `python3 scripts/bundle-macos.py --reference-ghidra`.
This enables the Cargo feature without bundling the external REA, Java, or Ghidra
installations. The option is off by default. `--skip-build` packages the existing
executables and does not change their compiled features.

Select **Settings > Engines > Ghidra (Java through REA)**. Set REA to the intended
executable or `.mjs` launcher, and explicitly select the Ghidra installation and
Java home. **Use Reference's copy** only copies the discovered REA path after a user
click. It does not silently replace a configured executable or choose Ghidra.
An old saved `ghidra` provider is not migrated. A build without the feature gives
an actionable error for saved `ghidra-rea` settings instead of falling back.

`analysis/rea/launch.rs` resolves the configured REA path canonically; a bare
command uses the existing executable search. JS/MJS launchers use an explicit
`tools.rea_node`, an adjacent bundled `node/bin/node`, or the resolved PATH Node,
in that order. Ghidra and Java require explicit configured directories containing
`Ghidra/application.properties` and `bin/java` (`bin/java.exe` on Windows).
Their effective canonical paths are passed to REA, so auto-detection cannot
silently substitute a different installation.

`analysis/rea/identity.rs` contributes `prism-rea-ghidra-v1` to this provider's
durable configuration. It records canonical paths, file sizes, modification
timestamps, digests of selected REA implementation files and sorted bridge Java
sources, Ghidra properties and JDK release metadata, Node/Java executable
metadata, analyzer choices, heap, workers, time limits, and optional language and
compiler-spec environment selections. It is a selected-component fingerprint,
not a complete digest of every installed dependency or executable. Runtime
context is prepared once rather than rehashing these sources on each worker turn.
The existing native configuration is unchanged.

Artifacts from different provider/configuration identities are not treated as
interchangeable successes. Existing durable jobs and artifacts remain stored;
resume requires their matching project, provider and configuration. Switching
backend does not convert old native pseudocode into Reference output.

## Explicit preparation and session lifecycle

REA `contracts/sessionLifecycleInputs.ts:33` accepts this strict open shape:

```ts
{
  path: AbsolutePath,
  provider_id?: ProviderId,
  snapshot_path?: AbsolutePath,
  format?: ExecutableFormatHint,
  architecture?: BinaryArchitecture,
  entry_point?: HexAddress
}
```

There is no stock `cache_required` option. `open_binary` is lazy in this runtime.
Prism's explicit project open calls `open_binary({path, provider_id:"ghidra"})`
and then `list_segments({})` before recording a prepared target. In REA's
`application/binary/BinarySessionExecution.ts:55`, Ghidra snapshot replay requires
native source admission; `BinarySession.ts:747` marks admission after a successful
Ghidra operation. Thus the explicit preparation call reaches the Ghidra session
even when an application inventory has already been saved.

Opening/reopening a project or explicitly choosing Analyze may restore or run
analysis. Bulk decompilation, interactive reads, reconstruction and raw agent
tools cannot lazily open a missing Reference session. A missing or invalid session
requires explicit preparation. Saved durable results remain readable after a
restart; new computation requires reopening/preparing the project.

One healthy prepared Reference session is retained until project close, backend
change, or shutdown. The native backend retains its existing 60-second idle
policy. A prepared session is marked only after successful preparation.

`analysis/rea/reply.rs` preserves completed domain errors and response-budget
errors. A correlated tool error invalidates the session when REA supplies
`details.diagnostics.failure_kind` equal to `cancelled`, `timeout`, `start`,
`process`, `protocol`, or `analysis_timeout`, or reports `cleanup_incomplete`.
Remote function errors, including a per-function decompiler timeout, do not
imply session loss. Transport, malformed response, ordering or coverage failures
also invalidate the batch/session.

This distinction follows installed `GhidraClient.js:107`: a successful start
promise stays retained after an unexpected child/socket failure; ordinary reads
do not create a replacement Program. Active request cancellation closes the
child and can clear that promise (`:323`), so Prism must invalidate the session
before another analysis call. Abandoned replies are correlated and parsed before
any later operation is admitted. Additional calls receive a pending/busy error
while the earlier response is outstanding. They do not destroy a healthy session.

## Exact provider batch exchange

Transport is stdio JSON-RPC MCP, initialized with `protocolVersion:"2024-11-05"`,
then `notifications/initialized`. Calls use:

```ts
{jsonrpc:"2.0", id: RpcId, method:"tools/call",
 params:{name: ToolName, arguments: Arguments}}
```

REA can return `structuredContent` or JSON in text content blocks. Tool errors
set `isError:true` and carry `error`; successful wrappers may carry `result`.
An analysis envelope can place the projected view in `normalized_result`.

The submitted arguments are bounded, validated numeric addresses already in the
saved inventory. No new functions or pattern selectors are requested:

```ts
// contracts/enhancedInputs.ts:19; optional pattern is intentionally unused.
batch_decompile({addresses: SavedAddress[1..256], detail:"summary"})

// domain/analysisView/batchDecompilationView.ts:17
{
  kind:"summary", parent_operation:"batch_decompile",
  parent_evidence_id: EvidenceId,
  summary:{total:Integer, succeeded:Integer, failed:Integer},
  coverage: Coverage, artifact: ArtifactIdentity,
  limitations: unknown[], unknowns: unknown[]
}

inspect_analysis_view({
  source:{kind:"retained-evidence", evidence_id:EvidenceId},
  view:{kind:"page", collection:"decompilations", offset:Integer, limit:1..64}
})

{
  kind:"page", parent_operation:"batch_decompile",
  parent_evidence_id:EvidenceId, items: DecompilationItem[],
  coverage:{status:CoverageStatus, examined:Integer, total:Integer,
            next_offset:Integer|null, exhausted:boolean},
  artifact:ArtifactIdentity, limitations:unknown[], unknowns:unknown[]
}
```

These are the required fields used by the adapter, not a claim that REA omits
other envelope fields. Item variants from `domain/native/batchDecompilation.ts:21`:

```ts
type Resolved = {status:"resolved", address:Address,
                 name:string|null, name_error?:ErrorProjection};
type Unknown = {status:"unknown", error:ErrorProjection};
type DecompilationItem =
  | {address:SubmittedSelector, procedure:Resolved,
     status:"ok", pseudocode:NonEmptyString}
  | {address:SubmittedSelector, procedure:Resolved|Unknown,
     status:"error", error:ErrorProjection};

type ErrorProjection = {
  code:ErrorCode, category:ErrorCategory, message:string,
  retryable:boolean, remediation:{action:string},
  details?:Record<string, JsonValue>
};
```

The full retained batch has `{items,total,succeeded,failed}`. Top-level errors
retain the complete JSON projection in Prism's error. Per-function durable
failures currently retain the message, bounded to 4096 characters. They do not
persist every upstream diagnostic field. A successful item over 512 KiB becomes
an explicit Prism artifact-limit failure.

A `resource_constraint` can carry
`details.evidence_reference:{kind:"retained-evidence",evidence_id}` or the same
reference under `details.reported_limits`. Prism retrieves it through
`inspect_analysis_view({source,view:{kind:"summary"}})` rather than rerunning the
batch. Oversized pages retry the same offset at half the page limit down to one.
Failure at one item is reported, with no truncated success or alternate algorithm.

The adapter verifies summary totals; evidence id and operation; each original
selector in frozen order; resolved successful entry address; page size, total,
examined count, exhaustion and next offset; and final succeeded/failed totals.
Final totals are checked before yielding the final page because the durable
runner may commit its last result without another provider poll. REA can resolve
an address to a containing function or file offset; Prism rejects a successful
resolution whose canonical entry differs from the saved selected entry.

## Worker API and durable persistence

`analysis::Engine` exposes the feature-gated API implemented in
`analysis/rea/batch*.rs` and `analysis/engine/batch.rs`:

```rust
begin_decompilation_batch(Vec<String>) -> Result<DecompilationBatch>
poll_decompilation_batch(&mut DecompilationBatch)
    -> Result<DecompilationBatchPoll>
abandon_decompilation_batch(DecompilationBatch) -> Result<()>
// Poll: Pending | Page(Vec<DecompilationItem>) | Complete
// Item: { address: String, result: Result<String, String> }
```

The opaque handle belongs to one engine session. Polling consumes at most 32
available protocol messages without blocking; normal worker scheduling waits
between Pending turns. Pause, cancel, failure, completion and project transitions
explicitly abandon the handle. If its RPC is outstanding, the engine retains
only the correlation state and drains it before accepting another call.

`worker/decompilation_run/reference.rs` uses the already prepared `State.engine`
directly. It requests up to 256 missing manifest addresses, fetches one page,
and records one item per turn through the existing atomic durable artifact and
cursor update. Verified already-successful artifacts are skipped. Dropping a
volatile batch cannot advance the persisted cursor or erase committed successes.
Resume reconstructs a missing-results batch from that cursor and frozen order.

Raw `batch_decompile` is unavailable through the general REA tool adapter.
Native bulk operations use Prism `decompile_batch` with `action:"start"`.
Uncached reconstruction cannot substitute single-function provider calls for
the batch path. Interactive one-function display remains supported after
saved-inventory validation. Existing native behavior stays in its provider path.

## Environment and analyzer profile

For `ghidra-rea`, launch supplies these effective values:

| Variable | Value |
| --- | --- |
| `REA_ANALYSIS_PROVIDER` | `ghidra` |
| `GHIDRA_INSTALL_DIR` | Explicit canonical Ghidra directory |
| `JAVA_HOME` | Explicit canonical Java home |
| `GHIDRA_MAXMEM`, `GHIDRA_HEADLESS_MAXMEM` | `analysis_memory_mib` followed by `M` |
| `REA_GHIDRA_DECOMPILER_THREADS` | `tools.ghidra_threads`, default 1, validated 1..8 |
| `REA_GHIDRA_CACHE_DIR` | Prism support directory plus `ghidra` |
| `REA_GHIDRA_STARTUP_TIMEOUT_MS` | Preparation choice in milliseconds |
| `REA_GHIDRA_DISABLED_ANALYZERS` | Comma-separated profile below |
| `REA_MCP_MAX_RESPONSE_BYTES` | `10485760` |

Preparation choices are 30 minutes, 1/2/4/6/8/12 hours. Open/request allowance is
at least preparation time plus 60 seconds. A smaller ordinary request setting
does not reduce that floor. REA's response-budget validator has a 10-MiB minimum
(`config/mcpResponseBudget.ts:9`), so this provider gets a narrow 10-MiB receive
framing mode. Other JSON/DAP receive limits remain 8 MiB; outgoing requests stay
8 MiB. Adaptive retained pages keep the current reply bounded by that cap.

False `find_parameters`, `rebuild_switches`, and `hidden_code` disable
`Decompiler Parameter ID`, `Decompiler Switch Analysis`, and
`Aggressive Instruction Finder`. Fast mode also disables
`Basic Constant Reference Analyzer`. False `swift_names` disables
`Demangler Swift`.

The app's fast list at `toolchain.ts:136` also contains
`Objective-C Message Analyzer`. Installed
`ReaGhidraAnalysisOptions.java:16` rejects unknown analyzer names for the current
Program. That entry failed on the independently built Rust Mach-O used for
validation, so Prism omits it. Remaining configured names may also be unavailable
for another Program/Ghidra combination; preparation then fails explicitly.
There is no automatic retry with a changed analysis profile. This is a documented
profile difference, not byte-identical analysis parity.

## Isolated operating example and observed run

An example macOS profile uses existing installations explicitly. These paths
are examples from this machine, not hardcoded discovery defaults:

```json
{
  "provider": "ghidra-rea",
  "rea": "/Applications/Reference.app/Contents/Resources/rea/scripts/rea.mjs",
  "analysis_memory_mib": 1024,
  "timeout_seconds": 300,
  "tools": {
    "rea_node": "/Applications/Reference.app/Contents/Resources/node/bin/node",
    "ghidra_dir": "/path/to/reference-profile/engines/ghidra_12.1.4_PUBLIC",
    "java_home": "/opt/homebrew/Cellar/openjdk/25.0.2/libexec/openjdk.jdk/Contents/Home",
    "ghidra_threads": 1,
    "ghidra_analysis_time": "30 minutes"
  }
}
```

For a fresh isolated support directory containing `settings.json`, the host is
`target/debug/prism --headless <absolute-own-prism-mcp-artifact>`, launched with
`PRISM_SUPPORT` set to that directory. Unset inherited
`REA_GHIDRA_CACHE_FALLBACK_DIRS` when isolation must exclude other applications'
caches. The `prism-mcp` executable normally attaches to a running Prism bridge;
in this validation its independently built executable is also the analysis target.
No GUI is needed.

The coordinated observed run used a debug feature build copied to
`/tmp/prism-responsiveness-20261019/reference-feature/reference-ready-exec`, with profile
`/tmp/prism-responsiveness-20261019/reference-feature/support/settings.json`.
Its aggregate results are in that directory's `report.json`, logs in
`reference-ready-stderr.log`, and sampled memory in `reference-ready-memory.jsonl`.
The profile used 1024 MiB and one worker; Node resolved to Reference's adjacent copy.
Preparation took 17.182 seconds and indexed 1172 functions. The selected 320
functions completed with 320 successes and zero failures in 7.129 seconds after
preparation, using two provider batches (256+64) and five result pages. The
maximum individual child RSS was 833344 KiB (Java). The whole sampled process
tree reached 1540480 KiB RSS, approximately 1.47 GiB, while retained. The aggregate
physical-footprint peak was 1445463456 bytes during decompilation. Node reached
538528 KiB RSS and a native decompiler child reached 128672 KiB. A 1024-MiB Java
heap setting therefore does not imply a 1024-MiB total memory ceiling. These tree
figures come from the coordinated aggregate of `reference-ready-memory.jsonl`;
`report.json`'s category maxima alone do not calculate the tree sum. Last status
latency was 4.015 ms.

This establishes an observed functioning route for that own artifact and that
profile. It does not compare Reference's wall-clock runtime or output, establish
behavioral correctness, validate large proprietary programs, or establish a
total-RAM bound. The run predates the final canonical-path/identity tightening;
it already used explicit matching paths, but its recorded identity differs from
the final adapter identity. Final coordinator validation should use the current
source and a matching fresh job configuration.

## Cache-required restoration: deliberately unsupported today

An existing Reference `.gzf` cannot safely be declared reusable from a filename or
settings label. REA `GhidraAnalysisCache.ts:91` hashes schema 2, target SHA-256,
provider version, profile digest, patch digests and bridge Java source contents.
`GhidraSnapshotManifest.ts` describes the committed archive identity, size,
digest, representation and compression. The runtime must verify the artifact.

More importantly, `GhidraAnalysisCache.ts:166` falls back to normal analysis
when cache preparation fails, and `GhidraHeadlessArguments.ts:173` requires
analysis when no valid archive is selected. A Prism-side preflight cannot close
that race or prohibit fallback. `-noanalysis` on the headless command does not
prove cache-only behavior because the bridge owns the analysis plan.

The current integration therefore uses its own explicit preparation phase and
live prepared session. It neither imports Reference archives during decompilation
nor claims cache-only restart guarantees. No proprietary archives were read for
this investigation.

A future strict restoration capability requires an upstream contract such as
`open_binary({...,analysis_policy:"cache-required"})`, advertised by the selected
REA runtime. That policy must propagate through session routing, launcher,
cache validation and analysis-plan construction; reject missing, incompatible,
corrupt, disabled or unsupported caches before import; and prevent the existing
catch-and-fallback branch. An explicit external cache root could then be used
after exact identity matching and provider-owned archive verification. This
capability is proposed here, not implemented or advertised by stock REA 6.5.0.

## Verification status

The integrated source passed default and feature-enabled compilation, strict
Clippy across all targets/features, formatting, the existing test, and the
MCP-only check/build. The feature-enabled release was packaged into the 22.15 MiB
macOS bundle and passed deep, strict signature verification. The independent
adapter review found no remaining actionable issue after correcting abandoned
ordinary-error handling during resume.

The final release also completed a 512-function durable job across shutdown
and reopening: 65 committed successes survived the restart, all 512 finished
successfully, and sampled pre-restart artifacts remained unchanged. Exact
timings, release hash, experimental Node options and measurement limitations
are in [the integrated report](RESPONSIVENESS_AND_BATCH_MEASUREMENTS.md).
No new tests, fixtures, harnesses or dependencies were authored. Native GUI
interaction and large proprietary binaries were not exercised.
