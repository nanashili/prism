# Reference and Prism performance comparison

Investigation completed on 2026-10-19. The final specialist passes used
GPT-6.1 Sol with max reasoning: sidebar rendering, tool transport, native
decompilation, and independent review of the fixes already made.

## What explains the difference

The source comparison identifies avoidable work in three different paths.
It does not establish that Rust itself, the model picker, or SQLite alone
causes the observed delays.

| Interaction | Reference implementation | Prism implementation and consequence |
| --- | --- | --- |
| Opening menus in a long conversation | Near-viewport message mounting and memoized message rows | Every retained message is rendered each frame; a new Markdown cache is created for each render. The menu shares that frame's cost. |
| Opening menus in an empty conversation | Skills loaded through a memoized composer path | The empty-state renderer calls a skill catalog refresh that can synchronously read files after a five-second expiry. This is a separate candidate for short-chat stalls. |
| Picking a model | Model state is retained in the component/session | Prism reconstructs some option data each frame and can synchronously persist a selection. The observed local catalog has only 13 model choices, so its size is weak evidence for major stalls. |
| Tool/control responsiveness | Asynchronous request handling and pending permission promises | A normal MCP call blocks the helper input loop; an ACP permission wait blocks that session's consumer. Status reads cannot bypass a call that the same helper has not finished reading. |
| Large decompilation export | Provider batches of 256, retained result pages of 64, streamed file output | One native mutable Program processes functions serially, with duplicate flow work, inline whole-program checkpointing, and two result-persistence paths. |

Detailed source evidence, line links, caveats, and ownership proposals:

- [Sidebar and dropdown rendering](REFERENCE_SIDEBAR_PERFORMANCE.md).
- [Tool transport, permissions, progress, and queue fairness](REFERENCE_TOOL_LATENCY.md).
- [Native decompilation and persistence critical path](REFERENCE_DECOMP_CRITICAL_PATH.md).

The inspected custom skill directory contained only one 77-byte Markdown file,
and the selected persisted chats were short. These observations weaken an
attribution of the current menu delay solely to skill scanning or large-history
rendering. Both synchronous mechanisms exist, but frame profiling is still
needed to identify the dominant cause in the user's active UI.

These compare the current local working trees. The enclosing repository was at
`85a9be336ecf442cd8a8aa27a8a98060cee0646a`, but both reference app changes and the
untracked Prism tree are outside that commit's reproducible contents. The
reports preserve critical source hashes. They do not establish that the same
Reference source snapshot produced the user's historical one-hour run.

## Fixes included in this pass

### Reuse healthy workers after ordinary errors

The protocol now distinguishes a fully received error reply from a disrupted
exchange. An ordinary function error preserves the reusable worker. Timeout,
cancellation, EOF, malformed transport, and resource-budget interruption retire
it. An unusable native worker does not attempt another checkpoint on shutdown.

A startup or saved-state restore failure pauses a durable job at the current
address instead of repeatedly trying the same broken restore for later entries.
Healthy active batches retain their engine; the 60-second idle release remains.

Owners: `protocol/{rpc,response}.rs`, analysis backend adapters, and
`worker/{analysis,thread,decompilation_run}.rs`.

### Read durable status and results outside the busy analysis worker

Read-only `decompile_batch` list/status/results calls use a short runtime-context
snapshot and their own read-only SQLite connection. They retain project identity
checks, transition rejection, result budgets, and the existing request cap.
Mutations remain serialized. This improves observation of ongoing work, not the
native decompiler's function execution time.

Owners: `worker/runtime.rs`, worker lifecycle publication,
`storage/database.rs`, and bridge request wiring.

### Reserve generated symbol IDs before snapshot restoration

Restore reserves the global generated-function ID range before architecture
decoding generates additional symbols. Normal symbol insertion also advances
its checked counter when preserving an existing generated ID. This prevents a
counter reset from reusing restored identities. It does not repair a snapshot
that already contains duplicate IDs.

Owners: vendored `database/symbol_identity.rs`, compact symbol-ID decoding,
and snapshot restoration. Provenance is recorded in
[the vendor patch record](../vendor/ghidra-decompiler/PRISM_PATCH.md).

### Keep skills on the correct batch path

The canonical and bundled decompile/recompile skills agree. Large decompilation
uses one durable job, resumes it, and retrieves bounded results. It does not
start reconstruction claims or submit source. It uses only indexed functions,
does not repeatedly retry unchanged failures, and keeps exported output modular.

Reference's older skill asks the agent to reconstruct source from assembly and save
analysis notes. Copying that text verbatim would conflict with the requested
separation between decompilation and reconstruction. Skill instructions can
reduce unnecessary tool calls, but cannot remove blocking, checkpoint, or native
execution costs. Custom skills can override bundled names; no current custom
decompile override was found in the inspected support directory.

## Measured operational results

The existing production MCP profiling client was reused against a saved analysis
of Prism's own executable, with private support directories. No new tests,
fixtures, or harnesses were written. No Unity function bodies, disassembly, or
proprietary checkpoint contents were read for this investigation.

### Same 256 indexed functions, before and after

Both runs finished with 256 successful results and zero failures. Every stored
address/status/digest record matched. Each retained one native process, and the
indexed function inventory stayed at 20,104.

