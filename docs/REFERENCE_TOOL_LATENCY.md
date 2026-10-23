# Reference and Prism tool latency

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Prism can expose a durable job identifier, show window progress, and answer
saved-state queries while native analysis continues. A completed uncached
native result still requires the native operation to finish. The useful
latency improvements are removing unrelated queue waits, keeping control
messages responsive, and reusing valid analysis state within a bounded memory
budget.

The current durable `decompile_batch` read path already removes one important
wait: `list`, `status`, and `results` read saved state outside the analysis
worker. It improves observability and interaction during decompilation; it
does not make the native decompiler execute functions faster.

## Source snapshot and evidence

This audit uses the local working trees on 2026-10-19:

- Prism: `/path/to/prism`.
- Reference: `/path/to/reference-checkout/app`.
- Parent repository `HEAD`: `85a9be336ecf442cd8a8aa27a8a98060cee0646a`.

`prism/` is untracked in the parent repository. The reference `app/` has modified
agent, bridge, event-buffer, and engine sources; its `sharedRea.ts`,
`reaBroker.ts`, `socketTransport.ts`, and `bridgeClient.ts` are untracked.
Consequently, references to Reference below describe the current local reference
implementation, not a verified pristine checkout of that commit. Line numbers
refer to this inspected working tree.

The findings distinguish confirmed source mechanisms from inferred cost. This
audit did not measure elapsed time, throughput, or resident memory and did not
run native analysis. Numeric capacities and deadlines below are source
constants, not benchmark results. No tests, fixtures, or production code were
added as part of this audit.

## What instant interaction means

Four different intervals matter:

| Interval | Completion condition | Appropriate behavior |
| --- | --- | --- |
| Acknowledgement | The request is accepted or rejected | Show queued or busy feedback immediately after admission. Do not report success before admission. |
| Saved-state response | A bounded database or file read finishes | Answer without waiting for unrelated native work, subject to project identity and transition checks. |
| First useful progress | A real analysis milestone or partial result arrives | Deliver correlated progress while the operation remains active. |
| Completed native result | The requested analysis and required persistence finish | Return the actual result or an explicit failure. An acknowledgement is not this result. |

