# Generational reconstruction

Prism coordinates reconstruction through the existing chat agent, Rust worker and
MCP bridge. The selected binary remains unchanged. Each output root is bound to
its input SHA-256 and contains its own `.prism-reconstruction/state.sqlite`.
This is a new Prism workflow; original Reference reconstruction jobs are preserved
by import but are not migrated into these generations.

## Workflow and ownership

`reconstruction_start` creates or resumes a generation with a free-form target
language and user instructions. A generation inventories indexed functions and
one `__project__` unit for build configuration, shared interfaces, resources and
platform integration. That unit must have source before function work becomes
eligible. Inventory insertion streams from the project database, rather than
loading the full function list into memory.

`reconstruction_claim` leases a module batch to a stable worker name for fifteen
minutes. Repeating a claim with the same worker returns its active lease. The
default is sixteen units per batch and two simultaneous batches; configurable
limits are one to sixty-four units and one to eight batches. Claims are gated by
contract readiness and dependency completion; strongly connected components are
kept together until every member contract is ready. Agent delegation depends on
the selected provider; Prism does not launch extra model processes just because
the worker limit is increased.

Always batch decompilation and reconstruction. `decompile_batch` on
`prism-analysis` accepts up to sixty-four addresses and returns at most four
results per page. Follow `next_offset` until null. A focused selection is a
one-item batch. For an explicit reconstruction request, use
`reconstruction_evidence` on `prism` instead. The existence of a generation does
not route a plain decompile request into reconstruction. Direct
`procedure_pseudo_code` calls through `analysis_call`
are rejected and omitted from `analysis_tools` for every provider, including
Ghidra, Rizin and REA. Individual UI code views remain available. MCP startup
instructions and chat context share the same batch requirement, and all nine
built-in playbooks require batch commands whenever they request decompilation.
Missing or failed batch tools must be reported rather than bypassed with direct
pseudocode requests, visible-text exports, shell loops or individual source writes.
Android method IDs are not numeric addresses for the native batch reader; use
the provider's advertised Android bulk or paginated source operations. Managed
targets require advertised bulk export or paginated type/method operations and
cannot be routed through a native decompiler. Each evidence page contains at most four
functions within a one-MiB reply budget. Oversized functions and analysis failures produce explicit
per-function errors. A full page never silently discards later functions.

The `decompile` and `recompile` skills require modular output. Decompilation
groups observed functions into bounded module files with an address/output/status
manifest, retaining uncertainty about original source boundaries. Reconstruction
records module ownership, shared interfaces and dependency direction in the
initial project unit, then uses `reconstruction_dependencies` and
`reconstruction_contracts` to persist graph and interface readiness. Contract or
dependency changes invalidate affected consumers while preserving files for
merge. These readiness gates do not prove recovered behavior.

The worker reuses its loaded analysis engine, checksum-verified durable
pseudocode artifacts and configuration-scoped view cache. Reconstruction
evidence excludes legacy upstream UI cache fallback entries.
Native Ghidra evidence adds local control-flow and direct-reference information
without requesting a whole-program caller scan. Successful new evidence in a
page is saved in one transaction. Native analysis stays serialized; batching
reduces request and persistence overhead rather than creating unbounded Ghidra
processes. Backend decompiler limits and incomplete indirect-call information
still apply.

`reconstruction_submit` atomically publishes UTF-8 source and every leased unit's
result. Files carry their previous SHA-256, with an empty hash for a new path;
conflicting edits require a read and merge. A repeated identical submission is
idempotent. Generated functions require recorded evidence and existing source
paths. Blocked units require a specific reason. These are agent reports, not
automated judgments of implementation quality or behavioral equivalence.

`reconstruction_control` pauses/resumes work, retries blocked units, reopens a
unit for refinement, and renews/releases leases. Pause releases active batches.
Expired batches become eligible again. Active leases must finish or be released
before a new generation freezes the old one. State survives normal app restarts.
The reconstruction screen uses the shared chat, shows progress and generation
history, and exposes start, continue, pause, build, source export and report.

## Reuse and storage