| Measurement | Before | After |
| --- | ---: | ---: |
| Median status response | 58.01 ms | 1.45 ms |
| Status p95, nearest rank | 186.41 ms | 9.97 ms |
| Slowest observed status response | 384.96 ms | 23.40 ms |
| Median results-page response | 11.90 ms | 1.27 ms |
| Observed batch completion | 6.75 s | 7.46 s |
| Native peak sampled RSS | 157.05 MiB | 154.67 MiB |
| Native peak sampled physical footprint | 88.42 MiB | 88.33 MiB |

There were 21 status samples before and 30 after, and only three results-page
samples per run. Polling occurred roughly every 250 ms plus request time.
Completion time includes the delay until the completing status was observed.
The profiling sampler and repeated reads were active in both runs.

This single pair supports faster status/results service. It does **not** show
faster decompilation: observed completion was about 10.5% slower after, and the
small memory difference does not establish a RAM reduction. Unequal polling,
function skew, and machine load prevent interpreting this as a precise throughput
regression or an optimization benchmark. It is not a reference comparison run.

### Worker reuse and process failure

An alternating sequence of valid assembly requests and invalid addresses used
three native workers before the change and one after. After the initial open,
valid requests following ordinary errors took about 2–3 ms with reuse, versus
about 247–255 ms before. These are assembly requests, not pseudocode throughput.

Terminating only the isolated profiling child produced an EOF response in
2.7 ms; the following request recovered through a new worker in 197 ms. Keeping
healthy workers therefore did not suppress actual process-failure recovery.

### Restart, saved results, and a harder continuation

After restarting the host, reading completed status plus four saved results
took 10.6 ms without a sampled native worker. This verifies that saved artifacts
do not require restoring the native analysis state.

A new 128-address batch from the next part of the same saved inventory remained
running at the 61.6-second observation boundary: 34 successes and no failures.
Orderly shutdown left 37 committed successes and a paused job. Native sampled
RSS peaked at 500.39 MiB; sampled physical footprint peaked at 367.97 MiB.
This was not a completed batch, nor a before/after comparison. Restore,
decompilation, and checkpoint time were not separated. It demonstrates that
function-dependent work can still be slow and memory-intensive; these fixes do
not establish a small RAM ceiling or fast full-inventory completion.

### Reproduction evidence

Operational records are under `/tmp/prism-worker-reuse-20261019/`:

- `durable-before-summary.json` and `durable-after-summary.json`.
- `durable-restart-summary.json`, corresponding memory JSONL, and vmmap captures.
- `reuse-after-summary.json` and `exit-summary.json`.
- Private SQLite stores retaining committed result identities and inventories.

The existing client is
`/tmp/prism-batch-memory-20261019/profile_batch.py`.
Artifact hashes:

| Artifact | SHA-256 |
| --- | --- |
| Analysed Prism executable | `dad3565ac398b75704b31d651e57e87dbfbda35589eb151b8c5bc3bf32e913fc` |
| Preserved pre-change executable | `7bb55f3b813cada135bf009b24561f508de9bff247bca5c7400ede79109f2eb9` |
| Final measured release executable | `8b24c23b00a541584f86d587a61442985a25d1b24705925edb46be1598dc1bdb` |

Temporary records are local operational evidence, not permanent benchmark
fixtures. RSS and physical footprint are different accounting measures and
should not be substituted for each other or for a complete GUI process total.

## Next implementation order

1. **Bound transcript rendering.** Retain Markdown state per stable message,
   render visible rows with a small overscan, and measure row heights. Invalidate
   geometry on width/font/content changes while preserving scroll anchors. Use
   explicit aggregate byte limits, not a cache for every historical message.
   Move expired skill scans and durable setting writes out of the paint path.
2. **Keep protocol consumers available.** Bound pending ordinary MCP calls and
   correlate replies without blocking stdin. Resolve permission promises while
   continuing session events. Give cancellation/control their own lifetime and
   give durable work a fair share of the worker. Preserve lossless terminal and
   permission events while coalescing replaceable progress.
3. **Remove duplicate persistence.** Write one durable pseudocode artifact and
   let normal interactive reads reuse it under the same identity checks. Batch
   commits by count, bytes, and elapsed time, reporting only committed progress.
   Do not discard resumability to obtain a faster apparent result.
4. **Reduce native repeated work and checkpoint cost.** Measure raw flow,
   full decompiler actions, snapshot capture/XML, compression, and sync separately.
   Preserve string semantics while eliminating redundant flow analysis. Change
   checkpoint scheduling or format with an explicit recovery contract.
5. **Consider concurrency only within the total memory budget.** A thread-count
   environment variable does not parallelize the built-in mutable Program.
   Independent workers would duplicate native state and currently collide on
   the same-image checkpoint lock. Batching and cheaper persistence come first;
   a pool needs explicit snapshot ownership and aggregate peak-RAM admission.

These are remaining implementation work, not changes included in this package.
The source details and constraints for each are in the three specialist reports.
Finishing 200,000 functions in one hour requires about 55.6 successful functions
per second across the actual corpus. No completed run here establishes that rate.

## Verification and delivery

Passed: Rust formatting; strict all-target/all-feature Clippy; the existing
application test suite (one existing bridge test); MCP-only checking; desktop
and MCP-only release builds; first-party source-size validation; and deep,
strict macOS code-signature verification. The independent final review found no
actionable issue in the changed worker reuse, read-only runtime, or symbol
reservation paths. No dependencies were added.

The rebuilt [Prism.app](../build/Prism.app) is 22.07 MiB. The running GUI and the
user's live analysis job were not restarted or modified. The package needs to be
launched later to use these changes. Menu frame latency, full 200k-function
decompilation, reconstruction throughput, and cross-platform behavior remain
unverified in this pass.
