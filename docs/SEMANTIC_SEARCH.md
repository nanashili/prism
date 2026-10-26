# Function behavior search

Prism indexes saved evidence from its native Rust Ghidra backend and returns ranked
function leads with source excerpts. `index_function_search` and `semantic_search`
are available to desktop agents and MCP clients, including the Find skill.

## Workflow

1. Open a native binary using **Ghidra (Rust through REA)** and use its saved function
   inventory. Search does not discover new functions.
2. Fill relevant missing pseudocode with `decompile_batch`. Indexing itself never
   starts a decompilation or provider analysis call.
3. Call `index_function_search` with `offset: 0`, then follow `next_offset` until
   null. Each transaction refreshes at most 300 saved functions and stops after roughly
   8 MiB of serialized evidence. Refresh after
   changing notes, names or saved analysis evidence.
4. Call `semantic_search` with `query`, for example `Find functions that parse a
   binary header` or `Find code that implements an object pool`. `limit` defaults
   to 20 and accepts 1–100. An optional native `similar_to` address selects an
   indexed structural exemplar.
5. Inspect returned excerpts and the matching function before making a behavioral
   claim. Use Prism's evidence/claim workflow when reporting findings.

The index combines names, notes, descriptions, saved imports/callees, callers,
constants, control-flow summaries and durable/cached pseudocode. References and CFG
metadata contribute when already cached; missing metadata is not invented.
No JVM, Java script, Java Ghidra installation or external vector service is required.

## Retrieval and explanations

The text route tokenizes identifiers and prose, expands a small synonym vocabulary,
and recognizes two explicit patterns: reads/parsing with header-format checks, and
free-list access with allocation/reuse/next-link evidence. Names alone cannot trigger
these patterns. Inferences include their observed inputs and remain hypotheses.
Function declarations are excluded from body text so a misleading function name
does not gain a second, stronger contribution as decompiler evidence.
This is an evidence retrieval baseline, not a general learned text embedding model.
An arbitrary concept needs supporting terms or annotations to yield an initial lead.

The structural route is implemented entirely in Rust. It builds weighted syntax
trigrams from normalized pseudocode, ignoring identifier names, numeric values,
comments and literal contents. Saved CFG successor degrees and edge directions add
structure. Each feature uses a stable hash and log-scaled occurrence count; stored
vectors are normalized. This tolerates renamed variables and relocated addresses,
but similar syntax can implement different behavior.

The current native worker does not expose BSim signatures. These signatures are
**not BSim**, and do not claim BSim's data-flow analysis or significance scoring.
[Ghidra's BSim tutorial](https://github.com/NationalSecurityAgency/ghidra/blob/master/GhidraDocs/GhidraClass/BSim/BSimTutorial_Intro.md)
is the reference for a future compatible native implementation. There is no Java
fallback.

FTS retrieves at most 400 text candidates; per-field term coverage reranks them.
Notes/descriptions weigh 1.0, decompiler text 0.9, calls/constants 0.8, control flow
0.6, callers 0.5 and names 0.2. Repeated mentions do not multiply a term's score.
Up to three text leads scoring at least 0.2 seed structural retrieval. Candidates
need cosine at least 0.85 and three shared features. Structural score is
`0.8 * exemplar relevance * cosine`; the higher of text and structural scores wins.
Explicit exemplars have relevance 1.0. Each structural result identifies its exemplar,
cosine, shared features and route score. Scores are ranking heuristics, not confidence
probabilities. The route examines at most 400 matches per exemplar.

## Persistence and limits

Indexes are scoped by binary SHA-256 and the full analysis/decompiler configuration,
including native runtime identity. Searches join the current saved inventory so
removed functions disappear without waiting for a refresh. Responses report indexed
and inventory counts; indexed functions may still lack pseudocode or metadata.
Only one binary/configuration is searched, not a global cross-binary corpus.

Documents are limited to 512 KiB and signatures to 16,384 distinct features. Syntax
extraction considers at most 32,768 tokens. The database enforces a 256 MiB evidence
budget (serialized documents plus estimated vector postings); SQLite/FTS overhead is
additional. Failed or cancelled refreshes leave the previous committed page intact.
`index_function_search` with `reset: true` removes derived search data for the current
binary across configurations. It preserves source evidence, notes and inventory;
reindex to rebuild. Reset and offset cannot be combined.

Automated checks cover behavior ranking against misleading names, persistence,
normalized structural retrieval, renamed native bodies, transactional rollback,
cancellation, configuration/inventory isolation and literal query syntax. Provider
acceptance is separate from these deterministic checks.
All six search tests and `cargo clippy --all-targets --all-features --locked -- -D warnings`
passed on the final implementation. Manual review covered the search modules and
their integration points; the automated source inventory is not evidence of a
manual review of the entire worktree.

`python3 scripts/check-rust.py` completed all 24 checks. Its initial `prism-clippy`
check encountered concurrent worktree errors in `analysis_repair/tests.rs` and
`ui/investigation.rs`; the other 23 checks passed. After those files were corrected,
the all-targets/all-features lint retry above passed. The initial coverage report
retains that first failure; it has not been rewritten to imply a clean first run.

## Native acceptance, 2026-10-26

An isolated macOS ARM64 profile used REA 6.7.0 with its matching native Rust Ghidra
worker. A compiled fixture indexed four functions with three saved decompilations.
The header query retrieved its parser, and a pool-function exemplar retrieved the
second pool implementation with cosine 1.0 and 32 shared structural features.
Shutdown reported no cleanup failures. The separate `scripts/smoke-rea.py` check
also passed on the fixture and The Unarchiver, with three successful native
decompilations per target. These are MCP/provider observations, not GUI or
Windows/Linux acceptance results.

The matched executables were under `/tmp/prism-rea-rust-release/rea-rust/bin`:

| Executable | SHA-256 |
| --- | --- |
| `rea` | `22d71d01fca7ee33159ea5625706428d4da515a701d1191c866b2d39efe77188` |
| `rea-ghidra-rust-worker` | `175dc42ac921ee6f090231bb891a46a8a0fb40e976c5243195d5039cf07bd463` |
