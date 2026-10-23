# Fast saved-analysis restore research

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Status: design research for Prism, with local source inspection on 2026-10-19.
This note uses the current Prism worktree and the original Reference/REA source as
primary local evidence, plus official Ghidra API/source documentation. It does
not use proprietary binary payloads or raw decompiler output. A source fact is
labelled as observed; a design recommendation is labelled as proposed. No
runtime claim in this note means “instant” without a cold/warm measurement.

## The two different restore problems

Prism has a cache-first application path and a separate native-engine restore
path. A matching UI or MCP view can avoid native restoration entirely; its
actual display latency still needs measurement. Reference follows the same
boundary: `Rea.open` shares one pending
open per path/provider, while `Rea.code` and `Rea.analyze` read the per-function
cache before calling `ensureOpen` (original Reference
`app/src/engine/rea.ts#L594-L630`).
That fast path is different from reopening an engine that must reconstruct its
in-memory index before an uncached function can be served.

The original REA Ghidra path stores a completed native database as a packed
Ghidra archive. The cache script calls `Program.saveToPackedFile` only after
analysis has completed and publishes an archive plus an identity manifest
atomically (`ReaGhidraAnalysisCache.java#L73-L148`).
On a hit, headless import receives that archive and the bridge disables the
normal analyzer path with `-noanalysis` (`GhidraHeadlessArguments.ts#L187-L242`).
Ghidra's official API describes packed databases as serialized databases that
can be reopened through `PackedDatabase.getPackedDatabase` and `open`
([PackedDatabase API](https://ghidra.re/ghidra_docs/api/ghidra/framework/store/db/PackedDatabase.html)).
The official `GhidraProject.saveAsPackedFile` API likewise defines a packed
program snapshot as a serialized program, not a source-level reconstruction
([GhidraProject API](https://ghidra.re/ghidra_docs/api/ghidra/base/project/GhidraProject.html)).

Prism instead restores a compressed JSON envelope. The envelope is bounded and
identity checked (`SCHEMA`, engine ID, image SHA, inventory limits) before it is
accepted (`src/analysis/native_ghidra/checkpoint.rs#L12-L115`).
`Session::open` then moves the decoded `ProgramSnapshot` and the owned
`Vec<u8>` image buffer into `Program::from_snapshot`
(`src/analysis/native_ghidra/open.rs#L22-L63`).
That restore parses one large architecture XML string, rebuilds native symbols,
and resolves saved records by address
([`vendor/ghidra-decompiler/src/program/snapshot.rs#L87-L143`](../vendor/ghidra-decompiler/src/program/snapshot.rs#L87-L143)).
It deliberately restores durable analysis state, not temporary decompiler
translator caches. Therefore first-query latency after restore remains an
unmeasured property even when discovery itself is not repeated.

The coordinator-supplied Prism measurement is a completed restore reply in 18.536 seconds
with a 1,062 MiB peak worker RSS and an unchanged checkpoint hash. It is one
warm local measurement, not a universal baseline. The currently sampled hot
path is `RangeMap::insert_hint`: the index stores sorted ranges in one `Vec`
and each insertion can memmove later elements
([`vendor/ghidra-decompiler/src/rangemap.rs#L87-L151`](../vendor/ghidra-decompiler/src/rangemap.rs#L87-L151)).
The previous `property_keys` range-copy issue was fixed in the worktree; it is
not evidence that all restore cost has been removed.

## What should be optimized first

1. **Make the interactive hit independent of engine restore.** Continue to
   serve saved procedure lists, pseudocode, assembly and dossiers from the
   existing provider/view/address-keyed cache. Show a session as “opening” in the
   background only when a miss requires the native engine. Do not claim the
   native process is ready until its restore future resolves. This preserves
   Reference's observed cache-first behavior while avoiding a false instant label.

2. **Replace the JSON/XML cold path with a binary, versioned snapshot.** Keep
   image and engine identity, schema version, checksums, and crash-safe
   publication, but encode architecture records and address indexes in a
   length-delimited binary format. Store large immutable sections separately so
   opening the file can validate a small manifest and map or stream only the
   section needed for the first request. A packed archive is a good persistence
   target, but copying and decompressing the entire archive into a temporary
   database must be measured; it is not automatically zero-copy.

3. **Build indexes once, then serialize their query form.** During checkpoint
   creation, produce a sorted symbol/range section and a compact lookup index.
   Restore should load the index directly rather than call thousands of ordered
   `Vec::insert` operations. A one-time bulk sort is a candidate for the
   read-mostly path, but feasibility is blocked until `RangeMap`'s `zip`,
   `unzip`, removal and live queries during decode are audited for ordering and
   mutation assumptions. Any later edits can use a small delta index,
   periodically merged into a new immutable generation.

4. **If a mutable index is required, use bounded blocks, not one giant Vec.**
   A private block sequence with a fixed target size (for example, 256 to 512
   ranges) can bound local split/move work, but prefix-rank directories must be
   updated and searched, adding directory maintenance and lookup costs. There
   is no optimal complexity claim until those costs are measured. Keep
   `PartIterator = usize` as a compatibility facade only if every existing
   query, remove, zip and unzip operation is audited. A container/layout-only
   change need not migrate checkpoints; a new binary snapshot format does, and
   should rebuild from records under an explicit format version. Neither change
   may alter ordering or symbol identity without a compatibility check.

5. **Measure initialization ownership separately.** `ObjectArchitecture`
   clones its image into the loader during initialization, but `from_snapshot`
   already clears `program.architecture.builder` before parsing XML
   ([`snapshot.rs`](../vendor/ghidra-decompiler/src/program/snapshot.rs#L87-L109),
   [`build_loader`](../vendor/ghidra-decompiler/src/object_arch.rs#L842-L867)).
   The original `open` buffer is moved, not retained as an independent copy.
   A shared immutable image could remove the transient initialization clone,
   but it cannot be credited with eliminating duplicate bytes throughout the
   entire restore. Measure initialization and XML/index peaks separately before
   prioritizing a new image ownership API.

6. **Bound restore work and expose progress phases.** Validate the manifest,
   map/stream the required sections, construct the query indexes, and publish
   the session as separate phases. Cancellation must leave the prior checkpoint
   intact. The existing atomic writer and “preserve invalid saved file” policy
   are good safety boundaries and should remain.

## Design comparison

| Design | Restore complexity | Memory behavior | Main risk | Recommendation |
| --- | --- | --- | --- | --- |
| Keep JSON/XML and tune allocations | O(records + XML parse + index inserts) | High transient strings and maps | Does not remove sorted-vector insertion | Short-term only |
| Rebuild sorted indexes from records | O(records log records) once, O(log n) query is a candidate, not a measured guarantee | One build buffer, compact final index | Must preserve range tie order and native IDs; decode-time query audit required | Candidate durable format |
| Bounded mutable blocks | Local moves may be bounded, but prefix-rank directory updates/search add costs; no final bound claimed | Fragmentation and block metadata | Every iterator mutation needs audit | Fallback if live edits need it |
| Packed Ghidra database | Provider-defined database restore | Potentially shared/cached on disk | Java/Ghidra version and process overhead | Reuse as a provider option, not Rust wire format |
| Memory-mapped immutable sections | Near O(index pages touched) after map | Low duplicate memory for read-only hits | Platform mapping, validation and edits | Add after binary format is stable |

The immediate bulk-build optimization is a candidate only. Before changing
the current snapshot restore, read the complete `RangeMap` mutation/query
surface, including `zip`, `unzip`, removal and queries that may run during
decode; then compare the old and bulk-loaded ordering. After that audit, an
immutable binary generation with a separate delta index is a longer-term
candidate. Jumping directly to blocks or memory mapping before preserving range
semantics risks trading one slow restore for difficult corruption and migration
failures.

## Correctness and acceptance criteria

These are proposed checks for implementation, not new authored tests. Existing
validation and manual measurements should verify them:

- A matching image, engine, schema, language and analyzer identity opens with
  the same function addresses, symbol names, data ranges, sections, call index
  and discovery metadata as the current snapshot.
- Range queries, `zip`, `unzip`, removal, and address ordering return the same
  sequence before and after restore, including overlapping and equal-endpoint
  ranges. Native symbol IDs may be remapped, but saved addresses must resolve
  to the same logical records.
- A crash or cancellation before manifest publication leaves the previous
  checkpoint usable. A checksum or identity failure never replaces it with a
  partial generation.
- Cold, warm, and first-uncached-function timings are recorded separately.
  Report wall time, CPU time, peak process-family RSS, decoded bytes, mapped
  bytes, and cache hit/miss state. The current 18.536 s / 1,062 MiB run is the
  baseline to beat, not a target that can be silently redefined.
- The interactive cache hit remains available while native restore is still in
  progress. A cache miss reports a bounded opening phase and never pretends to
  be a completed decompile.

An “instant” experience can be achieved for cached views by avoiding the native
restore on that path. An instant native engine restore for every binary is not
established by the Reference source or the Ghidra API. The original system
demonstrates a useful persistence boundary, while Prism must still measure its
own binary index and process startup costs.
