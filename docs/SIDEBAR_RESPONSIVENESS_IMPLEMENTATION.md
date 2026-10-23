# Sidebar responsiveness implementation

Implemented on 2026-10-19 against the native Prism source backed up under
`/tmp/prism-responsiveness-20261019/source-before`. This note covers the UI slice;
worker, transport, cancellation and persistence work have separate owners.

## Rendering boundary

`chat/transcript.rs` owns the active transcript renderer. The existing outer
egui scroll area uses `show_viewport`. It creates row metadata for retained
messages, then renders rows intersecting the viewport plus 128 points of
overscan above and below. Hidden rows contribute estimated or previously
measured heights; their markdown, activity disclosures and user bubbles are
not rendered. A message intersecting this region is rendered completely.
There is no nested scroll area per message. The existing shared composer,
permission, question and queue cards remain outside this scroll area.

Live rows receive monotonically assigned identities in `Chat`. Front history
trimming removes the corresponding identities. Saved-message IDs are not used
for live row identity, so saving a new message does not change its widget ID.
The scroll-area identity includes the tab and history page. Unsaved tabs have
independent scroll state. At most eight pages retain lightweight height
metadata; parsed markdown is held only for the active page.

Each visible row is measured on every draw. This observes streamed text,
activity updates, disclosure toggles, activity pagination, full-message
disclosure and image size changes. Width, font definitions, font sizes,
spacing, pixel scale, theme mode and thinking visibility invalidate height
measurements. Unmeasured rows start from a cheap text-length estimate, capped
at the existing 16,000-byte message preview length.

Before invalidation or front trimming, the renderer captures the first visible
row and the pixel offset within that row. Height changes restore this anchor,
clamped to the new row height and total scroll range. If trimming removes the
anchor itself, the view starts at the first retained row. A live transcript already
at the bottom follows the tail unless an upward scroll is in progress. egui
continues to own wheel, momentum and scrollbar input. Geometry compensation is
queued through the outer scroll area's immediate scroll-delta path before
native input and final clamping. This avoids applying a second correction to
an old offset already clamped by a shrinking transcript. Programmatic scrolling
in this area is immediate; native wheel and momentum behavior remains in egui.
Measurement changes
request one discard on the first egui pass and a repaint, allowing geometry
to settle without permanently drawing every message.

## Markdown cache ownership and admission

`message/body_cache.rs` owns one cache for the active transcript, shared by
reply, user, thought and subagent bodies. Entries use stable UI identity and
exact source equality. A changed streamed body replaces that body's backend
cache. Stable bodies reuse parsed markdown events. Page or tab changes drop
all parsed bodies; rows removed by history trimming are pruned. The empty
workbench also releases parsed bodies.

Admission bounds are explicit:

| Retained resource | Bound |
| --- | --- |
| Backend cache instances | 32 across the active transcript |
| Original source admitted per body | 64 KiB |
| Owned original plus normalized source across entries | 256 KiB |
| Height metadata pages | 8 |

The source ledger counts both the exact original source and the optional copy
with fenced-code language hints removed. Least recently drawn entries are
evicted before an admission would exceed either aggregate limit. Full bodies
over 64 KiB use a temporary backend cache. The existing 16,000-byte preview
and full-message disclosure behavior are preserved.

These bounds are **not a backend heap or process RSS cap**. The dependency does
not expose byte accounting for its private event vectors, syntax layout maps,
galleys, image metadata, hash-table capacity or shared font/image/texture
storage. Source admission limits retained work, but those additional bytes and
temporary parsing/layout allocations remain unmeasured. A narrower retained
heap guarantee would require a maintained accounting boundary in that backend.

Layout changes preserve the cached parse events while replacing the backend
cache's derived state, avoiding retention of multiple old layout variants in
each admitted body. Syntax-hint setting changes replace the body cache because
the rendered markdown source changes. The normal commonmark viewer still
hashes the source and clones the cached event vector on every visible draw;
this implementation removes repeated stable-body parsing, not all draw-time
allocations or layout work. No dependencies or vendor sources were changed.

## Existing behavior and remaining validation

The normal markdown viewer, image URI policy, link handling, selectable text,
context-menu copy of the complete message, activity paging and full-message
disclosure remain in their original rendering paths. Cache ownership is
passed explicitly through those paths. Activity paging moved into the focused
`activity_view/steps.rs` module to keep these interfaces readable.

