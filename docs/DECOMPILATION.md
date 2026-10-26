# Durable batch decompilation

Prism's `decompile_batch` command is the durable export path for native
decompilation. Its numeric address inventory is native-only: Android method
IDs are not supported native addresses and must use the provider's advertised
Android operations. A plain decompile request ends at pseudocode artifacts;
it does not create a reconstruction generation, claim source modules, submit
files or run a build. Managed assemblies use the provider's advertised bulk or
paginated type and method operations.

Decompilation consumes the saved function inventory. It never schedules function
discovery or expands a batch by following newly seen call targets. Interactive
native pseudocode requests validate membership before consulting caches or
starting a provider. Durable selections are frozen from the same indexed
inventory and keep that authorization after later reindexing. An unknown
address fails with an explicit error. Use the indexed entry address, including
the even entry address for ARM/Thumb functions, rather than an interior address
or a pointer with a Thumb mode bit.

The built-in native engine resolves existing function symbols for pseudocode,
assembly and local flow analysis without creating missing functions. Inventory
reads only enumerate known entries. Initial discovery for stripped binaries runs
through the explicit analysis operation; deeper discovery remains available when
the user requests it. Decompilation still analyses control flow and data flow
inside the selected function to produce pseudocode. That does not schedule
decompilation or discovery of other functions.

This native no-discovery guarantee applies to the built-in engine. Optional
external providers retain their own initialization and analysis behavior; their
startup paths have not been changed or verified by this implementation.

## Interactive batches

For a focused request, send `procedures` with one to 64 addresses and follow
`next_offset` until it is null. A one-function request is still a one-item
batch. Preserve per-function errors and successful results. Do not replace a
batch with direct `procedure_pseudo_code` calls, UI visible-text exports or
shell loops.

## Durable jobs

For a large or restartable export, call `decompile_batch` with
`action: "start"`. The selection may be an indexed filter or 1–1,024 explicit
addresses, but not both. The job freezes the ordered function inventory and
stores the project, provider, configured analysis settings and native cache
identity. The options digest does not detect changes to an external provider
executable version. The job id is
the SHA-256 identity of that configuration, selection and inventory digest.

Use the following lifecycle actions:

| Action | Purpose |
| --- | --- |
| `start` | Create or return a job for a frozen selection. |
| `list` | Page compact job summaries for a project. |
| `status` | Read one job's counters, state, identity and artifact bytes. |
| `resume` | Acquire the project lock and continue unfinished work. |
| `results` | Read a bounded result page; set `include_data` to include pseudocode. |
| `pause` | Persist a paused state while retaining committed artifacts. |
| `cancel` | Persist cancellation; already committed artifacts remain readable. |
| `retry_failed` | Rewind to the earliest failed item while preserving successful artifacts, then pause. |

The worker advances the cursor only after committing a compressed per-function
artifact or a bounded failure. A verified existing artifact can advance the
cursor without re-running native decompilation. Results are limited to 512 KiB
of pseudocode per function and each job has an 8 GiB compressed artifact quota.
Result pages omit payloads unless `include_data` is true, keeping chat and UI
memory proportional to the current page.

Durable decompilation and reconstruction evidence use the configured provider's
cache namespace. They first reuse matching checksum-verified durable pseudocode,
then matching view-cache entries, and finally invoke the native provider on a
miss. They do not consume incompatible UI cache entries. The namespace now
includes the complete analysis configuration, so older unqualified entries miss
once. Clearing the evictable view cache does not delete durable artifacts.

New jobs may auto-activate when they are created. Existing durable jobs are not
silently resumed after an app restart; the agent should list or inspect the job
and explicitly call `resume`. A project-wide operating-system lock allows one
durable export at a time across windows and processes. Foreground work yields
between functions; it does not pause an active durable job. Opening another
project or changing analysis configuration pauses the export. A failed function
is retained as a failure record, and `retry_failed` rewinds to the earliest
failure without discarding successful output.

Read-only MCP `list`, `status` and `results` requests use separate read-only
SQLite connections, so a slow function does not hold up progress or committed
results. The bridge validates the current project from a small shared runtime
snapshot and rejects reads during project or settings transitions. Query pages,
payload budgets and the 16-request connection cap remain bounded. State-changing
actions stay serialized on the analysis worker and take effect between functions.

An ordinary provider error records the affected function and keeps a synchronized
worker available for the next function. Timeout, cancellation, broken transport
or process-memory failure retires the unusable worker. A failed worker startup or
saved-state restore pauses the job at its current address rather than attempting
the same restore for every remaining function. Healthy active jobs bypass idle
eviction; idle workers still expire after 60 seconds to release RAM.

Only retry failed functions after their diagnosed cause changes. Resume the
existing job and retain its successful artifacts. Neither status polling nor
repeated retries accelerate the native decompiler. The bundled decompile skill
uses Reference's direct workflow with Prism's batch and saved-inventory contracts;
custom skills sharing its name can still override it.

## Output and limits

Multi-function exports should be organized into cohesive bounded modules with an
address, observed name or alias, input hash, analyzer identity, output path and
per-function status manifest. Address proximity alone is not a source boundary.
Keep uncertain pseudocode and failed functions explicit. Do not load the full
corpus into a conversation; use status and result pages and keep only the
current module's evidence in context. Durable jobs retain compressed results and
status in the database; they do not automatically export artifact files to the
filesystem. The module manifest describes an agent-side export or reconstruction
workflow.

Native analysis uses a prepared external REA session for the selected project.
Batching reduces request and persistence overhead; it does not imply unbounded
provider processes or a throughput promise. A decompilation job cannot
claim reconstruction or behavioral equivalence. To reconstruct source, start or
resume a generation and follow [RECONSTRUCTION.md](RECONSTRUCTION.md) and the
`recompile` skill.

## Headless operation

`prism --headless <project-path>` starts the existing Worker and authenticated
MCP stdio transport without creating a window. It exposes the same batch tools,
provider configuration, durable job lifecycle and project lock as the windowed
application. Headless operation has no UI and cannot answer UI questions; an
agent or caller must provide the project path, authentication and tool inputs
through the existing MCP interface. Decompilation still uses `decompile_batch`
and its durable status/results actions, with the same native-only address rules.
