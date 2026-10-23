# Prism snapshot patch

Vendored from https://github.com/koute/ghidra-decompiler at commit
`268c3a406d8d964bc00ae2ead4d3e1e1373aaa27` (2.1.0).
Only compiled sources, processor specifications, manifest and license notices are
included. Decompilation algorithms are unchanged.

The optional `snapshot` feature adds a serde representation and a narrow Program
save/restore API around the existing architecture encoder. Program-owned function
and data bookkeeping is restored by address with fresh native symbol IDs. Restore
initializes the loader/translator without loading symbols a second time. Images,
translator caches and temporary function analysis are not part of the snapshot.
Prism owns file identity, schema/version admission, compression, atomic replacement,
locking and checkpoint policy outside this dependency.

Generated symbol counters reserve all preserved global function IDs before
snapshot symbol decoding. Restored ordinary symbols receive new IDs above that
range, and inserting a preserved generated ID advances its scope's counter.
This prevents restored data or later explicit discovery from reusing a function
ID and causing a duplicate function-scope error. Allocation checks the 40-bit
counter boundary before mutation and retains the existing symbol cleanup path.
The compact XML index supplies the reservation in one linear scan without another
DOM or a traversal stack. Persisted IDs and references are not remapped; snapshots
that already contain duplicate function IDs require a separate reference-aware
repair and are not silently rewritten by this patch.

The scope decoder also accepts an omitted symbol list in an empty scope, matching
the existing encoder. This is needed to restore callee prototype scopes created
during flow discovery. Unexpected nonempty elements still fail validation.

The string-cache encoder closes each `string` element after its byte payload.
The upstream writer omitted these closing tags, making nonempty saved string
caches impossible to restore. Snapshot admission repairs only the complete legacy
shape with one byte payload per string and no closing string tags. It limits the
repair to the `stringmanage` section, leaves valid state unchanged, and keeps other
malformed state on the existing error path. Repair happens in memory; normal
atomic checkpointing writes the corrected format after subsequent analysis.

Memory-property range updates use a bounded mutable B-tree iterator after
splitting the existing start and exclusive-end partitions. The original lookup
cloned every subsequent partition key before filtering to the requested range.
Restoring many saved symbols therefore performed quadratic copying. Direct
iteration retains the same flag updates, exclusive end, and unbounded final
address-space behavior without allocating a temporary key list or revisiting
unrelated partitions. The saved-state schema and representation are unchanged.

Range indexes use fixed-size sorted blocks rather than one contiguous `Vec`.
Restore and normal range edits insert and remove entries at arbitrary sorted
positions; each block holds at most 1024 ranges and splits at 512, bounding
element moves while a compact cumulative-end directory preserves binary-search
lookups. Blocks start with ordinary empty-vector growth so tiny maps do not
reserve the full block size, and full blocks split before insertion to avoid a
temporary 2048-element allocation. The existing `usize` iterator facade,
ordering, and snapshot format remain unchanged. Directory updates remain
linear in the number of blocks; this is a container-only change, so serialized
snapshots and range semantics are unchanged.

## Compact saved-state XML restore

Snapshot restore now uses a compact source-range XML index under
`program/compact_xml.rs`, `program/compact_xml_parse.rs`, and
`program/compact_decode.rs`. It preserves the existing XML bytes and `Decoder`
operations while replacing the general `Element` DOM's per-node `Arc`, owned
name/content strings, and recursive child allocations with indexed nodes. The
snapshot path transfers the owned encoded buffer into the index and drops the
decoder before record restoration, so it does not retain a second XML copy or
the parser forest during later restore phases. The legacy DOM path remains
available for normal architecture/spec parsing. XML entities, CDATA,
attributes, child lookahead, skipping, depth validation, and malformed-input
errors are handled by the compact parser; no snapshot format change is
introduced.

The index uses 32-bit source offsets, contiguous attribute ranges and sibling
links. It rejects XML larger than 512 MiB and index capacities larger than
256 MiB, validates UTF-8/entities and XML characters, and caps nesting at 10,000.
These are parser admission limits, not a total process memory ceiling. The
optional `snapshot` feature now directly uses pinned `quick-xml` 0.41.0, already
present in Prism's dependency graph. XML formatting and serialized snapshot
identity remain unchanged; general architecture/spec parsing keeps the existing
DOM decoder. Valid snapshots from the existing native encoder remain supported;
malformed snapshots receive decoder errors rather than lossy text repair.