Durable start already returns an identifier and status after registering the
job, while execution advances separately
([durable_decompilation.rs:17-33](../src/worker/durable_decompilation.rs#L17)).
A UI navigation tool is a separate case: it crosses the UI event queue and
replies when `navigate` runs on the UI thread
([request.rs:90-100](../src/bridge/request.rs#L90),
[receive.rs:215-223](../src/ui/receive.rs#L215)).

## Queue and lifetime topology

The normal Prism analysis route is:

```text
agent MCP client
  -> MCP helper stdin reader, normal calls handled synchronously
  -> a new loopback TCP connection for each call
  -> one bounded bridge request thread
  -> analysis Job queue with capacity 8
  -> one State owning Store, project, and optional live Engine
  -> reusable native or REA process
  -> one bridge reply
  -> helper JSON result and stdout
```

The bridge has three alternative routes. Window tools send a UI
`Navigate` event; questions send a UI question and retain an answer channel.
Durable `list`, `status`, and `results` instead open a read-only Store using a
short runtime-context snapshot, before constructing an analysis job
([request.rs:68-100](../src/bridge/request.rs#L68),
[request.rs:103-136](../src/bridge/request.rs#L103)).

| Boundary | Current bound and ownership | Consequence |
| --- | --- | --- |
| MCP helper | Normal calls run on its stdin loop; only questions use pending request threads, at most 8 | One normal call prevents this helper from reading another request or cancellation. |
| Bridge admission | At most 16 request threads, each configured with a 512 KiB stack | Waiting questions and ordinary calls share admission with saved-state queries. These stack settings describe reservations, not measured resident memory. |
| Analysis | `sync_channel(8)`, one `State`, one optional `Engine` | Mutable native state has one owner. Queued storage reads and controls can still wait behind native work. |
| Native RPC input | `sync_channel(16)` between stdout reader and consumer | A stalled consumer eventually applies backpressure to the child process. |
| UI events | `sync_channel(32)` shared by agent, analysis, and navigation events | Bounded memory, but lossless producer sends block when UI consumption falls behind. |
| Agent commands | Prompt/preparation queue 2, control queue 8, at most 8 outstanding controls | Bounds are explicit, but queues do not wake every wait and cannot bypass a blocked permission handler. |

Sources: [session.rs:28-50](../src/mcp/session.rs#L28),
[pending.rs:41-72](../src/mcp/session/pending.rs#L41),
[listener.rs:16-55](../src/bridge/listener.rs#L16),
[thread.rs:20-42](../src/worker/thread.rs#L20),
[rpc.rs:35-47](../src/protocol/rpc.rs#L35),
[events.rs:99-124](../src/events.rs#L99),
[background.rs:39-42](../src/agent/background.rs#L39),
[control.rs:29-56](../src/agent/session/control.rs#L29).

The reference analysis route uses one application-owned `ReaBroker`, a
persistent `SocketTransport` per consumer, and a live worker shared by consumers
selecting the same target. Its broker also serializes tool calls globally
through one `SerialQueue`. This is a difference in dispatch and lifetime, not
evidence that the reference performs native analysis concurrently
(sharedRea.ts:13-30,
socketTransport.ts:20-37,
reaBroker.ts:74-79,
reaBroker.ts:223-244,
reaBroker.ts:263-273).

## Confirmed blocking and fairness findings

### Normal MCP calls stop the helper from reading control messages

`Session::run` reads a request, calls `respond`, and writes its response before
reading again. `respond` calls the endpoint synchronously. Only the question
tool is diverted into the bounded pending-request set; its cancellation map
does not contain ordinary tool calls
([session.rs:34-48](../src/mcp/session.rs#L34),
[session.rs:113-120](../src/mcp/session.rs#L113),
[pending.rs:41-55](../src/mcp/session/pending.rs#L41)).

Therefore a slow ordinary call blocks `ping`, `tools/list`, another tool call,
and cancellation notifications on the same helper. The read-only durable path
can answer through another helper or connection, but cannot overtake an
earlier synchronous call before that helper reads it. The ordinary endpoint
also uses a fresh false cancellation flag and a blocking reply read; the
question alone receives the cancellable read
([endpoint.rs:75-112](../src/mcp/endpoint.rs#L75)).

The bounded fix is to keep reading protocol messages while a small, explicitly
limited set of requests is pending, correlate each response with its request
ID, and propagate per-request cancellation. Keep mutable engine execution
serial. Multiplexing protocol requests does not require multiple engines.

### A pending ACP permission blocks all updates on that session

The session message pump dispatches `session/request_permission` synchronously.
Without an automatic policy answer, it waits inside `ask` for the user. Until
that handler returns, the same session loop cannot process later child/root
updates, send queued controls, or send its ordinary turn-cancellation message
([turn.rs:32-54](../src/agent/session/turn.rs#L32),
[turn.rs:91-109](../src/agent/session/turn.rs#L91),
[permission.rs:81-91](../src/agent/permission.rs#L81),
[permission.rs:107-124](../src/agent/permission.rs#L107)).

`ask` checks the atomic cancellation flag between waits, so an already-admitted
permission can resolve as cancelled. That does not preserve normal event and
control service during the human wait. If its initial UI send blocks, even
the cancellation check has not started. The separate stdout reader can buffer
16 messages and then blocks on its own send
([rpc.rs:35-47](../src/protocol/rpc.rs#L35)).

The reference keeps permission resolvers in a map and awaits a Promise in the
individual request handler. Its installed ACP SDK wire-reading loop dispatches
incoming messages without awaiting each handler Promise. Thus waiting for one
permission does not inherently stop that connection from receiving another
message
(session.ts:151-178,
session.ts:222-226,
sessionProcess.ts:49-56,
jsonrpc.js:631-639,
jsonrpc.js:724-738).

Use a bounded set of pending permissions keyed by RPC request ID. Poll answers
alongside incoming messages and controls; send the matching RPC response when
the answer arrives. Preserve per-request approval semantics and order, without
parking the entire session or spawning another agent process.

### UI event backpressure can stall protocol and analysis consumers

`Sink::send` uses blocking `SyncSender::send`, including for every root and
child text/thought/tool chunk. It requests repaint before and after the send.
The UI drains at most 64 events per update, and navigation executes on that
same receiving thread
([events.rs:99-124](../src/events.rs#L99),
[updates.rs:54-90](../src/agent/updates.rs#L54),
[receive.rs:7-22](../src/ui/receive.rs#L7),
[receive.rs:215-223](../src/ui/receive.rs#L215)).

A burst faster than UI consumption can fill the shared 32-event channel and
block an ACP or native progress consumer. Native analysis progress uses this
sink inside the RPC callback, so the consumer can stop reading the eventual
result while waiting to deliver UI progress
(client.rs:84-92).
This mechanism is confirmed; its frequency and elapsed cost are unmeasured.

The reference merges adjacent compatible root/subagent chunks and dispatches
them in one batch. It flushes nonstreaming events immediately; streaming events
have a 50 ms timer or a 32-entry flush threshold. This intentionally trades a
small streaming delay for fewer reducer updates and preserves permission and
completion ordering
(useEventBuffer.ts:12-40,
useEventBuffer.ts:54-82).

Retain bounded backpressure, but coalesce compatible chunks before delivery,
keep only the latest replaceable progress update, and impose byte limits as
well as event counts. Permissions, tool lifecycle changes, errors, and final
completion require ordering barriers. Reserve service for cancellation/control
without moving completion ahead of undelivered text. An unbounded event queue
would exchange a responsiveness problem for a memory problem.

### Analysis queue policy can starve durable decompilation and delay controls

The worker always takes any queued job and continues the loop before advancing
the durable runner. Only an empty queue permits the next durable function.
Sustained arrivals can therefore starve durable work; no fair share is
guaranteed. Conversely, an already-running function or ordinary synchronous
job delays later jobs until that operation returns
([thread.rs:101-116](../src/worker/thread.rs#L101),
[decompilation_run.rs:30-58](../src/worker/decompilation_run.rs#L30)).

Durable execution yields between functions. It does not monopolize the worker
for the entire job. However, durable `pause`, `cancel`, and `retry_failed`
arrive through the ordinary job route, and are processed in that queue
([request.rs:90-96](../src/bridge/request.rs#L90),
[durable_decompilation.rs:55-70](../src/worker/durable_decompilation.rs#L55)).
Storage-only `list_functions` and bounded `read_bytes` also enter the same
worker, despite their implementations not requiring a native engine
([remote.rs:11-29](../src/worker/remote.rs#L11)).

Sidebar requests are `Job::Sidebar` in this same executor. Their implementation
reads a stored navigation page and, for the root, file/cache inventory. They
can therefore wait behind unrelated native work. A cold root inventory still
has its own read cost; removing the native queue wait alone does not promise
an instantaneous page
([execute.rs:13-17](../src/worker/execute.rs#L13),
[sidebar.rs:50-68](../src/worker/sidebar.rs#L50),
[sidebar.rs:72-94](../src/worker/sidebar.rs#L72)).

There is an additional source-visible cancellation interleaving. `cancel`
sets one shared flag; every dequeued job resets it before execution. If Stop
arrives between durable steps and another queued job is selected first, that
reset can erase the flag before the durable runner next checks it
([thread.rs:85-86](../src/worker/thread.rs#L85),
[thread.rs:103-115](../src/worker/thread.rs#L103),
[decompilation_run.rs:30-33](../src/worker/decompilation_run.rs#L30)).
This is a possible lost cancellation, not a reproduced failure.

Use job-owned cancellation and a small control lane serviced before ordinary
work. Bound how many ordinary jobs can run before a durable step, while
retaining one mutable engine owner. Move only proven pure reads to bounded
readers with explicit project/configuration identity. A general engine pool is
not required to solve these queueing problems.

### Ordinary abandoned calls can keep consuming analysis capacity

The bridge answer wait uses a 300-second timeout for ordinary calls. It checks peer
disconnect only for questions. `Job::Remote` carries no deadline or per-call
cancellation token, and the worker detects a discarded answer only after
`self.remote` has finished
([request.rs:139-164](../src/bridge/request.rs#L139),
[job.rs:73-76](../src/worker/job.rs#L73),
[thread.rs:152-159](../src/worker/thread.rs#L152)).

The backend setting permits 30-1800 seconds and defaults to 300. Queue wait
also consumes the bridge deadline, whereas backend execution starts later.
Thus an otherwise valid long-running operation can outlive its bridge caller;
stale queued requests can still run after the answer is no longer needed
([settings.rs:79](../src/settings.rs#L79),
[settings.rs:147-153](../src/settings.rs#L147),
client.rs:42-47).

Propagate an absolute deadline and request identity through admission, queue,
and RPC. Skip abandoned queued reads and cooperatively cancel active work.
For mutations, preserve completion, rollback, and recovery semantics rather
than silently discarding a partially applied operation.

## What the durable read path fixes and leaves unchanged

`readonly_durable_batch` runs before `Job::Remote` submission and handles
`list`, `status`, and `results` directly. It clones the context and active job
under a short `RwLock` read, rejects a project transition, opens a read-only
Store, and invokes the existing query implementation
([request.rs:68-69](../src/bridge/request.rs#L68),
[request.rs:103-136](../src/bridge/request.rs#L103),
[runtime.rs:51-63](../src/worker/runtime.rs#L51)).

The response path remains bounded. Results default to metadata only. At most
100 results are returned per page, with one look-ahead row; pages with data
have a 1008 KiB payload budget, and each expanded stored artifact has a 2 MiB
decode limit and digest verification. Job lists are capped at 32 entries
([inspection.rs:23-46](../src/decompilation/inspection.rs#L23),
[pages.rs:15-24](../src/storage/decompilation/pages.rs#L15),
[pages.rs:42-53](../src/storage/decompilation/pages.rs#L42),
[pages.rs:71-88](../src/storage/decompilation/pages.rs#L71)).

This removes dependence on the busy analysis queue for those queries and
avoids reopening a native engine just to inspect saved progress. It preserves
project checks and the existing data limits. It does not bypass the MCP
helper's synchronous stdin loop, the bridge's 16-request admission limit,
database contention, result decompression, or output serialization. It also
does not move start/resume/pause/cancel out of the analysis owner.

Multiple helpers can collectively fill the bridge with long-lived questions
or calls even though each helper caps pending questions at eight. Reserve
admission for control and saved-state reads, or use a bounded multiplexed
dispatcher. Simply retaining one idle thread per persistent connection could
exhaust the same 16 slots and is not sufficient.

## Transport, engine reuse, and result representation

### Connection reuse and engine reuse are different

Prism connects TCP for every endpoint call and spawns a bridge thread for every
accepted connection
([endpoint.rs:84-106](../src/mcp/endpoint.rs#L84),
[listener.rs:41-55](../src/bridge/listener.rs#L41)).
The reference analysis `SocketTransport` retains one socket and writes multiple
messages on it. Its shared broker retains a worker for a selected target until
the final consumer releases it
(socketTransport.ts:20-37,
socketTransport.ts:61-69,
reaBroker.ts:182-201).
The reference window bridge, however, also opens a connection per exchange
(bridgeClient.ts:25-41).
Per-call connection setup is therefore a confirmed difference on the analysis
route, not a universal regression from Reference.

Prism retains a live `Engine` in worker state and opens it only when absent or
unusable. Calls update `last_used`; the idle lease is 60 seconds. The worker
drops the engine after idle expiry. Native Ghidra Drop attempts a checkpoint
for up to 30 seconds before its RPC process is terminated and reaped
([analysis.rs:20-44](../src/worker/analysis.rs#L20),
[engine.rs:15-46](../src/analysis/engine.rs#L15),
[engine.rs:67-74](../src/analysis/engine.rs#L67),
[thread.rs:118-134](../src/worker/thread.rs#L118),
client.rs:96-105,
[rpc.rs:180-190](../src/protocol/rpc.rs#L180)).

A new TCP connection does not imply a new analysis process or binary load.
Conversely, retaining TCP alone cannot avoid a native restore after engine
eviction. A memory-aware warm lease for one active engine is a separate tuning
decision from a persistent multiplexed bridge. Their benefit requires separate
measurements; neither establishes a native throughput increase.

### The agent currently loses analysis progress across the bridge

Native progress reaches the UI sink. The bridge extracts only tool name and
arguments for the worker, with no progress-token routing in `Job::Remote`.
The helper waits for one final reply
(client.rs:84-92,
[request.rs:58-66](../src/bridge/request.rs#L58),
[job.rs:73-76](../src/worker/job.rs#L73),
[endpoint.rs:106-120](../src/mcp/endpoint.rs#L106)).
No `notifications/progress` or `progressToken` handling was present in the
inspected Prism MCP, bridge, or worker sources.

The reference proxy and broker forward progress tokens, signals, and progress
notifications; the shared broker config enables timeout reset on progress
(reaProxy.ts:78-90,
reaBroker.ts:225-259,
sharedRea.ts:16-20).
Prism can therefore appear silent to a tool client while its window shows
activity. Correlated, coalesced progress should cross the request boundary.
Use meaningful-progress idle deadlines plus an absolute lifetime bound, so
progress cannot extend a stuck request forever. Durable work should continue
to return a job ID and support independent status queries.

### Large replies have an avoidable duplicate representation

Every successful helper tool result contains the object in `structuredContent`
and a JSON string of the same object in text content. The final response is
serialized into a byte vector before the 8 MiB output check
([session.rs:133-139](../src/mcp/session.rs#L133),
[pending.rs:155-167](../src/mcp/session/pending.rs#L155)).
For large data this duplicates content in the final wire message, with further
escaping in the text copy. Interprocess serialization itself is required, and
the endpoint takes the parsed result rather than cloning it
([endpoint.rs:113-120](../src/mcp/endpoint.rs#L113)).
The reference window tools return text only, but the analysis proxy preserves
the backend's result shape, so it is incorrect to assume all Reference analysis
results have only one representation
(mcp.ts:11-15,
reaProxy.ts:79-94).

Review actual consumer requirements before removing text fallback. Prefer
metadata pages and explicitly requested bounded data; retain one internal
representation until each required output encoding. The cost scales with
payload size and is unmeasured here. Tiny status results do not establish
serialization as the cause of multi-second native waits.

## Claims that should not drive a fix

| Claim | Correction |
| --- | --- |
| Every `recv_timeout(100 ms)` adds 100 ms to every answer | False. An arriving channel item wakes the wait immediately. The bridge returns from its `Ok(result)` arm immediately; RPC poll does the same. The timeout bounds idle flag checking, not ready-message delivery. |
| Prism lost parallel native analysis by using one worker | Unsupported by this comparison. The reference broker has a global serial tool queue too. Keep concurrency claims separate from fast read and control service. |
| Every tool starts a native engine | False. Per-call TCP is distinct from the retained engine. Idle expiry, project/configuration change, failure, or explicit teardown can require reopening. |
| The durable batch monopolizes the worker until every function finishes | False. It yields after each step. The remaining latency is an individual step plus queue policy, and queued work can instead starve the batch. |
| A larger or unbounded event channel fixes responsiveness | Incomplete. It postpones backpressure and increases retained data. Coalescing, byte bounds, ordering, and fair control service address the mechanism. |
| Faster `status` proves faster decompilation | False. It demonstrates a cheaper observation route. Completion time and successful native results per second are separate measurements. |
| Every JSON serialization is redundant | False. Process boundaries need encoding. The duplicated final text/object payload is a specific optimization candidate. |

For the timeout correction, see
[request.rs:154-162](../src/bridge/request.rs#L154) and
[rpc.rs:164-173](../src/protocol/rpc.rs#L164).
There is a narrower real scheduling delay: an already-open agent session can
be blocked in a 100 ms RPC idle wait when a prompt or control arrives on a
different channel. That channel does not wake the RPC wait. It may wait for
the remaining idle interval, but incoming RPC messages still wake immediately
([background/worker.rs:61-68](../src/agent/background/worker.rs#L61),
[background/worker.rs:114-121](../src/agent/background/worker.rs#L114),
[turn.rs:76-86](../src/agent/session/turn.rs#L76)).
An integrated wake mechanism can remove this limited delay without busy
polling or changing native execution.

## Recommended implementation order

1. Preserve the existing read-only durable path and its project/data limits.
   Extend it only for independently identified pure reads. Keep the analysis
   state under one owner.
2. Keep MCP input and ACP message consumption active during long calls and
   permissions. Bound pending operations and carry request IDs, cancellation,
   progress, and absolute deadlines through the route.
3. Make stop/pause/control service independent of ordinary FIFO work. Give
   durable steps a fair share and replace the global resettable cancellation
   flag with job-scoped ownership.
4. Coalesce streaming updates and replaceable progress within byte budgets.
   Preserve permission, lifecycle, completion, and error ordering.
5. Measure connection/setup time, cold restore, queue wait, native service,
   persistence, UI delivery, and result encoding separately before changing
   transport persistence or the 60-second warm lease.

Use existing Prism workflows and validation tools for subsequent
verification. Do not add tests, fixtures, or diagnostic harnesses under the
standing instruction. A performance claim needs the same corpus and settings,
cache state, successful-result count, completion wall time, tail interaction
latency, and peak memory. A smaller status-response percentile alone supports
an interaction claim, not a native throughput claim.