Source and evidence are SHA-256 addressed, Zstandard-compressed SQLite objects.
Generations share identical objects without duplicating their contents. The
source-reuse key includes input hash, target language, user instructions, schema
version, analyzer, model, prompt and language-adapter identity. Matching generations reuse
reported completed units and source mappings. Changing language, instructions,
model, prompt or adapter invalidates source work but retains language-independent evidence
when the analyzer matches. Changing the analyzer invalidates both. Validation
identity changes rerun validation without redecompiling. Failed or blocked units
remain pending in the new generation. Earlier generations remain frozen and
inspectable. The UI supplies the selected provider/model identity; when the
model is unknown (`default`), a fork conservatively invalidates generated source
instead of treating that placeholder as a stable model version.

Evidence and source objects are limited to one MiB each. Submissions allow at
most sixty-four files, four MiB of source and sixty-four results. SQLite uses a
two-MiB page-cache target and a 524,288-page database limit, approximately two GiB
with its normal four-KiB pages. The initial inventory is limited to one million
functions. Reports stream rows and snapshots export one file at a time.

These limits are not a global storage quota. Retained generations, original
imports, source snapshots, build products and external tool caches consume disk
space. Snapshot/build cleanup is manual. Object garbage collection,
binary-resource ingestion and source deletion remain outside the first version.
Dependency and contract changes increment the generation revision and invalidate
affected consumers. Sources and reports explicitly retain unverified status.

## Builds and reports

`reconstruction_build` first exports a unique snapshot tied to the generation's
source revision. Later submissions cannot change that snapshot's source through
Prism. Its build receipt records revision, output path, outcome and bounded
diagnostics. Export failures preserve partial output and attempt to save the
failure receipt while retaining independent commit errors. Export-only snapshots
have no successful compiler result. An interrupted build may leave an unfinished
receipt; that is never interpreted as success.

Default commands cover Rust, C, C++, C#, Swift, Go, TypeScript, Maven Java, Zig
and Python syntax compilation. Rust, TypeScript and Python are checked by
default; the other listed languages are built when their toolchains are
available. Other languages or build systems use an explicit
`command: {program, args}`. Toolchains must already be available. Rust uses
`cargo check`; Python uses `compileall`; neither establishes a packaged app.
Commands run without an implicit shell and use a single build worker where the
adapter supports it. Each command has a ten-minute timeout and cancellation
uses the existing process lifecycle handling. Inspect build scripts and hooks
before execution. Builds run in the shared background worker, so they temporarily
serialize other analysis requests. Independent snapshots do not currently share
a managed incremental build cache.

Compiler feedback can reopen affected units for another claim/repair/submission
cycle. `reconstruction_report` exports inventory states, blockers, counts, reuse
and the latest build receipt. `reconstruction_units` and `reconstruction_files`
page inventories; `reconstruction_read_file` supplies source and its current
hash. Large inventories are never automatically stuffed into a chat prompt.

Prism embeds and packages the nine playbooks from `skills/`, including the
updated `decompile` and `recompile` skills. The reconstruction prompt references
the installed skills without inserting their full text into the composer.
Normal agent context also requires bounded batch work.

## Performance target and verification limits

The implemented speed mechanisms are module batches, bounded concurrent leases,
loaded-engine and evidence reuse, generation source reuse, compressed shared
objects, streamed inventory/export, transactional publication, SCC scheduling
and dependency invalidation. Research into stable-prefix model caching, provider
Batch APIs and richer feedback is recorded in
[RECONSTRUCTION_RESEARCH.md](RECONSTRUCTION_RESEARCH.md). Those proposals are not
all implemented. The provider Batch API is not automatically used by ACP agents.

A tenfold speedup remains a target, not a measured result. Compare sequential
and batched workflows on the same authorized corpus, language, hardware,
provider/model and acceptance criteria, recording cold/warm cache separately.
Measure elapsed time, resource use, model cost, retries, compile outcomes and
remaining behavior gaps. No new tests, fixtures or harnesses were authored for
this feature. Compilation, existing checks and any manual acceptance reported
separately cannot establish universal recovery or behavioral parity.
`behavior_verified` remains false unless separately supported evidence exists.