## Shared executable and inventory ownership

`ObjectArchitecture` and `ObjectLoadImage` share the same immutable executable
through `Arc<ImageBytes>`, avoiding a second full-size executable allocation
when the loader is built. Public byte-loading methods keep their existing `Vec`
signatures and can build loaders repeatedly. Parsed object symbols keep
validated name ranges into this shared image, materializing owned names only
during symbol enumeration.
ELF retains lossy UTF-8 decoding and generic object formats retain their existing
empty-name behavior for invalid UTF-8. The loader owns the buffer throughout the
ranges' lifetime and preserves its established lock ordering.

On Unix, the immutable `ImageBytes` may use a privately owned anonymous-file
spool mapped read-only for the native loader. The spool's ownership remains
private to the image and avoids retaining the executable as a large heap
allocation. This is a read-only mapping design; it does not claim `MAP_PRIVATE`.
Windows uses the owned-buffer fallback because the Unix spool path is unavailable
there. Existing `Vec` byte-loading APIs and the snapshot schema remain unchanged;
the mapping is an ownership detail at the image boundary and does not alter
loader or snapshot semantics.

A compact `FunctionHandle` inventory borrows function names from the live Program
when rendering pages. Handles carry a Program identity token and validate symbol
existence, function kind and address before exposing a borrowed `FunctionEntryRef`.
The existing owned `Program::functions` API remains available. Prism retains the
compact handles, stable address ordering and first-alias deduplication, and
propagates stale-handle failures instead of substituting missing names. This
reduces duplicate strings without changing the saved-state schema or the
underlying decompilation algorithms. The duplicate current-name field was
removed: compact `FunctionRecord` values resolve the current display name from
the symbol table instead of copying a name into each record. Reconstruction
evidence, source and MCP payloads are borrowed from their stored objects while
consumed, and hashing streams those bytes instead of creating temporary copies.
These are ownership and access-path changes only; no measured throughput or
memory reduction is claimed here.

## Streaming snapshot encoding and focused string collection

The optional snapshot API also exposes a borrowed `ProgramSnapshotView`.
Architecture XML is written to a caller-owned encoder, and serde borrows the
existing function/data names. A compact address/SymbolId vector preserves the
owned snapshot's address ordering and last-ID alias selection without cloning
records. The owned `ProgramSnapshot` representation and save/restore API remain
available. This preserves schema-1 field values and permits existing checkpoints
to be read without migration; Prism continues to own identity admission,
compression, locking, save scheduling, and atomic file recovery.

`XmlEncode` shares one XML implementation between its traditional owned buffer
and a generic I/O sink. XML entities are yielded as borrowed fragments. The
streaming sink retains its first I/O error because the existing Encoder trait
has no error return; callers must check `finish()` as well as architecture
encoding. Finish does not flush the sink. Prism explicitly handles JSON escaping,
buffer completion, zstd finalization, and partial-file failure before publishing
the save. This avoids the owned whole XML/metadata snapshot during native saves,
but still traverses the entire architecture and uses the compact function index,
encoder internals, and compression memory. It is not an incremental checkpoint
format or a fixed process-memory ceiling.

String enrichment retains the same raw-flow setup, error behavior, analysis
clearing, and string registration order. Its collector now keeps only constants
inside string sections and does not collect the unused call-target list or body
range. Discovery continues to collect full flow facts. The raw pass and later
full decompilation remain; no p-code cache or semantic string-removal shortcut is
introduced. Runtime performance and saved-state restart verification are separate
from successful format/build checks; no throughput or peak-memory claim is made.

## Numeric analysis storage observations

`Funcdata::analysis_storage_counts` reports live values and monotonically assigned
arena slots for operations, varnodes, blocks, high variables, variable groups, and
call specifications. The accessor only reads existing counters. It does not
change arena representation, reuse IDs, mutate analysis, or expose code text.

Prism uses these counts only for opt-in decompilation diagnostics, after the
Program clears analysis and before the function is released. Allocated slots
therefore describe work history retained by those arenas, including cleared or
removed values and regenerated graphs. They are neither function body sizes nor
live byte counts. Existing action statistics supply bounded selected rewrite
counters alongside these observations; diagnostic output does not alter the
decompiler's transformations or their ordering.