The GUI was not opened or restarted during this implementation. Interactive
scroll anchoring, tail following, selection across virtualized row boundaries,
image load transitions and resize/disclosure combinations still need an
observed native UI pass. Hidden geometry is approximate until visited. Large
single expanded messages can still dominate a frame. Cache churn is possible
when more than 32 body widgets are visible, or aggregate source exceeds the
admission budget. These are known limits, not measured latency results.

Validation observed for the transcript/cache slice, before the model-menu
follow-up below:

- `cargo fmt --all -- --check` passed.
- `cargo check -p prism` passed with default desktop features.
- `cargo clippy -p prism -- -D warnings` passed.
- At this validation point, all changed Rust source files were below 250
  physical lines; the largest was `src/ui/app.rs` at 226 lines. No crate roots
  were expanded.
- Prism contains neither `tools/check_architecture.py` nor
  `tools/check_source_sizes.py`; the OpenUnity repository checkers are not
  Prism validation targets.
- No tests, fixtures, benchmark harnesses or dependencies were authored.

The initial formatter/check attempt encountered another agent's incomplete
worker-module split. The successful commands above ran after those shared
files became available. No sidebar-specific compile or clippy errors remained.

## Measurement boundary

There is no existing sidebar click-to-present instrumentation or GUI benchmark
target in this source. The vendored `MDV_DIAG_SLICE` and `MDV_DIAG_SPLIT` hooks
report the commonmark viewer's internal scrollable path; the transcript uses
its normal `show` path, so those diagnostics do not measure this outer viewport
boundary. Native process sampling and an observed UI interaction pass can
measure the integrated executable without introducing a benchmark harness.
Build, packaging and before/after process measurements are coordinated by the
parent task. This UI slice makes no numeric latency or RSS improvement claim.

## Model popup and composer menu snapshot

Opening the model chip now updates its popup page and draws the available
choices without preparing an agent session first. The former click handler
called `prepare_agent_session`, which constructed a prompt, copied context and
history, and prepared tool-server configuration before drawing the popup.
Explicit agent/provider selection and the existing `Load models…` action keep
their preparation path. Choosing a cached model queues the selection until a
session is prepared. Sending the first prompt still constructs and starts the
agent through `prepare_prompt`.

`agent_options/menu_cache.rs` retains one immutable `Rc<[OptionMenu]>` snapshot
for the active tab. It caches normalization of the already parsed session JSON
value, the saved-catalog fallback, and pending/saved selections. Cache hits
compare the tab, composer revision, model-catalog revision, and exact agent
ID/command/argument strings. They clone only the `Rc`; they do not traverse or
hash the session JSON value or clone all choice strings on each frame.

Composer revisions advance after a pending selection, option confirmation or
rejection, successful saved-model change, custom-agent reset, or agent
disconnect. Catalog revisions advance after saved models load or live session
models arrive. Tabs retain their own composer revision. The tab identity and
agent settings distinguish a newly opened or switched tab. Future mutation
paths for these inputs must advance the corresponding revision too.

Only one snapshot is retained by the cache, replaced on the next lookup after
invalidation. A caller can keep its current snapshot alive during selection
handling without borrowing the mutable application. This is a snapshot-count
bound, not a byte or heap cap: an unusually large option catalog can still
produce a large snapshot and a costly first normalization. Popup search and
row rendering retain their existing behavior.

This follow-up changed `agent_options.rs`, its model selection/selector and new
menu cache modules, `agent_picker.rs`, and narrow field/invalidation wiring in
`app.rs`, `state.rs`, `agent_catalog.rs`, `chat/updates.rs`, `settings_files.rs`
and `chat_tabs/project.rs`. The transcript implementation was not changed.
Targeted Rust 2024 formatting checks passed, and complete modified methods and
their immediate dependencies were re-read. All reference changed Rust files are
below 250 physical lines; the largest is `app.rs` at 228. Cargo builds were held
at the coordinator's request while parallel feature work was integrating, so
the compilation and lint results above do not validate this follow-up yet.
No dependencies, tests, fixtures, harnesses or GUI sessions were added or run.
Popup latency and live-catalog/selection behavior still need an observed native
UI pass on the integrated executable.
