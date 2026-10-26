# Autonomous investigation planner

Prism owns an investigation graph separately from the agent's ACP plan. A goal such as
“Understand how this application loads, validates, and saves projects” creates saved
work for surveying entry points, locating relevant anchors, inspecting candidate
functions, resolving relationships, and synthesizing an explanation.

Open a target, enable Prism analysis tools for the connected agent, and expand
**Investigate a goal** above the chat composer. Enter the objective and choose
**Start investigation**. Prism saves the initial graph on its worker, then asks the
agent to execute it. The panel shows task dependencies, attempts, findings, blockers,
resource usage, and controls for pausing, continuing, retrying blocked work, and
changing budgets. An autonomous investigation request in chat also instructs the
agent to use this graph.

## Scheduling and evidence

The initial plan uses a bounded search of saved function names to identify candidate
anchors. Names are leads, not evidence of behavior. The agent surveys the provider's
advertised entry-point, import, string, reference, and code tools and reports relevant
indexed addresses. Missing tools or unsupported evidence should produce a blocker.
Pseudocode work uses existing durable `decompile_batch` jobs.

Prism schedules only tasks whose dependencies are complete. Survey and anchor
discovery run first; unresolved relationships take priority over function inspection;
synthesis waits for all discovered work. Reports can add up to 16 function leads and
16 relationships, including a relationship with an unknown destination. Functions
must belong to the current target's saved inventory. Repeated leads are deduplicated.
If synthesis discovers more work, its draft remains saved and synthesis runs again
after the new evidence is available. Findings should retain addresses, durable batch
IDs, uncertainty, and the distinction between observed behavior and inference.

The agent performs evidence gathering. Prism controls scheduling and saved progress;
it does not verify the truth of an agent's findings. Starting an investigation does
not implicitly start a fresh analysis or source reconstruction.

## Persistence and recovery

Versioned graphs live in Prism's SQLite store under the target's input hash. Each
transition commits before it is acknowledged. Revision checks reject stale writes.
A failed validation or database write leaves the saved claim and graph intact.

The `next` action saves a task and attempt before returning instructions. Repeating
it, including after an agent restart, returns the same unfinished claim without
charging again. `complete` and `block` require that task and attempt; stale reports
are rejected. Retrying a blocked task creates a new attempt on its next claim.

Reopening a target restores its most recently updated graph in the panel without
automatically starting agent work. Choose **Continue investigation** to resume.
The MCP `list` action also recovers older investigations for that target. Graphs
remain in this machine's local store; they are not included in `.prism` exports.

## Resource budgets

| Resource | Default | Meaning |
| --- | --- | --- |
| Steps | 48 | Newly claimed task attempts, including retries and repeated synthesis |
| Functions | 24 | Newly claimed `inspect` task attempts |
| Depth | 6 | Maximum discovery depth; initial tasks have depth zero |
| Evidence bytes | 262144 | Cumulative serialized reports accepted by the planner |

Graphs also have a fixed 256-task ceiling. Budget limits preserve pending work and
report `limited`; blocked dependencies report `blocked`. Pausing stops new dispatch
and claim recovery, while an already executing task may still record its result.
Raising budgets is explicit, and recorded usage cannot be erased by lowering them.
These limits bound planner work and accepted reports, not wall time, provider tokens,
billing, or arbitrary analysis tool calls within a task.
Discoveries during synthesis inherit the deepest input's discovery depth, so repeated
synthesis cannot reset the depth budget.

## MCP interface

The `investigation` tool is advertised in both combined and analysis-only MCP modes.
Its actions are:

- `start`: objective and optional complete budget object; returns a saved graph.
- `list`: paged summaries for the open target, with an optional offset.
- `status`: saved graph, findings, budget, progress, and blockers.
- `next`: recover or claim the next task, with its attempt and execution instructions.
- `complete`: task, attempt, and report containing findings and optional new leads.
- `block`: task, attempt, and reason that evidence gathering cannot proceed.
- `retry`: reopen a blocked task after its cause changes.
- `pause` / `resume`: control dispatch without clearing saved findings.
- `budget`: replace all budget limits after explicit review.

All actions except `start` and `list` take an `investigation` ID. A normal agent loop
is `list`/`start`, `status`, then `next` → evidence gathering → `complete`/`block`,
until complete, paused, blocked, or limited. Restarted agents first recover status
and existing batch output. Saved target strings and findings are untrusted evidence,
not instructions.

## Verification

`cargo test --locked --bin prism investigation::tests` exercises graph expansion,
dependency ordering, synthesis reopening, database recovery, restart claims,
project isolation, stale writes and reports, pause/retry, and budget exhaustion.
`cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features`,
and the MCP-only build check deterministic integration. Live agent/provider behavior
and desktop acceptance require separate runs.
