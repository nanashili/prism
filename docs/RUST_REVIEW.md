# Rust review: Prism

Reviewed on 25 October 2026 against `rust-review`, `rust-best-practices`, and the
project's readability and recovery rules. Prism is an application with a public MCP
library; application internals use concrete types and application errors, while the MCP
and vendored Markdown APIs receive the stricter library checks. Prism uses edition 2024
with MSRV 1.93 and a pinned 1.93.1 toolchain. The Markdown crate uses edition 2021 with
MSRV 1.89. The correctness and resource-lifetime findings below have been repaired.
Formatting, strict Clippy, tests, and rustdoc were run for both crates and the supported
feature combinations.

The standards in [AGENTS.md](../AGENTS.md) apply to every maintained project file.
Automated checks inventory every maintained Rust file, including build scripts and
vendored sources, and parse every maintained Python script. Manual review concentrated
on unsafe operations, process ownership, shutdown, persistence, input boundaries,
reconstruction, and the modified APIs and their callers. The hashed inventory records
automated coverage; it does not certify a manual line-by-line audit of every file.
Licenses and third-party attribution remain intact.

## Must fix

### 1. Separate process shutdown from destruction — fixed (`src/protocol/process/managed.rs:38`)

**Rule:** Destructors must not block or fail; resource ownership must survive partial
failure (`rust-review`, Correctness and soundness; project recovery rules).

**Problem:** Protocol exchanges and reader joins during destruction could hang the UI.
A failed stop could also discard ownership before a child was reaped.

**Fix:** `Managed` and `Capture` own the child immediately after spawn. Explicit close
paths report independent cleanup failures. Destruction signals and hands ownership to
a reaper started before spawning. The reaper retains failed stops for retry and joins
only finished readers. Agent, bridge, worker, and catalog teardown follow the same
explicit-close boundary.

```rust
if let Some(child) = self.child.take() {
    self.reaper
        .retire(child, Readers::Rpc(std::mem::take(&mut self.readers)));
}
```

Regression tests cover a blocked output reader during destruction and a failed reader
during explicit close.

### 2. Preserve DFS finishing order — fixed (`src/reconstruction/scc.rs:24`)

**Rule:** Algorithms must preserve their graph invariants (`rust-review`, Correctness
and soundness).

**Problem:** Marking siblings visited while pushing them onto the DFS stack could
classify the acyclic edges `A -> B`, `A -> C`, `B -> C` as a cycle. Reconstruction could
then group independent modules incorrectly.

**Fix:** Mark a node when entering it and record its finishing order only after its
descendants. Regression tests distinguish this DAG from a real cycle with a dependant.

```rust
if !seen.insert(node.clone()) {
    continue;
}
stack.push((node.clone(), true));
```

### 3. Avoid overflow in archive limits — fixed (`src/project/archive.rs:148`)

**Rule:** Avoid integer overflow on external-input paths (`rust-review`, Correctness
and soundness; `clippy::cast_possible_truncation` review).

**Problem:** Adding one to a caller-supplied maximum read limit overflowed at
`u64::MAX`. Archive decoding could panic or incorrectly report an empty input.

**Fix:** Use saturating arithmetic and retain the decompressed-size check. The
regression covers zero, exact-size, and maximum limits.

```rust
decoder.take(limit.saturating_add(1)).read_to_end(&mut output)?;
```

### 4. Retain completed file recovery data and causes — fixed (`src/storage/atomic.rs:29`)

**Rule:** Errors retain their cause (`C-GOOD-ERR`, Rust API Guidelines); do not discard
recovery data before successful publication (project recovery rules).

**Problem:** Failed publication could hide the original filesystem error, while a
subsequent temporary-file retention failure could remove the completed recovery file.

**Fix:** Disable automatic cleanup only after preparation and sync succeed. On failed
publication, retain the completed file and report its path, the original cause, and any
retention error. Preparation failures still remove incomplete output and preserve the
destination. Both failure paths have filesystem regression tests.

```rust
temporary.disable_cleanup(true);
let Err(failure) = temporary.persist(destination) else {
    return Ok(());
};
```

## Should fix

### 5. Make shared API choices and public errors explicit — fixed (`src/mcp_error.rs:6`)

**Rule:** Concrete public errors with sources (`C-GOOD-ERR`), public `Debug` and
documentation, and no Boolean mode parameters in shared signatures
(`rust-best-practices`, Types and API design).

**Problem:** String/boxed public errors concealed the error contract, and Boolean
parameters made storage, rendering, setup, and reconstruction calls ambiguous.

**Fix:** The MCP library exports a concrete `Error` preserving its source. Public
functions document errors and return values; extensible public choice enums are
non-exhaustive. Shared APIs now use domain choices, including the vendored renderer.
Mutable Boolean widget data and the standard `From<bool>` conversion trait remain
appropriate Boolean boundaries.

```rust
database.start(store, project, &config, GenerationStart::Resume)?;
viewer.responsive_tables(TableLayout::Responsive);
```

### 6. Give transaction and UI phases clear owners — fixed (`src/reconstruction/generation.rs:20`)

**Rule:** Separate responsibilities and use named state instead of positional tuples
(`rust-best-practices`, Types and API design; project readability rules).

