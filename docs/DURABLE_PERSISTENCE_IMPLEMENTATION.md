# Durable persistence implementation

Updated 2026-10-19. This change follows the second-wave persistence work in
`REFERENCE_DECOMP_CRITICAL_PATH.md`. Measurement and application packaging belong to
the coordinator; this report makes no throughput or resident-memory claim.

The later feature-gated Reference provider adds bounded volatile batch windows as
described in `DURABLE_BATCH_WINDOW_IMPLEMENTATION.md`. The single-function and
no-new-buffer statements below describe this persistence slice and the existing
providers; the Reference integration retains one bounded page instead.

## Behavior

Cold durable pseudocode reads use an explicit `DurableResult` view-cache policy.
They can reuse compatible existing output, but do not compress and write a second
copy into the generic view cache before recording the durable result. Ordinary
strict reads and interactive cache writes retain their existing behavior.

Interactive pseudocode reads first use their exact configuration-specific generic
cache entry. If that entry is absent, they can reuse a compatible durable result
before trying the legacy upstream cache. Strict and durable reads retain their
durable-first lookup order. Durable lookup still matches the project SHA-256,
provider, serialized analysis configuration, and canonical function address. The
existing configuration includes the native engine identity and relevant tool,
memory, and timeout settings. Address aliases resolve to the same canonical
numeric address. Durable reads retain bounded decompression, digest validation,
and UTF-8 validation; invalid output is a cache miss.

During a retry or resume, a saved success is decompressed and hashed once rather
than twice. Verification, current-cursor validation, manifest-address validation,
and cursor advancement share one SQLite transaction. A missing or invalid artifact
returns a miss without advancing the cursor. Database and cursor errors still
propagate. Reusing a success does not rewrite its artifact, attempts, or result
counters. New results still commit their artifact, accounting, and cursor in the
same transaction.

The persisted schema, level-3 Zstandard encoding, SHA-256 digests, job identity,
and existing data remain compatible. The runner still executes one function at a
time, records each function separately, and keeps no new result buffer. All/filter
selection continues to stream the saved inventory; no whole-inventory copy was
introduced.

## Durable address admission

`MAX_DURABLE_ADDRESSES` is 16,384. The MCP catalog, protocol validation, and saved
selection parser use the same constant and count validator. This admits the
reported 2,251-function selection while keeping a finite explicit-list bound.
Admission still canonicalizes addresses, rejects numeric aliases as duplicates,
and checks every selected address against the saved index when creating the
frozen manifest. Deduplication stores numeric addresses rather than duplicate
address strings.

The complete request still has the existing 8 MiB transport ceiling, including
JSON overhead. A request can reach that ceiling before the item limit, especially
with long address strings. Each address remains limited to 1,024 characters by
the schema. For hundreds of thousands of functions, use `filter` or omit both
`addresses` and `filter`; these paths do not construct a large address request.
The separate interactive `procedures` limit remains 64, and bounded per-function
execution is unchanged. Invalid durable counts now report the supported range
instead of the generic `Invalid item count for addresses` error.

## Files

- `src/worker/analysis_cache.rs`: explicit cache policy, compatible durable lookup,
  and conditional generic-cache persistence.
- `src/worker/analysis.rs` and new `src/worker/analysis/view.rs`: policy-aware view
  reading and focused engine-view/text handling.
- `src/worker/decompilation_run.rs`: select the durable pseudocode reader.
- `src/decompilation/job.rs`: use the verified cursor-advance operation.
- `src/storage/decompilation/results.rs` and `results/record.rs`: remove the
  redundant success verifier and combine verification with cursor advancement.
- `src/mcp/decompilation_arguments.rs` and `src/mcp/mod.rs`: shared durable address
  bound and validation export between the library and desktop binary.
- `src/mcp/catalog/decompilation.rs` and `src/mcp/validation.rs`: advertise and
  validate the bound with the specific range error.
- `src/decompilation/identity.rs`: enforce the same bound and deduplicate numeric
  addresses.

No dependencies, schema migrations, tests, fixtures, or harnesses were added. This
slice does not change native checkpointing, worker scheduling, Stop handling, UI,
transport budgets, or live jobs.

## Limits and verification

Existing limits remain: 512 KiB raw pseudocode, 768 KiB serialized string, 4 KiB
error text, 2 MiB compressed artifact guard, 8 GiB compressed artifacts per job,
and the existing bounded Zstandard decoder. Resume verification retains its
existing 2 MiB decoded-artifact bound; interactive durable pseudocode lookup
retains its stricter 512 KiB bound.

The source was reviewed after formatting for cache precedence, identity, ownership,
bounded decoding, transaction boundaries, and failure propagation. No live batch
or proprietary analysis was run for this slice. Existing job metadata still
reloads after each record or skip, each skip still commits one cursor transaction,
and retry still walks the frozen manifest from the first failed item. These costs
remain for measurement and later work.

Checks executed during implementation:

- Scoped Rust formatting passed.
- `cargo check --locked --offline --no-default-features --bin prism-mcp` passed.
- `cargo check --locked --offline --bin prism` passed.
- A final physical-line scan of all 403 first-party Rust files under `src` passed the
  250-line ordinary, 100-line root, and 120-line main limits at that scan.
- `cargo clippy --locked --offline --bin prism --bin prism-mcp -- -D warnings`
  passed after the owners resolved concurrent bridge/UI warnings.
- `cargo fmt --all -- --check` passed after concurrent transcript formatting
  settled.

No repository `tools/check_architecture.py` or `tools/check_source_sizes.py` exists
in this Prism checkout, so those named checks could not run. No tests were written
or run. Runtime admission of the 2,251-item request and before/after measurements
are coordinator-owned checks, not results claimed by this report.
