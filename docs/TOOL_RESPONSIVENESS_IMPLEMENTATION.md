# Bounded MCP requests and serial worker scheduling

This implements the MCP, bridge, and analysis-owner portion of
the current native responsiveness measurements. It changes protocol admission
and scheduling without adding native engine workers or changing the existing
saved-result query limits. ACP permission/event changes are described separately
by their owning implementation slice.

## MCP helper

`Session` keeps reading stdin while validated tool requests wait on independent,
bounded endpoint calls. Each pending entry retains its JSON-RPC ID, cancellation
flag, admission class, byte charge, and join handle. Responses carry the original
ID and share one serialized output lock. `ping`, initialization, and `tools/list`
continue through the input loop.

Structural JSON-RPC validation precedes dispatch. Tools require successful
initialization and the existing mode/tool/argument validation. Questions retain
their dedicated parser. A duplicate active ID receives an invalid-request error
and does not replace the original pending request. Cancellation notifications
resolve the matching active ID, including ordinary tool requests.

Helper admission and global bridge admission use the same classes:

| Class | Pending requests | Aggregate request bytes | Overall deadline |
| --- | ---: | ---: | ---: |
| Ordinary | 4 | 16 MiB | 300 seconds |
| User question | 8 | 1 MiB | 3600 seconds |
| Durable control | 2 | 1 MiB | 300 seconds |
| Saved/window state | 2 | 1 MiB | 300 seconds |

Durable control means `decompile_batch` actions `pause`, `cancel`, and
`retry_failed`. Saved/window state means durable `list`, `status`, and `results`,
and `get_state`, `current_view`, and `list_marks`. Each class has independent
capacity, so ordinary calls and questions cannot consume its reserved entries.
Other operations retain ordinary admission.

The helper allows at most 16 request threads, each configured with a 256 KiB
stack. Input and reply messages retain the 8 MiB limits. Final helper response
serialization uses a capped buffer and emits a correlated size error when the
reply does not fit. The existing structured result and text fallback are
preserved. Byte charges measure incoming frames, not exact heap use; parsed JSON,
result values, text fallback, and stacks add overhead. These are resource bounds,
not measurements of RSS or latency.

EOF or input failure cancels all pending calls and joins them. Independent join
failures are collected. An original input/output error is preserved alongside
shutdown failures rather than being replaced by cleanup status.

## Bridge admission and request lifetime

The bridge retains a connection-per-call protocol. Its listener allows at most
20 connection threads, each configured with a 512 KiB stack. Sixteen calls can be
admitted across the classes, leaving four connection slots for receiving and
rejecting additional requests. A caller must deliver its initial frame within
an absolute two-second handshake limit. The read checks shutdown and bounds the
frame at 8 MiB, including when a client trickles bytes. Final socket writes have
a two-second write timeout.

Authentication and the existing tool validation happen before admission. The
helper transmits the remaining overall timeout; the bridge clamps it to the
class maximum and constructs a request lifetime. Reading and validation consume
that budget. Missing timeout metadata retains the explicit 300/3600-second
limits for existing direct bridge clients.

All waiting routes check the deadline, bridge shutdown, and peer disconnect.
Answer-channel waits wake immediately when a result arrives, with 50 ms idle
checks. Sending a UI event uses the nonblocking `Sink::try_send`, retains the
unadmitted event, and checks lifetime/disconnect between bounded retries.
Questions preserve their individual answer channel, chat identity, and active
flag. Finishing or abandoning a question clears its active flag and repaints.

Saved durable `list`, `status`, and `results` remain direct read-only operations.
They still use `RuntimeSnapshot`, reject project transitions, open a read-only
Store, and retain the existing context, pagination, decode, digest, and payload
checks. They do not enter the mutable analysis queue. Window-state requests keep
the UI-owned route.

Native `Job::Remote` carries an optional `RequestLifetime`. Bridge requests always
supply it; the local inspector supplies `None` to retain its existing local
execution semantics. A queued request checks its lifetime before entering
`State::remote`, so stale requests do not start analysis or mutations. During
execution only that exact request can signal the engine's existing cancellation
flag. A locked activation guard detaches the flag before releasing the result,
preventing late cancellation from poisoning the next native job. Cancellation
cleanup errors are attached to the original operation failure when both occur.

