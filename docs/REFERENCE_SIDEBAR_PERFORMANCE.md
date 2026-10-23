# Reference and Prism sidebar performance

Status: source research completed on 2026-10-19 against the current working tree.
This report contains aggregate runtime data, source-backed costs, and proposed
changes. It contains no production changes, new tests, fixtures, timing harnesses,
or proprietary binary analysis. The user's observation is that Reference's menus,
tool display, and decompilation feel much faster. This sidebar review establishes
work on the UI path; it does not establish the duration of that work or explain
the decompiler comparison.

## Finding and confidence

Reference opens its option popovers by changing local component state. Prism also
queues session preparation without waiting for the agent, but its menu shares an
immediate-mode frame with transcript rendering, event handling, and some durable
writes. Prism traverses the retained transcript on every frame and creates a fresh
markdown cache for each body. Those are confirmed scaling problems. They make a
long conversation more expensive even when the user's action only concerns a
dropdown.

The local data does not justify attributing a large current stall to model-list
size or transcript length alone. The cached model file contains only 13 choices,
and the chats selected in persisted project settings have two to six messages.
No click-to-present trace, CPU profile, allocation profile, or matching source-to-
running-bundle verification was performed in this review. The size of each
contribution remains unmeasured.

## Source and runtime provenance

The enclosing repository is `/path/to/reference-checkout`, branch
`fix/resource-browser-performance`, HEAD
`85a9be336ecf442cd8a8aa27a8a98060cee0646a` (2026-10-16,
`fix: keep large chats responsive during rendering and saves`). Prism is an
untracked directory in that repository; Reference's `app/` has modified and
untracked files, including the chat virtualization components. HEAD therefore
does not reproduce either complete source tree. These findings describe the
inspected working tree, not a verified pristine upstream release or shipped app.

Read-only aggregate queries against
`~/Library/Application Support/Prism/reference.db` completed at
`2026-10-19T20:08:45.926705+00:00`. The connection used `mode=ro`, `query_only`,
and a 50 ms busy timeout. No message, activity body, credential, model value, or
skill body was printed. SQLite text character counts below are not byte counts,
retained heap measurements, or rendering measurements.

| Aggregate | Observed size | Implication |
| --- | ---: | --- |
| All saved messages | 295 rows; 384,911 text characters; largest text 18,256 characters | There are larger historical conversations to exercise. |
| Saved activity JSON | 129 rows; 9,750,246 characters; largest row 505,994 characters | Serialized history has substantial volume, but this is not the active heap. |
| Largest chat by text | Chat 23: 31 rows; 94,378 characters | Full transcript traversal has a meaningful larger input. |
| Other larger chats | Chat 18: 39 rows/58,687 characters; chat 12: 45 rows/45,379 characters | Stored totals can exceed a single retained history page. |
| Persisted selected chats | IDs 29, 31, 33, 34, 35: respectively 6, 2, 4, 2, 2 messages and 337, 236, 219, 96, 496 text characters | These are small; multiple project selections do not identify the currently visible in-memory chat. |
| Cached models | `agent-models.json`: 1,950 bytes, two providers, 5 + 8 choices | Per-frame menu work is real but currently has a small input. |
| Saved picks | `agent-picks.json`: 145 bytes | Storage size is small; durable write latency is not determined by payload size. |
| Custom skills | One candidate Markdown file, 77 bytes | The periodic synchronous scan is locally tiny. |

For byte-based context, chat 23's latest 31 messages contain 94,828 UTF-8 text
bytes and 1,431,058 activity-JSON bytes. Summing `min(text_bytes, 16000)` gives a
90,859-byte preview upper bound before assistant reply extraction. Chat 18's
latest 39 rows have 59,124 text bytes and 2,276,224 activity-JSON bytes; the
2 MiB history-page limit can stop restoration before all those rows are admitted.
These aggregate bounds must not be presented as actual bytes parsed in a frame.

## What a Prism click shares

