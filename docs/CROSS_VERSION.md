# Cross-version binary intelligence

Open the destination version in Prism and select **Diff → Compare with previous
version…**. Choose a baseline executable, macOS application executable, or saved
`.prism` project. A saved project supplies its confirmed function labels, notes and
bookmarks. Comparison does not execute either application.

Review suggested pairs, their evidence method and changed-byte indicator. Duplicate
candidates appear as ambiguous groups with an explicit destination selector.
Possible additions/removals and extraction failures appear separately. Select pairs
before **Transfer reviewed labels, notes and bookmarks**. Conflicting destination
labels or notes reject the entire edit. Transfers use normal project persistence
and Undo; saving a `.prism` file retains both annotations and transfer provenance.

## Matching and evidence

The matcher is implemented in Rust using the existing `object` and `sha2`
dependencies. It does not launch BinDiff or require Ghidra's Java Version Tracking
service. REA supplies indexed procedures and instructions through the selected
analysis provider. Providers run sequentially; starting comparison pauses bulk
decompilation and closes the existing analysis session before opening the baseline.

Matching proceeds through unique exact byte hashes, normalized instruction
signatures, structural signatures, then up to eight rounds of already matched
callee anchors. Every stage proposes pairs for review. Names and addresses alone
never establish a correspondence. Duplicate groups remain compact rather than
materializing every possible pair. Unmatched entries are possible changes in
inventory, not proof that behavior was added or removed.

Evidence scopes remain explicit:

- `recovered_blocks`: file-backed block bytes and CFG successor ordinals from a
  provider that returns complete basic blocks.
- `listing_span`: file-backed bytes spanning the Rust REA instruction listing,
  with its final decoded instruction length. Direct branch destinations within
  the listing are normalized to instruction ordinals for structural matching.
  These are inventory-bounded listings, not recovered function bodies or CFGs.
  Padding and undiscovered functions can fall within the listing.

Comparisons require identical architecture, endianness and binary format.
Universal Mach-O selection follows the architecture attested by the provider;
ambiguous slices are rejected. Normalization preserves register and operand shape
but masks numeric values, including constants and addresses. Changed constants,
relocated references, indirect calls, compiler changes, inlining and missing loader
symbols all require investigation. Equal bytes do not prove equal runtime behavior.

Only confirmed function-level project annotations transfer. Instruction-relative
findings, patches, types and raw runtime evidence are not remapped. Transferred
notes remain discoveries to revalidate in the destination. Provenance records the
source/destination SHA-256 identities and reviewed address pairs in
`version-transfers.json`.

## MCP

`compare_versions({"baseline":"/absolute/path/to/previous.prism"})` compares the
open destination and returns a summary plus the first 100 suggested pairs.
`version_matches` accepts `kind` and `offset`; kinds are `matches`, `ambiguous`,
`source_only`, `target_only`, `source_unknown`, and `target_unknown`.

`ambiguous` returns one group at `offset`, counts, and at most 100 entries per side.
Use `source_offset` and `target_offset` to review additional candidates within that
group. Candidate pages include confirmed baseline labels, bookmarks and bounded
note previews. Reports expose evidence scope and limitations.

In a desktop window, `transfer_version_annotations` accepts the report's
`source_sha256`, `target_sha256`, and an explicitly reviewed `pairs` array of
`{"source":"0x…","target":"0x…"}`. Both hashes and a one-to-one candidate mapping
must validate. Headless sessions support comparison and inspection; annotation
transfer uses the desktop project's save-and-undo path.

## Resource limits

Each input image is capped at 256 MiB, with at most 20,000 indexed functions,
65,536 instructions and 1 MiB assembly per function, and a 64 MiB uncached assembly
work budget per image. Signatures use the existing compressed, budgeted SQLite
cache, keyed by file identity and provider configuration/runtime identity. Images
are hashed before and after capture. Transfer history is limited to 100 edits and
2 MiB; it counts toward the undo memory budget. UI and MCP collections are paged.

The Rust backend needs no decompilation for these signatures. Its listing boundary
limitations still apply. This is a conservative native first stage; full Ghidra
Version Tracking/BinDiff matching and semantic equivalence are not implemented.

## Verification

Deterministic tests cover moved functions, changed constants, call anchors,
duplicates, unknown extraction, relocated topology, incompatible evidence scopes,
architecture rejection, one-to-one review, atomic conflicts and saved provenance.
Run `python3 scripts/check-rust.py` for the repository feature matrix. The generated
source inventory records automated coverage, not a claim of manual review of the
entire repository. Provider and platform observations are recorded separately.

On 2026-10-26, `python3 scripts/check-rust.py` passed all 24 configured checks,
including all six comparison tests. Logs and the hashed source inventory are in
`build/rust-review/coverage.json` and its adjacent log files.

### Local provider observations, 2026-10-26

On macOS ARM64 with REA **6.7.0**, provider `ghidra-rust`, the live inspection
reported engine **2.1.0** and worker protocol **2**. Isolated support profiles were
used for the following read-only checks:

| Check | Observed result |
| --- | --- |
| Two `clang -O0` fixture versions | Three suggested pairs, including the relocated stable function; two changed-byte pairs; one duplicate ambiguity group; one possible addition and removal; zero unknown extractions |
| Universal Chrome launcher compared with itself | One exact listing-span match; zero changed-byte pairs or unknown extractions |
| `scripts/smoke-rea.py`, Chrome launcher | Indexed one loader function; bulk decompilation completed with pseudocode and provider warnings; clean shutdown |

The fixture comparison completed in about 2 seconds in this single local run.
This is an acceptance observation, not a performance benchmark. The Chrome
launcher inventory contains only one function, so it does not establish complete
application coverage. Raw results are in `build/cross-version-smoke/fixture.json`,
`universal.json` and `apps.json`; these are generated local artifacts. No Windows,
Linux, interactive UI or FFI soundness claim follows from these checks.

### Manual review scope

The new comparison modules and changed worker/UI/MCP integration paths were
reviewed against `rust-best-practices`, `rust-review` and the repository's resource
and readability rules, including their immediate save, cache and provider
lifecycle dependencies. The review corrected these issues before completion:

- **P1, evidence integrity:** the Rust provider has no recovered CFG or function
  body. Capture now distinguishes inventory listing spans from recovered-block
  evidence, rejects incomplete extraction, and reports the limitation.
- **P1, atomic edits:** validation precedes annotation copying and persistence;
  late conflicts leave the destination untouched. Save precedes publishing Undo
  state. Transfer provenance survives project archive serialization.
- **P2, resource ownership:** baseline cleanup preserves a primary capture failure,
  providers run sequentially, shared reports use `Arc`, signature caches are
  budgeted, and duplicate groups/pages avoid an unbounded pair cross product.
- **P2, readability:** matching phases, reviewed candidate validation, annotation
  copying, provenance recording and UI review controls have separate coherent
  operations. Checked address arithmetic guards file-backed span extraction.

No unresolved finding remains in this review scope. The review does not cover all
other concurrent changes in this working tree, prove semantic correspondence, or
replace platform and interactive acceptance testing.
