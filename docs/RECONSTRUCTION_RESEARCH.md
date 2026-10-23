# Fast, generation-based reconstruction research

Status: design research for Prism, with a local code inspection on 2026-10-19.
This note uses public documentation, published research and Prism's source. It
does not use proprietary decompiler output, raw binary analysis, or live model
prompts. “Evidence” means a source reports or specifies the behavior. “Proposal”
means a Prism design recommendation.

## What the sources establish

Traditional reconstruction is a pipeline: disassembly extracts instructions,
then decompilation reconstructs higher-level structure and source-like code.
That separation is explicit in the USENIX description of reverse engineering
and matters because a readable result is not necessarily behaviorally correct
([Kruegel et al., 2004](https://www.usenix.org/legacy/publications/library/proceedings/sec04/tech/full_papers/kruegel/kruegel_html/node1.html)).
Compilation is lossy: names, types, structured control flow, and other
source-level information may be absent ([Yang and Nagy, 2025](https://www.usenix.org/system/files/atc25-yang-zao.pdf)).
Universal reconstruction therefore needs an explicit uncertainty state and
cannot promise recovery of the original source or a single canonical language.

Ghidra’s Java API exposes the useful unit of work. Its decompiler API returns
source-like C, a prototype, a high-level function structure, markup, warnings,
timeouts, and cancellation status per function
([DecompileResults](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/DecompileResults.html)).
Ghidra also exposes call/data/control-flow graph types and graph algorithms
including subgraphs, cycles, dominance, and strongly connected components
([graph package](https://ghidra.re/ghidra_docs/api/ghidra/graph/package-summary.html),
[GraphAlgorithms](https://ghidra.re/ghidra_docs/api/ghidra/graph/GraphAlgorithms.html)).
The function graph is therefore a defensible basis for dependency-aware work
units rather than sending a whole binary to a generator. These links document
the upstream Java API, not Prism’s effective public surface: Prism uses the
pinned `koute` Rust wrapper and its existing native worker, which expose only a
subset of Java Ghidra services. Any implementation must first map each proposed
field or graph query to the wrapper capability actually available in Prism;
unavailable values remain unknown rather than being inferred from the Java
documentation.

There are hard analysis bounds. Ghidra can refuse functions above its maximum
instruction count and can stop following an indirect jump when a jump table
exceeds its configured entry limit
([DecompileOptions](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/DecompileOptions.html)).
These are expected outcomes to record as `bounded` or `blocked`, not failures to
hide with retries.

LLM decompilation results support a staged workflow, but do not establish
universal equivalence. LLM4Decompile reports better re-executability than its
baselines and a further improvement when refining Ghidra output
([Tan et al., 2024](https://arxiv.org/abs/2403.05286)).
ReF Decompile reports that relabeling jump targets and enriching function-call
context improves source-level recovery on its benchmark, but its reported
61.43% score is benchmark performance, not proof of recovery for arbitrary
stripped binaries ([Feng et al., 2025](https://arxiv.org/abs/2502.12221)).
DecLLM explicitly defines a narrower goal, recompilable decompilation, and
uses static recompilation plus dynamic runtime feedback to repair decompiler
output ([DecLLM, ISSTA 2025](https://daoyuan14.github.io/papers/ISSTA25_DecLLM.pdf)).
Recent feedback-driven work treats compilation, execution, and I/O comparison
as separate signals rather than treating a plausible response as proof
([Liu et al., 2026](https://arxiv.org/abs/2606.16162)).
The practical consequence is a generator plus a verifier loop: parse, compile,
run permitted differential checks, and retain the evidence and unresolved
diagnostics alongside the candidate. “Translation” means producing a
source-like or compilable artifact. “Verified recovery” requires a separately
recorded equivalence or behavioral oracle, with its scope and blind spots;
these terms must not be conflated in Prism status labels.

## Proposed Prism architecture

The following is a proposal that fits Prism’s existing Rust/SQLite/Ghidra
shape. It is deliberately language-neutral: the target language is selected by
an adapter, while the intermediate record and evidence model remain stable.

1. **Inventory and identity.** Hash every input artifact and record architecture,
   format, compiler hints, symbol/debug metadata, and Ghidra analysis version.
   Give each function a stable key `(artifact_hash, address, view, analysis_key)`.
   Store source candidates, generated candidates, diagnostics, and validation
   results as immutable records keyed by content hash; make the latest accepted
   pointer a separate SQLite row.
2. **Dependency graph.** Materialize functions, imports, callers, callees,
   globals, types, strings, and resources as graph edges. Condense recursive
   strongly connected components, then schedule leaf-first batches. Send a
   function with its minimal slice of callees, signatures, relevant globals,
   and ABI facts; add callers only when the requested contract needs them.
3. **Generation stages.** Use deterministic, bounded stages: normalize Ghidra
   output; infer a language-neutral contract; emit target-language code; ask a
   repair pass only with structured diagnostics; then attempt symbol/name/source
   recovery from available debug data and prior accepted records. Preserve every
   candidate, prompt metadata, model/version, and decision reason.
4. **Feedback boundary.** A build adapter parses syntax and compiler diagnostics.
   An execution adapter may compare outputs, traces, or system-call slices only
   when a safe, independently authored oracle exists. A failed check creates a
   new content-addressed attempt. A successful compile is a gate, not a
   behavioral-equivalence claim. Trace-based binary comparison is useful but
   conditional and incomplete ([BinSim](https://www.usenix.org/conference/usenixsecurity17/technical-sessions/presentation/ming)).
5. **Scheduling and persistence.** Keep Ghidra analysis serialized per project
   where its process/database requires it, but parallelize independent function
   extraction, prompt preparation, generation requests, parsing, and validation.
   Use a bounded Rust worker pool and SQLite transactions for state transitions;
   never let an unbounded model queue or a cache miss create unlimited work.

## Incremental work and reuse

Cargo’s fingerprint model is a useful precedent: a unit is fresh or dirty,
fingerprints include dependency fingerprints, and failed compilation does not
publish the new fingerprint ([Cargo fingerprint design](https://doc.rust-lang.org/stable/nightly-rustc/cargo/compiler/fingerprint/index.html)).
LLVM ThinLTO similarly combines compact module summaries, optimizes independent
backends in parallel, and supports an incremental cache with explicit cache
pruning ([Clang ThinLTO](https://clang.llvm.org/docs/ThinLTO.html)).

**Proposal:** model each reconstruction phase as a pure-ish action with a
canonical input manifest and a SHA-256 output record. The manifest should
include artifact hash, function key, dependency hashes, target adapter, prompt
schema version, model identifier, generation settings, and tool versions. A
cache hit is reusable only when all fields match. Failed or partial actions
remain inspectable but do not replace the last accepted output. Evict by size
and last access, retaining the manifest and failure reason so a later run can
explain a miss.

**Proposal:** reuse symbols and source in descending confidence: embedded debug
records; verified symbol/import metadata; prior accepted Prism records with the
same artifact/function key; structurally similar functions; then generated
inference. Never overwrite a stronger fact with a weaker inference. Keep
provenance at field level so a recovered name does not imply a recovered type or
implementation.

## Batching, prompt reuse, and bounded parallelism

OpenAI’s Batch API supports asynchronous request groups, higher rate limits, a
24-hour completion window, and 50% lower cost for suitable non-interactive work
([Batch API](https://developers.openai.com/api/docs/guides/batch)). Prompt caching
reuses an unchanged prefix, reduces processing latency, and reports cached
tokens; reuse requires the rendered prefix to match and has minimum-length and
retention constraints ([Prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching)).

**Proposal:** group requests by stable prefix: system contract, target-language
rules, ABI schema, and shared type/symbol table first; put the function-specific
slice last. Use synchronous calls for repair decisions that need immediate build
feedback and Batch API for independent inventory, naming, and first-pass
generation. Keep model, tools, schemas, and ordering stable. Measure cache-hit
tokens rather than assuming a hit.

**Proposal:** use two explicit budgets: CPU/Ghidra concurrency and model request
concurrency. Start with `min(physical_cores, 4)` CPU workers and a model limit
set by provider quota; tune from measurements. Rust’s Rayon supports an explicit
thread count and work stealing ([ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html)).
Apply per-project and global semaphores, queue age limits, cancellation, and
back-pressure. Parallelism is safe only across independent graph components and
immutable records; Ghidra database mutation and final publication stay ordered.

## A target of 200,000 functions in one hour

This target means 55.56 successfully exported unique functions per second.
Counting failed attempts, duplicate addresses or assembly-only results as
completed pseudocode would misstate coverage. Initial analysis or snapshot
restore, output publication and retries must fit inside the hour if the claim
is end-to-end. Report cold and warm runs separately.

Local inspection of [`decompile_batch`](../src/worker/decompile_batch.rs)
shows a sequential `read_view` loop, up to sixty-four requested addresses but at
most four results per reply, with a byte limit that can make pages smaller.
Thus 200,000 functions require at least 50,000 MCP replies today, or an average
of 13.89 replies per second within an hour. Agent decisions between pages can
dominate the work. A batch command currently reduces tool overhead; it does not
create parallel decompiler execution.

The following are arithmetic budgets, not measured Prism throughput. They assume
all functions succeed and ignore loading, coordination, I/O and retries:

| Mean decompiler time per function | Serial time for 200,000 | Minimum ideal workers for one hour |
| --- | --- | --- |
| 50 ms | 2 h 47 min | 3 |
| 100 ms | 5 h 33 min | 6 |
| 500 ms | 27 h 47 min | 28 |

At 100 ms per function, eight fully effective workers would take 41 min 40 s
before overhead. These figures do not predict this corpus's average or long
tail. No 200,000-function end-to-end benchmark has established a one-hour result.
AI source reconstruction also adds model latency, integration and repair, so
this decompilation target cannot be transferred to reconstruction.

**Proposed implementation order:**

1. Add a persistent backend export job invoked through the batch MCP interface.
   The backend owns the address cursor, module output, manifest, checkpoints,
   progress and cancellation. Return a job identifier and bounded status rather
   than requiring the model to read every function. Keep small paginated reads
   for interactive inspection. This job is not an advertised Prism tool yet.
2. Reuse matching saved analysis and pseudocode, writing completed exports to
   durable, hashed artifacts. The bounded UI view cache is evictable and cannot
   be the sole completion record. Resume only verified outputs; preserve
   explicit failed and pending addresses. Stream output with back-pressure.
3. Introduce a CPU and memory bounded pool with private mutable decompiler state
   and a coordinator owning checkpoint publication. The current native snapshot
   takes an exclusive per-image lock, so simply spawning more existing workers
   will fail rather than provide safe concurrency. Profile restore, state
   sharing, per-worker memory and effective throughput before choosing a worker
   count. Retain per-function cleanup and orderly worker teardown.
4. Schedule bounded chunks dynamically so expensive functions do not hold up an
   entire static partition. Record timeouts and retry only unresolved work
   within an explicit budget. Report coverage as well as elapsed time; a timed
   partial export is not a complete decompilation.
5. For reconstruction, reuse these artifacts and submit coherent modules with
   compact shared interfaces and dependency fingerprints. Group mutually
   recursive functions and rebuild affected consumers. This reduces repeated
   context and repair without treating pseudocode export as completed source.

The pool design has a public precedent: Ghidra's
[DecompilerCallback](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/DecompilerCallback.html)
manages a pool of decompiler interfaces and supports per-function timeouts;
[ChunkingParallelDecompiler](https://ghidra.re/ghidra_docs/api/ghidra/app/decompiler/parallel/ChunkingParallelDecompiler.html)
processes groups of functions. Those are Java APIs, not concurrency currently
available in Prism's Rust backend. Adapting the ownership pattern is a proposal,
not a reason to replace the backend or assume its program state is thread-safe.
The modular, incremental scheduling proposal likewise follows the compact
summaries and independent backends documented by
[ThinLTO](https://clang.llvm.org/docs/ThinLTO.html), without implying that compiler
optimization and source reconstruction have the same work or correctness gates.

## What “10× faster” can and cannot mean

The 10× figure must be a measured end-to-end claim with a fixed corpus,
hardware, model, prompts, quality threshold, and cold/warm-cache definition.
Report wall-clock time, CPU time, model latency, token counts, cache-hit rate,
queue time, retries, accepted functions, compile rate, and behavior-validation
rate. Compare against a documented sequential baseline that uses the same
decompiler, target language, model, and acceptance policy. Include p50/p95 and
cost, not just the fastest run.

Use a stage model:

`T = T_inventory + T_graph + T_serial + T_parallel/P_eff + T_model + T_feedback + T_publish`.

The serial fraction bounds speedup. Amdahl’s original paper is the primary
source for this limit ([Amdahl, 1967](https://ieeexplore.ieee.org/document/4785615/)).
With serial fraction `s`, unbounded parallel speedup is at most `1/s`. If 20%
of end-to-end time is serial, the ceiling is 5×. Reaching 10× with any finite
parallel worker count requires `s < 10%` (the unbounded limit reaches exactly
10× only at `s = 10%`), and that still ignores queueing, rate limits, cache
misses, retries, and validation. Therefore
the honest target is “up to 10× on a stated corpus and warm-cache regime,” with
stage-level speedups and quality retained as separate acceptance measures.

## Limitations and decision

Universal reconstruction is bounded by stripped symbols, compiler optimization,
obfuscation, indirect control flow, missing runtime inputs, undefined behavior,
and the target language’s ABI/runtime. A model can invent a coherent API or
source structure where the binary does not determine one. Compilation and a
finite test set provide evidence for tested paths, not proof for all inputs.

The recommended Prism design is therefore generation-based but evidence-first:
function/SCC work units, minimal dependency context, immutable content-addressed
artifacts, stable-prefix prompt batching, bounded workers, compiler feedback,
and explicit behavioral-validation grades. Implementing these pieces can make
repeated runs much faster and can plausibly approach 10× on cache-friendly,
parallel corpora; the sources do not justify claiming 10× universally.