[`Prism::update`](../src/ui/app.rs#L170-L191) performs capture completion,
agent-event receiving, inspector updates, catalog polling, and due settings
persistence before rendering the shell. Within the workbench,
[`panel_layout`](../src/ui/panel_layout.rs#L10-L39) renders the code pane before
the agent dock. Within the conversation, the composer is built before the
transcript, but all of that work must finish before the frame can be presented
([`conversation`](../src/ui/chat.rs#L13-L24)). Drawing the menu first does not
isolate its visible latency from later transcript work.

The main code pane already uses visible-range `show_rows`
([`code.rs`](../src/ui/code.rs#L19-L40)); this review does not establish that an
entire decompiled source file is laid out for every sidebar click.

### 1. Retained messages are rebuilt, and markdown parsing cannot stay warm

[`conversation_messages`](../src/ui/chat.rs#L46-L96) uses ordinary
`ScrollArea::show` and iterates every message in the current page. Clipping paint
does not skip those calls. A normal page is bounded to 40 messages and 2 MiB of
retained text/activity accounting after trimming
([`Chat::trim_history`](../src/ui/state.rs#L68-L91)); an active turn can briefly
add messages before the next trim. Restoration also has a 40-row, 2 MiB raw
text-plus-activity page bound
([`message_page.rs`](../src/storage/message_page.rs#L6-L94)). This is bounded
work, not an unbounded whole-database traversal.

Each body calls [`markdown::show`](../src/ui/message/markdown.rs#L7-L34), which
passes `CommonMarkCache::default()` directly into the viewer and then drops it.
The normal frontend hashes the full body, parses owned events on a cache miss,
copies the complete event vector, and walks the events
([`pulldown.rs`](../vendor/egui_commonmark_extended/src/parsers/pulldown.rs#L1648-L1738)).
Consequently every current body is a miss every frame. With block colors disabled,
[`without_syntax_hints`](../src/ui/message/markdown.rs#L37-L64) adds another
markdown pass and may copy/edit the source before the viewer parses it.

The preview bound is 16,000 UTF-8 bytes per message. Expanding “Read full message”
renders the full body in addition to its preview
([`message.rs`](../src/ui/message.rs#L70-L87)). Active assistant text can reach
1 MiB ([`chat_session.rs`](../src/ui/chat_session.rs#L4-L25)); expansion can
therefore defeat the cheap-preview assumption. Normal text access usually borrows
existing text; the confirmed issue is not a blanket full transcript string clone
([`render_message`](../src/ui/message.rs#L11-L43)). Egui font caches can reuse
galleys, so source inspection alone also does not prove fresh glyph shaping for
every paragraph.

Two library details matter when fixing this:

* The pinned `egui_commonmark_backend_extended` 0.28.3 cache has a **single**
  normal-document `cached_events` slot. `set_cached_events` replaces it. One
  global cache shared sequentially by different messages will thrash; each warm
  document needs its own cache. The owning source is registry
  `src/misc.rs:2204-2246,2658-2670`, identified further below.
* Even a hit calls `.to_vec()` on the cached events
  ([`pulldown.rs`](../vendor/egui_commonmark_extended/src/parsers/pulldown.rs#L1700-L1705)).
  The adjacent “cheap pointer copies” comment is incomplete: parser conversion
  makes events static
  ([`latex_delimiters.rs`](../vendor/egui_commonmark_extended/src/parsers/latex_delimiters.rs#L100-L103)),
  and pulldown-cmark 0.13.4 `CowStr::Clone` clones boxed strings
  (`src/strings.rs:165-173,264-271`). Persistent caches remove repeated parsing;
  they do not eliminate the event copy or all layout work.

Prism disables the commonmark dependency's default features and selects pulldown
([`Cargo.toml`](../Cargo.toml#L37-L38)). Attributing these frames to repeated
default syntect theme initialization, math, or Mermaid would be unsupported.

### 2. Activity processing adds work beyond visible answer text

The collapsed activity summary scans blocks to find the last non-text step and
count visible steps. Expanded history displays 32 steps per page but still counts
and skips through the block sequence
([`activity_view.rs`](../src/ui/activity_view.rs#L12-L61),
[`step_page`](../src/ui/activity_view.rs#L107-L168)). Expanded thought/text steps
call the same markdown renderer
([`step_text`](../src/ui/activity_view.rs#L173-L190)). Activity retention is
bounded to 2,048 blocks and 256 KiB of retained payload; retention scans/recounts
on incoming text/thought/tool updates
([`retention.rs`](../src/model/activity/retention.rs#L3-L29),
[`activity.rs`](../src/model/activity.rs#L79-L114)). Repeated chunks can therefore
do work proportional to retained block count before rendering begins.

Active, focused shimmer schedules a repaint after 33 ms. It visits generated
shapes and can copy shared galley/row allocations through `Arc::make_mut` before
recoloring glyph vertices
([`shimmer.rs`](../src/ui/activity_view/shimmer.rs#L8-L99)). This is conditional
on active visible text and the moving band; it does not explain idle-chat latency
by itself. Its allocation and frame-time impact has not been measured.

### 3. Event receiving and durable writes can precede the menu frame

[`receive`](../src/ui/receive.rs#L7-L23) handles up to 64 events per frame with no
elapsed-time budget. Parked-chat events can run the same update path after moving
the relevant state into place
([`routing.rs`](../src/ui/chat_tabs/routing.rs#L6-L46)). Text, thought, and tool
updates perform retention work; completion persists a reply and refreshes chat
metadata ([`receive.rs`](../src/ui/receive.rs#L173-L224),
[`finish_agent`](../src/ui/chat_session.rs#L28-L74)). The event channel has capacity
32 and a blocking producer `send`
([`events.rs`](../src/events.rs#L99-L124)). A slow UI can back-pressure producers,
which then makes newly completed work appear late. No fixed delay follows merely
from these bounds, and a receiver's 100 ms timeout wakes immediately on a message.

Several conditional operations use synchronous storage on this thread:

| Trigger | Confirmed work |
| --- | --- |
| Select a non-model option | Remember the pick before local option state is updated ([`updates.rs`](../src/ui/chat/updates.rs#L45-L65)). Picks read/serialize JSON and rewrite even an unchanged value ([`picks.rs`](../src/agent/picks.rs#L37-L45), [`read_file`](../src/agent/picks.rs#L99-L112)). |
| Confirm a changed model | Persist per-chat models to SQLite; unchanged model values have an early return ([`model_selection.rs`](../src/ui/agent_options/model_selection.rs#L89-L111)). |
| Finish a turn | Serialize activity and commit the reply transaction ([`history.rs`](../src/storage/history.rs#L101-L120)); then refresh chat list/count ([`chat_list.rs`](../src/ui/chat_list.rs#L19-L44)). |
| Due settings save or agent selection | Save settings synchronously; queued ordinary settings changes wait 350 ms before flushing ([`settings.rs`](../src/ui/settings.rs#L152-L179), [`agent_picker.rs`](../src/ui/agent_picker.rs#L52-L63)). |
| Due edited inspector note | Clone/update and save the project record ([`inspector/request.rs`](../src/ui/agent_dock/inspector/request.rs#L68-L101), [`notes.rs`](../src/ui/agent_dock/inspector/notes.rs#L104-L133)). |

Atomic JSON/settings publication flushes the file and containing directory around
rename ([`atomic.rs`](../src/storage/atomic.rs#L5-L39)). The UI SQLite connection
uses a three-second busy timeout, WAL, and `synchronous=FULL`
([`database.rs`](../src/storage/database.rs#L24-L29)); the worker opens another
write-capable connection ([`worker/thread.rs`](../src/worker/thread.rs#L43-L45)).
Writer contention or filesystem flushes can delay a frame. Their occurrence and
duration during the observed dropdown stall have not been measured. The timeout
is not evidence that each operation waits three seconds. Removing durability
would change behavior and is not the proposed performance fix.

### 4. Dropdown preparation repeats small allocations and scans

[`composer_toolbar`](../src/ui/composer_toolbar.rs#L26-L48) calls
`composer_option_menus` on every frame, including frames with closed menus. The
implementation traverses already-parsed JSON `Value`s, allocates choice strings,
clones fallback choices, and reapplies picks
([`agent_options.rs`](../src/ui/agent_options.rs#L11-L40),
[`options.rs`](../src/agent/options.rs#L34-L94)). Describing this as reparsing a
JSON file or decoding raw JSON text on every frame would be inaccurate.

The selected-model popup renders all matching choices with an ordinary scroll
area ([`model_selector.rs`](../src/ui/agent_options/model_selector.rs#L116-L164)).
Generic choice menus limit rendering to 60 rows but first scan/filter all choices
and create lowercase search strings
([`agent_options.rs`](../src/ui/agent_options.rs#L98-L147)). Provider menus rebuild
and sort **provider** rows by borrowed names, not the entire model collection
([`providers.rs`](../src/ui/agent_picker/providers.rs#L15-L55)). Their limit is
60 displayed models **per provider**, and matching rows after that limit are
still scanned ([`menu.rs`](../src/ui/agent_picker/menu.rs#L90-L110)). These are
real O(choice count) costs; 13 locally cached choices make them a weak sole
explanation for a large stall in this installation.

Opening the selected-model popup calls `prepare_agent_session` before displaying
it ([`model_selector.rs`](../src/ui/agent_options/model_selector.rs#L32-L48)).
Preparation constructs a prompt, captures bounded context/history, and may start
a native worker thread ([`chat_prompt.rs`](../src/ui/chat_prompt.rs#L54-L83),
[`agent_prompt`](../src/ui/chat_prompt.rs#L102-L147),
[`history/context bounds`](../src/ui/chat_session.rs#L138-L190)). The final prepare
operation is `try_send` to that worker
([`background.rs`](../src/agent/background.rs#L71-L88)). It does **not** wait for
the agent's process startup, network connection, or model discovery response on
the UI thread. Empty preparation also returns before custom prompt skill
resolution ([`invocation.rs`](../src/ui/playbooks/invocation.rs#L29-L34)).

There is separate synchronous filesystem work on the empty conversation screen:
[`empty_agent`](../src/ui/agent_dock.rs#L62-L64) asks for enabled skills, and
[`refresh`](../src/ui/playbooks.rs#L23-L51) rescans after five seconds. The scan
admits up to 64 directory entries and reads candidate files up to 128 KiB each
([`source.rs`](../src/ui/playbooks/source.rs#L31-L107)). This is a periodic cost,
not every-open model discovery. The observed one-file/77-byte directory sharply
limits its likely contribution here; filesystem latency remains unmeasured.

## What Reference's current source does differently

| Boundary | Source-backed behavior | Limit of the comparison |
| --- | --- | --- |
| Open a menu | Local `useState` toggle in `usePopover`; menu mounts conditionally in `OptionMenu`. Anchored/deferred native popover sets layer ordering. | `deferred` is not evidence of asynchronous agent preparation. |
| Prepare choices | Options normalize on config-update translation and are stored as arrays (`translate.ts`, `threadUpdates.ts`). Empty-query menu reuses choices and displays at most 60 (`OptionMenu`). | This particular model menu uses a bounded ordinary scroll, not native row virtualization. Non-empty queries still scan choices. |
| Prepare session | `useThread` ensures a session when the live target/agent effect runs. | Earlier preparation improves readiness but does not establish an absolute timing advantage. |
| Render messages | Memoized `MessageRow`, native `virtual-list`, overscan of four rows (`MessageList`). Expensive rows mount near the viewport. | Placeholder mapping still walks message identities. It is not proof every JS operation is O(visible rows). |
| Render answer/steps | Memoized answer derivations (`AgentReply`); 12,000 UTF-16-code-unit text pages (`ChatText`); collapsed steps do not mount step content; larger step lists virtualize chunks of 16 (`StepHistory`). | Native markdown cache internals were not established from this TS source. Its text bound uses different units from Prism's UTF-8 byte bound. |
| Deliver streaming UI | Adjacent compatible chunks merge; one buffered dispatch flushes after 50 ms, at 32 events, or on a non-streaming event (`useEventBuffer`). Historical message references survive tail updates (`threadUpdates`). | Tool steps are streamable events too. Reference intentionally permits batching latency; “instant” does not mean zero delay. |
| Persist history | Five-second autosave with flush on new-message/idle boundaries (`useThreadPersistence`); preparation goes to a worker (`historyAsync`, `historyClient`). | Patch preparation and final commit/rename/index work still include UI-thread work. This is not entirely asynchronous persistence. |

These mechanisms explain why opening local menus can have a much smaller shared
rendering workload in Reference's current source. They do not quantify the user's
observed difference, establish native markdown implementation details, or prove
that the running Reference bundle includes these working-tree changes.

## Proposed smallest maintainable improvement

No UI implementation is included in this pass. Keep ownership in the chat UI:
one `TranscriptView` owned by `Prism`, a cohesive transcript/viewport module, and
a body-cache module below `ui/message`. No new crate, global cache, or general
rendering framework is needed. Preserve existing message disclosure and scroll
identities while adding explicit render-state ownership.

### Persistent body cache with a stated memory boundary

Give each admitted body its own `CommonMarkCache`, plus its normalized source if
syntax hints were removed. Key by chat/session identity, history-page identity,
stable message identity, and body role (preview/full/activity block). Use a
monotonic UI identity for unsaved messages and stable activity identities across
front trimming; a vector index alone can reuse a cache for a different message.
On persistence, migrate or invalidate the unsaved entry deliberately. Existing
[`chat.rs`](../src/ui/chat.rs#L60-L78) identities show where this belongs.

Separate content invalidation from layout invalidation. Content revision or
verified hash plus parse mode determines parsed events; width changes should not
discard parsed text. Complete `ChatAppearance` already supports `PartialEq`
([`chat_appearance.rs`](../src/settings/chat_appearance.rs#L4-L14)). A layout key
must also include actual body width, code font/family, global font revision,
effective scale/pixel ratio, theme/layout resource revision, and thinking-mode
visibility ([`ChatStyle`](../src/ui/message/style.rs#L19-L55)). Do not retain
mutable state through an egui `Context` data lock while invoking the viewer,
which itself accesses the context.

The existing normal `show` API will still hash text and copy cached events. This
first change deliberately promises fewer parses, not allocation-free rendering;
an API change to remove the event copy should follow measured evidence.

For a low-memory first policy, propose **one 1 MiB retained cache-owned payload
budget for the active transcript**, with a finite entry limit, rather than an
allowance per parked chat. Enforce the admission/eviction ledger in
`TranscriptRenderCache::admit` and `finish_frame`; evict least recently visible
entries before admitting another, remove obsolete page/message entries, and drop
oversized bodies' caches after rendering. This is a proposed policy, not an
existing setting or a measured optimal value.

An honest byte ceiling needs backend support: the current cache's private event,
layout, map, and scroll-cache capacities have no retained-byte accounting API.
Add a narrowly scoped accounting API in the maintained dependency boundary, or
explicitly report a weaker source-byte/entry admission policy. Account for owned
event strings, vector capacities, normalized source, layout/map payloads, and
scroll data; source length alone is not a cache heap limit. Check the ledger
after rendering because rendering can grow a cache, and drop an over-budget
entry immediately. A 1 MiB retained-payload ceiling still excludes allocator
overhead, transient parsing/layout, and shared egui font/image/texture storage;
it must never be described as a process-RSS limit. Existing `cache_mib` settings
govern analysis view storage, not chat markdown RAM
([`analysis_cache.rs`](../src/worker/analysis_cache.rs#L109-L125)).

### One variable-height transcript viewport

Replace the outer `ScrollArea::show` with its existing `show_viewport` primitive.
Keep row heights and cumulative offsets for the current retained page; scanning
at most roughly 40 identities is cheap. Select rows intersecting the viewport
plus a small pixel overscan, allocate top/bottom spacers, render those rows, and
record complete measured heights including margins and message spacing. Preserve
the current single scroll area and tail-follow behavior.

The dependency already has [`CommonMarkViewer::show_scrollable`](../vendor/egui_commonmark_extended/src/lib.rs#L576-L596),
but it owns an inner scroll area and performs a full-layout bootstrap when
content/layout changes
([`pulldown.rs`](../vendor/egui_commonmark_extended/src/parsers/pulldown.rs#L1821-L1942)).
Its steady path skips markdown event ranges using split points
([`show_viewport`](../vendor/egui_commonmark_extended/src/parsers/pulldown.rs#L1986-L2037)).
Putting one such scroll area inside every message would introduce nested scrolling
and repeated full bootstraps on streaming/resizing. Use outer row virtualization
first. If one expanded message remains expensive after measurement, expose the
library's existing viewport/split-point core for an externally owned viewport;
that requires a focused dependency API change rather than pretending the current
`show_scrollable` API already composes with a transcript.

Variable-height correctness requires explicit rules:

* Width, font, scale, and spacing changes invalidate row metrics. Use estimates
  for stale offscreen rows and measure only a bounded visible/overscan region;
  do not synchronously lay out every hidden message to restore exact scrollbar
  geometry. First-pass scrollbar height can be approximate.
* Anchor scrolling by stable message identity plus offset inside that row.
  Compensate when measurements above the anchor change. Preserve user scroll-away
  and jump-to-tail semantics; when following, anchor the bottom.
* Streaming invalidates the changed tail body/height, not historical bodies.
  Measure visible rows each frame to catch disclosure, activity-page, full-text,
  and tool-card changes. Global thinking-mode changes invalidate relevant metrics.
* Async image size/layout revisions can change height without source changes.
  Treat those metrics as stale, remeasure visible content, and retain the anchor.
  Cache invalidation must not silently discard link, copy, selection, or disclosure
  behavior.

The dependency layout signature includes width, text-style heights, dark mode,
and some viewer options, but not every chat font-family/line/paragraph/message
spacing input ([`compute_layout_signature`](../vendor/egui_commonmark_extended/src/parsers/pulldown.rs#L1458-L1482)).
An application-level style revision is therefore necessary. Height records and
body-cache payloads need independent lifetimes: eviction of parsed text must not
reset the scroll position or retain a stale height as permanently exact.

### Subsequent focused changes, after measuring the remaining frame

Normalize menu options only on config/pick/catalog revisions; retain lowercase
search keys, reuse the empty-query list, and virtualize simple fixed-height model
rows or apply a clear global row bound. Refresh skills off-thread while retaining
the old snapshot. Remove empty session-preparation prompt/context cloning from
the click path by ensuring discovery/readiness earlier.

For event traffic, combine adjacent compatible chunks before repeated retention
and use a per-frame elapsed-time budget while preserving permissions, completion,
navigation replies, and event order. For durable writes, use an ordered persistence
owner and explicit acknowledgements/errors, preserving atomic publication and
shutdown flushing. A worker conversion must not report a rejected model pick as
committed or lose the last settings/activity update. These are distinct changes;
none is justified by an assumed duration of a timeout or by deleting `fsync`.

## Validation boundary and follow-up evidence

This pass inspected source and local aggregate sizes only. It did not benchmark
popup latency, validate running artifacts, measure cache RSS, or prove that any
proposed change meets a frame budget. Source links and document formatting were
checked; no new tests or fixtures were authored and no application build was
needed for this documentation-only output.

Before selecting an implementation, capture an existing short selected chat and
an existing larger chat, cold/warm dropdown opens, and event-heavy frames. Separate
pre-shell receive/persistence, transcript parsing/layout, menu preparation, and
present scheduling. Record frame percentiles and process/cache memory on the same
artifact, font, scale, conversation, and backend state. A 16.7 ms frame budget
would be a 60 Hz target, not a result established here.

Existing conversations and UI actions should cover resizing and font/theme/scale
changes; streaming while scrolled away; expanded thoughts/tools/full replies;
image arrival; history paging and chat/tab switching; unchanged/rejected option
picks; failed durable writes; competing database writers; and shutdown. The
critical regressions are scroll jumps, stale heights/text/style, reused message
identity, event reordering, and apparent success before persistence failure.

## Dependency and source identifiers

Registry primary sources used above reside under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`:

* `egui_commonmark_backend_extended-0.28.3/src/misc.rs`:
  cache ownership; lines 2658-2670 own normal event lookup/replacement. Lines
  2604-2645 expose layout revision/image-size updates; lines 2682-2697 prepare
  loaders.
* `pulldown-cmark-0.13.4/src/strings.rs`:
  owned-string clone; lines 264-271 own conversion to static strings.

Selected SHA-256 identifiers captured during inspection:

| File | SHA-256 |
| --- | --- |
| `prism/src/ui/app.rs` | `66a50e13b27f69edffab6e3b71a373c9ed0b9621424a32e4fde416708bed2070` |
| `prism/src/ui/chat.rs` | `5e25ba1c1a971e7b9e4cfddf054cd7d14e034fa85ac2c9379e628d285e93aa32` |
| `prism/src/ui/message/markdown.rs` | `0e115fa6571ff605d7eb60b240c837073ded894fba88441e2c7fcd9f61636880` |
| `prism/vendor/egui_commonmark_extended/src/parsers/pulldown.rs` | `1c3ce91f2eb960270c674abe31abc253618ccbdfb1cceb46ade13fdf4a1a67d7` |
| `app/src/chat/OptionMenu.tsx` | `8d4d320187e0eda1e9d5651054460e94d9ae1f1cb29d53f314b1f6d1546d1eda` |
| `app/src/chat/MessageList.tsx` | `32c95a64ae0a10066b37af6ab70036ae30763fcdd7829806b1a51da1c2416ed4` |
| `app/src/chat/ChatList.tsx` | `9d5f7e1fef7f87848a960468289e25c3fa7e6e8d1d2c292c21d0d4802c7dadd6` |
| `app/src/chat/useEventBuffer.ts` | `e54997fdfad1e0e22fb1ec2f7f588627800fe6e46ec006eeb83ca984f799d301` |