Window events also carry the lifetime. The UI checks it immediately before
applying the command, so an expired queued rename, note, navigation, or other
window command is rejected. A delayed screenshot carries the lifetime through
to its file-write check. Window commands that validly enqueue a local operation
retain their existing acknowledgment semantics: once accepted by the UI, the
scheduled local operation belongs to the window.

## One mutable owner and durable fairness

`WorkerPort` replaces the exposed ordinary sender. It uses nonblocking admission
to an eight-entry ordinary queue and a separate four-entry control queue. Only
validated durable control actions use that lane. There remains one worker,
`State`, and mutable native `Engine`.

A scheduling round services at most two controls, then at most two ordinary
jobs, then one durable function step if a runner remains active. Sustained
foreground arrivals therefore cannot indefinitely defer the next durable step.
Controls are checked before ordinary work in each round. An idle worker checks
again within 25 ms. An asynchronous batch step reporting no progress uses the
same wait when the round did no foreground work, avoiding a busy polling loop.
These are bounds in completed operations, not wall-clock
preemption: an individual native function or ordinary job can still delay the
next control.

UI Stop uses an atomic generation counter in addition to the native execution
flag. At the next execution boundary, the worker clears the transient flag,
observes any unacknowledged Stop generation, and checkpoints/pauses the durable
runner before running another queued job. It acknowledges the generation only
after pause succeeds. A Stop arriving during that boundary remains visible in
the counter and/or current execution flag. Clearing the shared flag for a new
job consequently cannot erase a pending durable Stop.

Per-request cancellation does not increment the global Stop generation. It
cancels that request's native execution and does not pause an unrelated durable
runner. The following execution boundary clears its transient flag. Worker
shutdown stops taking new work and attempts the existing durable pause/checkpoint;
a failed checkpoint is reported instead of silently discarded. A checkpoint
failure before a queued job is returned to its caller and stops the worker.

## Verification and limits

The changed methods and immediate dependencies were read in full after
formatting, including validation, cancellation ownership, queue transitions,
result delivery, and cleanup. All MCP slice source files are within the required
physical line limits. `tools/check_architecture.py` and
`tools/check_source_sizes.py` are not present in this Prism checkout; a direct
line-count audit covered the 29 touched Rust files instead.

Executed during implementation:

- `cargo fmt --all`.
- `cargo check --no-default-features --bin prism-mcp`.
- `cargo clippy --no-default-features --bin prism-mcp -- -D warnings`.
- Shared `cargo check --bin prism`, coordinated with the ACP owner after the
  `Navigate` lifetime change.
- Shared `cargo fmt --all -- --check` and
  `cargo clippy --bin prism -- -D warnings`, both passing after all coordinated
  changes, with zero Clippy warnings.

The independent read-only reviewer found no remaining actionable MCP, bridge, or
worker issue after checking the final source. That review did not exercise the
runtime. No new tests, fixtures, or test harnesses were authored. No GUI, native
analysis run, release build, or profiling session was launched by this slice.

Cancellation remains cooperative. An already-running filesystem/database/native
operation is not forcibly interrupted by `RequestLifetime`; existing backend
cancellation and timeout behavior still governs its return and mutation recovery.
The 300-second ordinary overall deadline remains explicit even when a backend
setting permits up to 1800 seconds. Queue time consumes the overall budget. A
long configured native operation can therefore be cancelled by the transport
deadline before its configured backend timeout.

Read-only SQL queries check lifetime before and after the query rather than
interrupting SQLite midquery. Endpoint discovery still performs the existing
bounded metadata/file read. Helper output requires a functioning output consumer:
a generic blocking `Write` can remain blocked if the caller neither consumes nor
closes its pipe. Native completion and teardown can also take their existing
backend cleanup time. The bounded request waits do not establish a universal
shutdown deadline for arbitrary external I/O.

This slice introduces no dependency and makes no native throughput, tail-latency,
or memory-reduction claim. Those require the coordinator's before/after
measurements with matching inputs, cache state, settings, and successful results.