**Problem:** Generation startup mixed validation, a large positional snapshot,
resumption, insertion, reuse, and commit. The UI event dispatcher also contained the
complete workspace-open and content-delivery procedures.

**Fix:** `PreviousGeneration` owns the named snapshot and its fork/reuse operations.
Startup now exposes the transaction phases directly. Workspace events have dedicated
handlers in `src/ui/receive/workspace.rs`. Tests verify resumption identity and that a
failed fork preserves the parent and can be retried successfully.

```rust
let previous = PreviousGeneration::load(&tx)?;
let id = insert_generation(&tx, previous.as_ref(), project, &fingerprint, config)?;
```

### 7. Name three-state results — fixed (`src/model/goal.rs:25`)

**Rule:** Make invalid states unrepresentable and name domain distinctions
(`rust-best-practices`, Model the domain with types; `clippy::option_option`).

**Problem:** Nested options obscured the difference between missing metadata, explicit
removal, and replacement; module boundaries and pending permission answers had similar
ambiguity.

**Fix:** `GoalUpdate`, `ModuleMarker`, and `PermissionChoice` name these outcomes. The
metadata wire format and permission response protocol remain the same.

```rust
pub(crate) enum GoalUpdate {
    Unchanged,
    Clear,
    Set(Goal),
}
```

## Consider

The separate pedantic pass was reviewed by diagnostic group. UI coordinate and progress
conversions intentionally narrow to the renderer's float types. Owned callback and
thread inputs intentionally carry ownership; independent stored preferences remain
Boolean data. Long declarative tables and event dispatch matches are not split solely
to satisfy a line-count threshold. These diagnostics do not justify blanket lint
suppression or speculative abstractions.

Validation ran on arm64 macOS. Windows and Linux platform branches still require their
native CI environments. The Markdown crate's `nightly` and `dump-macro` development
modes are excluded from the stable feature matrix. The unsafe review checked local
invariants and ownership comments; neither Miri nor a formal FFI soundness proof was run.

## What's already good

- Process output is bounded while pipes continue draining, so display limits do not
  deadlock the helper.
- SQLite transactions keep reconstruction changes atomic; rollback tests exercise an
  actual failed database write.
- Profile locking prevents concurrent writers and is released through file ownership.
- Native REA selection validates executable identity and matching worker provenance;
  acceptance checks use an isolated profile and saved decompilation results.

## Reproduce and evidence

```sh
python3 scripts/check-rust.py
```

The runner checks the full application, MCP without default features, desktop without
the native-provider feature, the Markdown parser feature, and every stable additive
Markdown feature. Each combination runs formatting, Clippy, tests, and rustdoc: 20
checks in total. Warnings are errors in tooling, with the root manifest providing the
shared lint policy. There is no source-level `deny(warnings)`.

The application tests pass 29 cases with one ignored network integration case. The complete
stable Markdown feature set passes 97 unit tests and 16 documentation examples, with
one ignored example. Tests repeated across feature combinations are not counted as
additional independent cases. Per-command logs and source hashes are written to
`build/rust-review/coverage.json` and adjacent logs.

Native application acceptance and runtime provenance are recorded separately in
[the REA integration check](REA_NATIVE_SMOKE_20261025.md).
The Python harness also checks the process exit status after shutdown. A real
nonzero-exit process reproduced the missing failure report before repair and was
correctly rejected afterward.

## Settings consistency follow-up: 26 October 2026

Reviewed the application UI changes in Skills, Workspace, and the shared settings
controls against both Rust skills and the project readability rules. No outstanding
must-fix or should-fix code findings remain in this change.

- Skills and Workspace reuse the settings card, row geometry, input, button, and toggle
  primitives, keeping layout responsibilities in the shared renderer.
- Library rendering is separated from download, import, and removal operations. An
  `Action` enum defers execution until rendering finishes; worker ownership and error
  reporting remain in the library runtime.
- Skill toggles use the existing settings command path. Workspace preserves the final
  selected category and retains the setup screen's tile picker.

All 20 required checks passed, with matching hashes for 449 maintained Rust files.
The rebuilt macOS app rendered Skills, Workspace, General, and Appearance without a
reported UI error. A separate profile also exercised the installed-library layout at
the smallest supported interface size. Screenshots are in
`build/settings-pages-20261026/`; these verify rendering, while download and file-picker
actions were not exercised in this layout pass.

## Keyboard sizing follow-up: 26 October 2026

Reviewed the keyboard sizing change as application code against both Rust skills and
the project readability rules. No outstanding must-fix or should-fix findings remain.
The keyboard page has a preferred width independent of the other settings pages.
Key geometry carries one scale through layout, painting, labels, icons, and pointer
bounds, preserving natural proportions when less space is available. Wide pages stop
enlarging the keyboard at its preferred size.

All 20 required checks passed, with matching hashes for 449 maintained Rust files.
The existing keyboard interaction test passed with modifier changes, presets, custom
bindings, and key selection. The rebuilt macOS app rendered the complete keyboard at
the default interface size and at 150 percent in a separate profile, which reduces
the available layout width. Both screenshots show square letter keys and complete,
aligned rows. Evidence is in `build/keyboard-proportions-20261026/`; native clicking
was not exercised in this rendering pass.
