# Streaming native checkpoints

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Implemented 2026-10-19 in `src/analysis/native_ghidra/checkpoint/` and the
vendored decompiler's snapshot/XML adapters. This is a save-path ownership change;
the existing reader and admission policy remain in use.

## Representation and compatibility

The checkpoint retains schema 1, the existing engine `CACHE_ID`, exact image
SHA-256 binding, language/description fields, and every saved program, discovery,
call-index, automatic-discovery, and seeded-address field. It uses the same
zstd level 3 with checksums and 512 MiB uncompressed JSON limit. Existing saved
state needs no migration or deletion. The owned `Program::snapshot()` API remains
available; Prism saves through the new borrowed `Program::snapshot_view()` API.

Architecture XML uses one encoder implementation for both owned and streaming
sinks. Its element state, numeric formats, XML entities, indexed attributes, and
formatting rules are retained. The JSON string writer escapes quotes,
backslashes, and all control bytes while passing UTF-8 through unchanged.
Metadata still uses `serde_json`, preserving its existing serialization behavior.
Function metadata remains ordered by address and keeps the greatest SymbolId at
an aliased address, matching the old BTreeMap collection. Data ordering and all
field names remain unchanged. Compressed bytes need not be identical.

## Ownership and remaining costs

The single native Program stays the sole mutable owner. A snapshot view borrows
its state and names until writing finishes. Saves no longer construct an owned
whole-program XML string or cloned function/data records. XML fragments pass
through JSON escaping into a 64 KiB buffer, the size limiter, and zstd.

The view retains a compact `(address, SymbolId)` vector to preserve ordering and
alias selection. Readiness validation still obtains the architecture's existing
temporary symbol-ID list. These allocations scale with function count; this is
not a fixed total-memory guarantee. The live architecture, its internal encoder
work, and zstd also consume memory.

Saves still traverse the whole durable architecture, compress it, and synchronize
storage before the response continues. No asynchronous copy, incremental log,
replay protocol, or second mutable Program is introduced. The dirty flag,
ten-second normal gate, forced-save behavior, and exclusive same-image lock are
unchanged. Removing the full synchronous save requires a separately specified
durability/recovery contract.

## Completion and recovery

The XML Encoder trait has no fallible methods, so the streaming implementation
retains its first I/O failure and suppresses further writes. Its explicit
`finish()` returns that error. Prism checks both architecture encoding and XML
finish, preserving both failures when they occur together.

After successful serialization, Prism explicitly flushes and consumes the buffer,
checks its recovered pending bytes, and checks zstd `finish()`. On serialization
or flush failure it consumes the buffer without a destructor retry and abandons
the incomplete stream. A recovered writer-panic error is attached to the original
failure. A failed save leaves the dirty flag and successful-save timestamp intact.

The unchanged atomic writer then synchronizes the completed temporary file,
renames it, and synchronizes the parent directory on Unix. Preparation failures
remove the incomplete temporary file and report cleanup failures. Rename failures
retain the completed temporary file when possible and report its recovery path.
A directory-sync failure is reported even though replacement may already have
occurred. Previously committed state is never proactively removed.

## String enrichment

The string prepass still follows the same raw flow and clears temporary analysis
on the same success/follow-flow-error paths. Its collector now retains only sorted,
deduplicated constants inside string sections. It skips body-range bookkeeping,
call-target collection, and storage of irrelevant constants. Full discovery keeps
the complete FlowFacts collector. String creation and subsequent full analysis
remain unchanged. This change does not remove the repeated flow pass or retain
p-code across functions; its performance effect requires runtime measurement.

## Production observation and verification

Setting `PRISM_PROFILE_CHECKPOINTS` emits successful-save counts/timings to stderr:
`bytes`, `stream_ms`, `finish_ms`, `atomic_overhead_ms`, and `total_ms`.
`bytes` is uncompressed JSON written. Streaming time includes view preparation,
XML/metadata serialization, compression writes, and buffer flush. Finish time is
zstd finalization. Atomic overhead is the remaining elapsed time, including
temporary-file setup, file/directory sync, and replacement; it is not a pure fsync
measurement. No addresses, paths, names, or saved content are logged. The variable
only enables observation; normal save behavior needs no configuration.

Existing validation executed for this slice: `cargo check --locked --offline -j 2`,
`cargo fmt --all -- --check`, and
`cargo clippy --locked --offline --all-targets -j 2 -- -D warnings` passed.
Direct line counts put the three native
checkpoint files below 250 lines. This checkout has no `tools/check_architecture.py`
or `tools/check_source_sizes.py`. No new tests, fixtures, or harnesses were authored,
and no proprietary checkpoints or binary outputs were inspected. Runtime restart,
old/new saved-state compatibility, artifact digest comparison, peak memory, and
throughput measurements are owned by the coordinator and are not established by
these source checks.
