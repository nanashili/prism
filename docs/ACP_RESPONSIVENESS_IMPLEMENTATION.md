# ACP permission responsiveness

Implemented 2026-10-19 in the existing Prism worktree, based on
the current ACP responsiveness implementation. The enclosing source commit is
`85a9be336ecf442cd8a8aa27a8a98060cee0646a`; Prism is an existing untracked
subtree in that checkout. The pre-change source snapshot is
`/tmp/prism-responsiveness-20261019/source-before`.

## Behavior

An ACP permission no longer waits inside the inbound request handler. The
session stores the original JSON-RPC identifier and polls a bounded approval
queue while continuing to receive root/child updates, resolve control replies,
send controls, and check Stop. The same dispatcher serves prompt, idle,
initialization, session creation, and configuration exchanges. Session controls
remain queued until session creation supplies an identifier.

At most eight human approval requests are retained. Automatic policy decisions
remain immediate. Excess requests receive the ACP cancelled outcome. Only the
oldest request is presented, so the existing single permission card cannot
replace an earlier unanswered request. Requests preserve their original string
or numeric RPC identifiers. Reusing a pending identifier, or supplying an
invalid/oversized identifier, fails the session instead of ambiguously
answering two requests.

The existing policy priority is preserved: Never, whole-file confirmation,
remembered project approval, and configured tool level. Each queued request
retains the policy snapshot from its admission. Only an offered option may be
selected, only recognized ACP allow/reject kinds are displayed, and only an
explicit `allow_always` choice can save a remembered approval. Skip retains the
UI's reject-option choice when offered; dismissal, disconnected answer channels,
invalid answers, and cancellation produce a cancelled outcome.

The queue retains bounded display/selection metadata rather than raw tool input:
RPC string IDs are limited to 1,024 decoded bytes, original titles to 4,096
bytes, and existing parsing limits cap previews, proposal text, and the first
16 options. An oversized human-approval title receives cancellation. The
existing eight-MiB transport frame bound remains in place.

## Delivery and lifetime

`Sink::try_send` preserves the existing 32-event channel. It returns an
undelivered event on saturation and reports application closure. Chat-scoped
events are returned without the sink-added wrapper so callers can retry the
same event. Successful admission requests repaint after enqueueing, avoiding a
wakeup-before-admission race.

Permission admission uses this nonblocking method. A full UI queue retains the
request locally and retries from the session loop, allowing controls and Stop
to run between attempts. No new threads, child processes, channels with
unbounded capacity, or dependencies are introduced.

Each permission event carries an active token. Stop and prompt completion
invalidate all cards before cancellation replies. Session drop invalidates
tokens before child-process teardown. Queued inactive events are ignored by the
UI, and an already visible inactive card is removed when painted. A late UI
answer cannot approve a subsequent request.

Prompt completion and failed startup/configuration exchanges settle remaining
permissions with correlated cancellation responses when the RPC connection is
usable. Independent cancellation sends are attempted even after another send
fails; failed request metadata remains owned until session disposal. The
original operation failure and cleanup failures are reported together. An
interrupted initialization/configuration call or a broken write connection
invalidates cards and is closed. The existing reusable-connection rule is
preserved through explicit RPC begin/finish operations.

## Files and integration

- `src/agent/permission.rs` is the thin permission module root; `policy.rs`
  preserves approval policy, `request.rs` owns one approval lifetime, and
  `pending.rs` owns bounded admission, answer polling, and cancellation.
- `src/agent/session/exchange.rs` owns ACP exchange and deferred dispatch.
  Session startup, configuration, prompt, idle, and background preparation
  pass the existing control receiver through this service path.
- `src/protocol/response.rs` adds a narrow correlated RPC result method and a
  completed-call operation. `src/protocol/rpc.rs` shares the existing
  reusable-connection guard through `begin_call`.
- `src/events.rs`, `src/ui/state.rs`, `src/ui/receive.rs`, and
  `src/ui/chat/permission.rs` carry nonblocking admission and active-token
  handling. Existing text/tool/terminal event order is preserved.

## Verification and limits

The complete changed methods and immediate dependencies were re-read for
ownership, response identity, bounded retention, approval rules, cancellation,
and cleanup. The independent responsiveness review found no remaining
actionable ACP issue after the RPC connection-lifetime guard was preserved.
It covered queue ordering, choice validation, policy snapshots, teardown,
configuration/control overlap, and response routing.

The integrated desktop source passed:

- `cargo check --bin prism`
- `cargo fmt --all -- --check`
- `cargo clippy --bin prism -- -D warnings`
- A physical-line audit of all 16 ACP-touched source files against the required
  ordinary/root/main limits.

Prism has no `tools/check_architecture.py` or `tools/check_source_sizes.py`, so
those repository-specific scripts were unavailable in this checkout.

No tests, fixtures, or test harnesses were authored. No GUI window was opened
or restarted. A live ACP agent issuing overlapping permissions has not been
exercised, so runtime ACP responsiveness and timing remain unverified.

This slice removes the human permission wait and blocking permission-event
admission. Existing synchronous RPC pipe writes can still wait for a child
that stops reading. Ordinary UI event delivery and remembered-approval storage
remain synchronous. General stream/progress coalescing, an integrated wakeup
for separate control channels, and global byte budgets for existing UI events
are outside this change. The existing 100-ms idle poll bounds checks on other
channels; ready RPC messages wake it immediately. No throughput or percentile
latency improvement is claimed.
