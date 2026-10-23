# Language-neutral, generation-based reconstruction research

## Scope and evidence discipline

This note concerns Prism's reconstruction coordinator for large projects and small binaries under a small-footprint CPU/RAM budget. It treats “any language” as an adapter question: a language adapter can define parsing, compilation, execution, and comparison, but no universal method can guarantee semantic equivalence from a stripped binary. Paper numbers below are benchmark results, not Prism measurements. Recommendations are proposals; “implemented” refers only to the local paths cited here. No proprietary binary analysis, new tests, fixtures, or harnesses were used. The local observations were read from the current worktree; this note does not claim a unique parent commit because the worktree contains uncommitted changes. External primary URLs below were accessed and checked for the claims attached to them.

## What Prism already has

The existing design has several good foundations for a fast generation loop:

* A generation is bound to the input SHA-256 and creates a project unit; `reconstruction_start` accepts a target language and instructions ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L3-L16)). A generation fingerprint currently covers schema version, project, language, instructions, and analyzer identity ([src/reconstruction/generation.rs](../src/reconstruction/generation.rs#L14-L83)).
* Work is leased transactionally. The queue validates worker limits, expires stale leases, and claims up to the requested number of units for a 15-minute lease ([src/reconstruction/queue.rs](../src/reconstruction/queue.rs#L7-L60)). The documented defaults are 16 units per batch and two simultaneous batches, with bounds of 1–64 units and 1–8 batches ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L18-L24)).
* Evidence is already batched and bounded: `decompile_batch` accepts up to 64 addresses, while evidence pages are capped at four functions or 1 MiB ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L26-L43)). Loaded analysis and pseudocode caches are reused, while native analysis is serialized ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L54-L60)). This is the right place to enforce the project’s batch MCP requirement: request compact pages through the batch tools instead of making one MCP call per function.
* Sources and evidence are immutable, SHA-256 addressed, and Zstandard-compressed ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L78-L85); [src/reconstruction/objects.rs](../src/reconstruction/objects.rs#L7-L51)). Submission is transactional, requires evidence for generated units, and is idempotent for an identical receipt ([src/reconstruction/submit.rs](../src/reconstruction/submit.rs#L7-L75)). Build snapshots preserve files and receipts when export fails ([src/reconstruction/snapshot.rs](../src/reconstruction/snapshot.rs#L15-L87)).
* Build commands are explicit, shell-free, cancellable, and bounded to 600 seconds. Defaults cover several languages, but each uses one build job and an unknown language needs an explicit command ([src/reconstruction/build_commands.rs](../src/reconstruction/build_commands.rs#L17-L91)). Status reports intentionally set `behavior_verified: false` ([src/reconstruction/status.rs](../src/reconstruction/status.rs#L7-L70)).

The main scheduling gap is material: the queue selects pending units by module/address and does not yet use dependency or strongly connected component (SCC) order ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L18-L24); [src/reconstruction/queue.rs](../src/reconstruction/queue.rs#L64-L116)). The documented architecture also says Prism does not automatically verify module architecture or schedule dependencies ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L45-L52)). The generation fingerprint does not include model/provider/version, prompt and tool schema, dependency summaries, compiler command, or toolchain. Those omissions can make an apparently reusable result stale.

## A practical language-neutral contract

Use a compact immutable **Unit Contract** between analysis, generation, and validation. Keep it separate from source text and from raw evidence. A contract should contain:

1. A stable unit key (input digest, function or data symbol, address range, module, and language adapter).
2. ABI facts (calling convention, parameter/return widths, volatile state, globals, imports/exports, and ownership assumptions).
3. A normalized control-flow summary (blocks, successors, exceptional edges, loops, and unresolved indirect branches).
4. Data and effect facts (reads/writes, calls, memory regions, possible I/O, concurrency or signal boundaries, and confidence/provenance for each fact).
5. Dependency keys for callees, types, constants, shared interfaces, and generated module contracts.
6. An adapter version and an explicit list of unsupported or unknown facts.

The model should produce a candidate implementation plus a contract-resolution report; the coordinator should reject a candidate that silently drops unknown effects. This makes a Rust/SQLite implementation small: normalized contract rows and dependency edges can be stored beside the existing unit/evidence tables, while large evidence remains in the existing object store. It avoids putting a whole program IR into every prompt. LLVM bitcode documents a typed, versioned serialized IR ([LLVM Bitcode Format](https://llvm.org/docs/BitCodeFormat.html)); MLIR documents extensible operations, regions, and dialects ([MLIR Language Reference](https://mlir.llvm.org/docs/LangRef/)). These are useful design precedents, not proof that either IR can recover arbitrary binaries. For Prism, a deliberately smaller contract is more suitable for low RAM and for language adapters that do not share LLVM semantics.

## Dependency scheduling and bounded parallelism

Build a graph from contract dependencies, collapse each SCC into one scheduling node, and schedule the resulting DAG. First freeze shared interfaces and public contracts; then release callee/dependency-ready components whose prerequisites are complete. A high-fan-out score may prioritize among ready nodes, but it must never bypass DAG readiness. Within an SCC, use a fixed-point round with bounded revisions. Cache the graph and each node’s dependency summary by digest.

Ghidra’s public graph API explicitly includes SCC, ancestor/descendant, and topological algorithms ([Ghidra GraphAlgorithms API](https://ghidra.re/ghidra_docs/api/ghidra/graph/GraphAlgorithms.html)). Prism’s current Rust `koute` integration should expose only the narrow graph facts it needs; it does not expose the whole Ghidra Java API. Native analysis is currently serialized ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L54-L60)), so parallelize model calls and independent contract work around that boundary rather than starting multiple analysis engines. Keep separate budgets for CPU workers, resident analysis engines, provider concurrency, and build processes. Admission control should stop new model requests when RSS, provider quota, or queue age exceeds a configured limit.

Do not fan out one MCP request per function. The official MCP basic specification defines JSON-RPC request, response, and notification messages ([MCP basic specification](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2025-06-18/basic/index.mdx)); Prism therefore needs its application-level `decompile_batch` and `reconstruction_evidence` tools for work batching. Provider batch endpoints are optional accelerators, not a property of MCP. For example, OpenAI’s Batch API is asynchronous with a stated 24-hour completion window and lower price for eligible work ([OpenAI Batch API](https://developers.openai.com/api/docs/guides/batch)); it fits noninteractive bulk regeneration, while an interactive repair should use a normal request.

## Content-addressed incremental work

Extend the existing generation key into an action key. Include the unit-contract digest, input/evidence digests, transitive dependency-summary digests, target adapter and version, model/provider/version, prompt-template hash, rendered tool/schema hash, generation settings, compiler command and toolchain fingerprints, and validator configuration. Store the action result under that key and publish an accepted pointer only after validation. A changed dependency summary should invalidate dependents transitively; an unchanged source object can still be reused when only a sibling unit changed.

Keep invalidation scopes separate: a model, prompt, or generation-tool change invalidates generated candidates and downstream validation, but should not invalidate binary analysis or language-independent evidence. A compiler command, compiler version, or validator change invalidates build/validation receipts and any acceptance decision, but should not regenerate source when the generation action key is unchanged. An analyzer or input change invalidates analysis and all dependent contracts. This separation preserves useful work while keeping acceptance claims honest.

Cargo’s official fingerprint documentation is a useful precedent: dependency fingerprints distinguish fresh and dirty work, and a failed compilation does not update the fingerprint ([Cargo fingerprint module](https://doc.rust-lang.org/stable/nightly-rustc/cargo/compiler/fingerprint/index.html)). Clang’s ThinLTO documentation similarly describes summaries, parallel backends, caching, and cache pruning ([ThinLTO](https://clang.llvm.org/docs/ThinLTO.html)). Prism already has immutable objects and generation forking, but documents no object quota/GC, model/tool fingerprints, or automatic transitive invalidation ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L78-L99)). Add a mark-and-sweep object GC rooted at frozen generations, active leases, snapshots, and receipts; retain failed diagnostics separately until their action key is superseded.

Stable prompt prefixes matter only when the provider actually caches them. OpenAI’s prompt-caching guidance requires an exact matching prefix and recommends stable tools, schemas, and ordering ([Prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching)). Put invariant contract schema and instructions first, then stable project/module context, and put volatile evidence and the unit-specific request last. Record cached and uncached token counts so prompt-cache savings are measured rather than assumed.

## Compiler feedback and semantic validation

Compilation is necessary but insufficient. Prism’s current build receipt can drive a repair loop, but its status deliberately reports no behavioral verification ([src/reconstruction/status.rs](../src/reconstruction/status.rs#L7-L70)) and build commands are sequential, language-specific checks ([src/reconstruction/build_commands.rs](../src/reconstruction/build_commands.rs#L17-L91)). Define adapter capabilities with graded outcomes:

* `parsed`: source is accepted by the language parser;
* `checked`: a configured checker accepts the source (for example `cargo check`, `tsc --noEmit`, or Python `compileall`); these do not produce a deployable artifact;
* `built`: the configured toolchain produces the requested artifact;
* `smoke`: bounded entry points run without timeout or sanitizer failure;
* `differential`: selected oracle inputs produce equivalent outputs, traces, or serialized effects;
* `formally_checked`: an adapter-specific proof or verified refinement exists.

Every level needs scope, input set, toolchain digest, timeout, and evidence. For languages without a runnable oracle, stop at the strongest honest level and expose unknown behavior. A validation adapter should provide `check`, `run`, `compare`, and `diagnostics`; diagnostics become bounded structured repair context, with a revision cap and no automatic claim of equivalence.

DecLLM is primary evidence that compiler errors and dynamic runtime outputs can be fed back into iterative decompilation; its paper describes static recompilation feedback and dynamic feedback such as ASAN/runtime results ([DecLLM paper](https://daoyuan14.github.io/papers/ISSTA25_DecLLM.pdf)). The result is a research benchmark, not a Prism guarantee. LLM4Decompile reports benchmark re-executability and refinement results for its evaluated models ([LLM4Decompile](https://arxiv.org/abs/2403.05286)); ReF Decompile reports a Humaneval-Decompile benchmark result for its relabeling and function-call strategy ([ReF Decompile](https://arxiv.org/abs/2502.12221)). These demonstrate that feedback and data strategy can improve selected benchmarks; they do not establish semantic recovery for arbitrary stripped programs. AutoDecompiler likewise frames compilation/execution/I/O feedback as a way to catch single-turn compilable but divergent outputs ([AutoDecompiler](https://arxiv.org/abs/2606.16162)).

## Minimum vertical slices and trade-offs

Implement in this order, preserving the current transaction and lease boundaries:

1. **Action-key completeness.** Add model/tool/prompt/compiler/adapter/validator fingerprints and dependency-summary hashes. This is low risk and prevents false reuse, at the cost of more cache misses when tools change.
2. **Compact Unit Contract.** Persist normalized ABI/CFG/effect/dependency facts and provenance, with bounded evidence pages. This costs analysis work and storage but reduces prompt size and makes language adapters explicit.
3. **SCC scheduler.** Add graph edges, condensation, critical-path priorities, and bounded fixed-point rounds. This improves dependency readiness but adds graph maintenance and can delay independent leaves if priorities are poorly tuned.
4. **Structured repair loop.** Convert compiler diagnostics and receipts into bounded, deduplicated repair events with retry budgets. This improves compile rate while increasing provider calls and latency on hard units.
5. **Validation adapters.** Add smoke/differential execution only where a lawful oracle exists; retain `behavior_verified: false` otherwise. An execution adapter is a production capability, not a request to author new test fixtures or harnesses. Stronger confidence costs build and runtime resources and remains sample-limited.
6. **GC and measurement.** Add object-root accounting, transitive invalidation traces, and cold/warm telemetry before tuning concurrency.

For a small binary on low RAM/CPU, start with one resident analysis engine, streamed SQLite rows, four-function/1 MiB evidence pages, a small model concurrency cap, and one build process. Increase concurrency only after RSS and provider queue telemetry show headroom. The existing one-job build defaults protect resource use but leave throughput on the table; parallel builds should be an adapter-level opt-in with a memory budget.

## Measurement and limits

Compare the same corpus, target language, hardware, provider, model, prompt version, and acceptance policy in cold and warm runs. Report wall time, CPU time, peak RSS, analysis time, queue wait, model latency, input/output/cached tokens, provider cost, batch size, retries, compile rate, validation level, accepted-unit rate, and p50/p95 per-unit and whole-project latency. Attribute speedups to batching, cache hits, scheduling, or fewer repair rounds.

An “effective 10x” claim is meaningful only against a specified baseline and quality floor. More model parallelism cannot remove serial analysis, graph coordination, compilation, or validation; larger batches can increase tail latency and failure blast radius; stronger semantic checks can dominate time. Prism currently has no 10x or universal-equivalence measurement ([docs/RECONSTRUCTION.md](RECONSTRUCTION.md#L142-L148)). Treat 10x as an experiment hypothesis, never as a guarantee, and publish results separately for each language adapter and validation level.
