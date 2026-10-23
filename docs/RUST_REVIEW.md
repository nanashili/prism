# Rust code and performance review

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

2026-10-18. Scope: the Prism Rust application in this directory, its standalone MCP server, and its build configuration and scripts. The original React application and the separate OpenUnity engine were not refactored. The invoked rust-skills guidance informed ownership, allocation, error handling, arithmetic, unsafe adapters, and module boundaries. The pinned Rust 1.93.1 toolchain is retained.

The original optimization pass inspected all 253 first-party Rust files. The [file inventory](RUST_FILE_REVIEW.md) records their sizes, hashes, and changed or retained status, refreshed after the follow-ups below. This pass changes 32 Rust files and the release profile, adds two cohesive internal modules, and adds no dependencies. Existing UI layouts, storage budgets, cache policy, protocol tool names, and supported features are retained. Existing unsupported features remain unsupported; this review does not establish complete upstream feature parity.

## Measured results

Measurements are local observations on this Apple Silicon Mac, using a saved pre-change release executable and the optimized release executable. MiB means 1,048,576 bytes. The native analysis inputs were Prism's own Rust MCP helper. Saved user analysis was compared without inspecting proprietary decompiled contents. These are operational measurements, not a new benchmark suite.

| Operation | Before | After | Observation |
| --- | ---: | ---: | --- |
| Full upstream import, peak resident memory | 236.28 MiB | 190.72 MiB | 45.56 MiB, or 19.3%, less |
| Same import, elapsed time | 7.619 s | 7.647 s | Essentially unchanged in this run |
| First native function analysis, first comparison | 4.253 s | 2.961 s | 30.4% less elapsed time |
| First native function analysis, repeated comparison | 6.755 s | 3.994 s | 40.9% less elapsed time |
| Native worker user CPU time, repeated comparison | 4.45 s | 3.25 s | 27.0% less CPU time |
| Desktop executable size | 17,971,872 bytes | 18,411,136 bytes | 2.4% larger in exchange for faster analysis |

The native worker's peak RAM did **not** show a reliable reduction. It was approximately 411 MiB before and after in the first comparison; in the second comparison the baseline was approximately 310 MiB and the optimized worker approximately 410 MiB. Repeated function requests used the existing cache and completed in approximately 0 to 5 ms. A drastic reduction in native analysis RAM is not established.

After launch, export, and returning to the stored project, the rebuilt desktop sampled at 0.0% idle CPU, approximately 128 MiB resident memory and 190 MiB physical footprint. Its main thread waited for AppKit events. The older process had a different lifetime and activity history, so these desktop samples do not support a before/after memory claim.

## Implementation

- Native Ghidra uses speed optimization only for its dependency package; the rest of the release retains size optimization, thin LTO, and one code-generation unit. Call queries and function discovery no longer materialize discarded flow-block JSON. Existing function release and cancellation boundaries remain in place.
- Incoming references use sorted call edges and binary range lookup instead of scanning the complete call index for every selected function. Image reads reserve the validated slice size and still detect truncation.
- RPC, REA, engine and MCP paths move owned JSON results out of envelopes. MCP replies serialize once before their size check and write. Valid UTF-8 history text reuses its owned byte allocation; invalid text retains the previous lossy fallback.
- Inventory migration decodes and inserts one procedure at a time inside the existing SQLite transaction. Invalid input, trailing data, and row-consumer failures still fail the operation and roll back the inventory. Input size limits are unchanged.
- Project saving borrows unchanged imported metadata and streams JSON through a bounded buffer, digest, and Zstandard encoder. It avoids a full serialized byte vector per metadata entry. Metadata limits, raw hashes, binary verification, temporary payloads, and atomic destination replacement remain enforced.
- Graph construction uses binary search over ordered assembly, retaining a linear fallback for unsorted providers. Rendering resolves edge destinations once, preserves the first matching duplicate address, culls invisible edges, and borrows block previews. Chat history trims its prefix in one drain; activity paging and prompt history avoid temporary collections. Code-font lookup borrows the cached family catalog.
- Registry loading deserializes directly into typed entries. Byte comparison compares the existing buffers directly. Unsafe process and macOS memory adapters document their safety invariants. No new unsafe code or global mutable state was introduced.

## Validation actually performed

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings -W clippy::undocumented_unsafe_blocks -W clippy::redundant_clone -W clippy::needless_collect`
- `cargo check --no-default-features --bin prism-mcp`
- `cargo test --all-targets --all-features`: the one existing bridge-secret test passed. No tests, fixtures, or test harnesses were written.
- `cargo build --release --bins`
- Physical source-size inspection: all 253 Rust files fit the applicable 250-line source, 100-line module/library root, and 120-line main-file limits. Python build scripts compiled for syntax checking without generating bytecode.
- `python3 scripts/bundle-macos.py --skip-build`: release app and helper packaged and ad-hoc signed, then verified with strict codesign verification. The bundle is approximately 19.9 MiB, including native Ghidra and dependency notices.
- Native worker comparisons returned identical complete JSON for open, 300-row function listing, first function analysis, and the cached repeat in both before/after pairs. Each worker exited successfully.
- Imports ran in separate temporary profiles containing copy-on-write clones of the existing snapshot. Both added 7 projects, 18 chats, 234 messages, 216,135 function rows, and 7 saved views, while skipping 17 existing records. All database table contents matched after normalizing only scratch-profile paths and view import timestamps. Both databases passed `quick_check`.
- The rebuilt desktop launched and displayed the stored project, sidebar, icons, chat controls and notifications. Before/after exports contained the same five entry names and raw content hashes, including the 118,864,128-byte binary. Both archives were independently decompressed and every entry's length and hash verified. Compressed file sizes differed by 12 bytes. The project's normal source path was restored after export verification.

The OpenUnity architecture and size scripts are outside this standalone project and were not applied to it. An exploratory large-stack-array Clippy check identified existing bounded 64 KiB streaming buffers; those were retained rather than adding heap allocations. The final strict command above passes without weakening existing validation. macOS was exercised; Linux/Windows runtime behavior, live agent streams, all optional external engines, forced resource exhaustion, and transactional failure injection were not exercised in this pass.

## Remaining RAM work

The pinned ghidra-decompiler source stores an executable image in both the architecture and loader, and clones large processor translator data into its static cache and active engine. This is the largest remaining opportunity identified by source inspection. The size of each contribution has not been isolated with allocator profiling.

A substantial reduction should be pursued upstream through shared immutable processor tables and shared image ownership, with explicit per-session teardown. The current public API does not expose clearing the static translator cache. Modifying a Cargo checkout or vendoring a large private fork would add maintenance and storage cost, so this pass leaves the pinned dependency intact.

At that stage, killing an idle worker was rejected because its discovery queue and discovered function inventory were live session state. The later native-analysis persistence follow-up below adds the missing durable checkpoint. Lowering memory budgets would stop analyses sooner rather than make the same analysis more efficient.

Large chat archives still retain their required JSON representation and the original compressed snapshot. A later streaming chat migration could further reduce import peaks, but must preserve activities, opaque payloads, and recovery data. That change needs its own measured compatibility work. Existing inactive-tab eviction, bounded history, paged catalogs, lazy worker startup, and compressed SQLite views already limit the application-side working set.

## Shared-tab follow-up

Binary and conversation headers now reuse one internal `ui/tab_strip.rs` component. Both have the binary header's rectangular surface, 36-pixel height, active top marker, natural title sizing, horizontal overflow, hover close button and middle-click close action. Conversation tabs keep their agent artwork, busy indicators, drafts and existing session operations; binary preview and context-menu actions remain with the workbench. The four chat controls reserve their own header space. No dependencies or storage contracts changed.

The updated 254-file inventory includes the shared component and refreshed hashes for its four callers. Formatting, all-target/all-feature checking, strict Clippy with redundant-clone and needless-collect checks, the existing bridge-secret test, source-size checks and the optimized release build passed. No tests, fixtures or harnesses were added. The performance measurements above belong to the preceding optimization pass and were not repeated for this presentation change.

The rebuilt macOS bundle passed strict signature verification and launched as one window on the stored project. A visual comparison with a Notes workbench tab confirmed matching header heights, active markers, tab surfaces and aligned controls. The normal project view was restored afterward. Live multi-tab overflow and pointer close actions were reviewed in source but were not exercised through UI automation.

## Radio-control follow-up

Engine choices and permission preset cards now use the same settings radio painter. The indicator keeps a 16-pixel outer diameter in both states, with a 1.5-pixel ring and a centered selected dot. Hover contrast and a separate keyboard-focus outline replace the previous oversized selected strokes. Existing engine commands, permission changes and click targets are retained. Engine choices also expose their radio label and selected state through widget metadata. No dependencies or data formats changed.

Formatting, strict all-target/all-feature Clippy, the existing bridge-secret test, source-size checks and the optimized release build passed. The macOS bundle was packaged, signed and verified, then relaunched as one window. Captures of Engines and Permissions confirmed the updated selected and unselected indicators. Keyboard focus and pointer selection were reviewed in source but not exercised through UI automation. No tests, fixtures or harnesses were added.

## Storage-settings follow-up

Storage now uses the same centered 720-point content limit as the other settings pages. The “This device” overview reuses existing settings buttons and grouped cards, with a Refresh action, proportional usage bar, responsive category legend, and the local data path. Both settings scroll areas hide their scrollbar while retaining wheel and trackpad scrolling. The shared scroll-area configuration covers normal pages and search results.

The metadata-only scan runs in one cancellable background thread. It skips symbolic-link entries, reads no file contents, and stops at 100,000 entries, 64 nested folders or a ten-second elapsed-time check. Filesystem calls themselves remain blocking. Scan failures and limits produce visible partial totals rather than a complete-looking estimate. Results stay cached between frames and refresh on entering Storage, requesting Refresh, or completing a storage action. Closing the application or invalidating an active scan requests cancellation. No persistent storage index or continuous scan timer was introduced.

Six categories describe actual files under Prism's data folder: the shared projects/chat database, code-view cache, generated files, agents, import backups, and other local files. The figures are logical file sizes, not allocated disk space. External project inputs and exports are excluded. The database cannot cheaply attribute shared pages to individual chats, so the UI does not invent separate per-chat or subagent byte totals.

Chat review uses a separate cross-project metadata query, oldest first, retaining at most 50 rows and one continuation probe. Search, paging, individual deletion, archived-chat restoration and existing cache clearing reuse the storage layer and settings confirmation controls. Project-scoped sidebar queries retain their existing indexed filter. Deleting chats frees reusable SQLite space; it does not promise that the shared database file shrinks. Projects, notes and original import backups are retained by chat and cache cleanup.

Formatting, strict all-target/all-feature Clippy, the existing bridge-secret test and the optimized release build passed. The standalone MCP configuration was also checked without desktop features. All 259 Rust files fit their source-size limits. The macOS bundle was rebuilt, ad-hoc signed and passed strict signature verification, then opened as one window on the stored project. General and Storage framebuffer captures confirmed matching content edges and hidden scrollbars, with a wrapped category legend. These captures inspect the rendered application content, not macOS compositing.

Read-only inspection of the saved database passed `quick_check` and found 24 projects, 31 chats, 259 messages and 278,488 function rows. Usage labels were compared with a separate metadata-only size inspection. No chats, caches or import backups were deleted during verification. Destructive confirmations, archived-chat restoration, pointer scrolling, scan-limit behavior and filesystem-failure paths were reviewed in source but were not exercised through UI automation. No dependencies, tests, fixtures or test harnesses were added. The earlier optimization measurements were not repeated for these settings changes.

## Chat-layout follow-up

2026-10-19. Activity rows use a consistent two-point gap instead of adding ten points after every block. Thought and progress text reuse the existing Markdown renderer, trim surrounding line breaks for presentation, and retain the 16,000-byte preview and full-text disclosure. Activity paging still renders at most 32 blocks. Paragraph spacing is scoped to the renderer, and message actions reserve less space without changing stored message text or copy contents.

Tool headers reserve separate regions for the title, elapsed time and caret. Titles break at any character when necessary, elide on one line and paint within their clip rectangle. Hovering exposes the full title. The Tool badge keeps a consistent height. These changes reuse existing UI modules and add no dependencies or persistent caches.

Formatting, strict all-target/all-feature Clippy, the existing bridge-secret test, source-size limits and the optimized release build passed. All 259 Rust files fit their limits and the source inventory matches their hashes. The 19.90 MiB macOS bundle passed strict signature verification and reopened as one window on the saved project, with no pending job or startup error. Before/after framebuffer captures show the reduced message spacing. Expanded activity, paging, tool details and narrow-panel interaction were reviewed in source but not exercised: macOS denied automated pointer access. No tests, fixtures or harnesses were added, and no new performance measurements were taken.

## Window-resize follow-up

2026-10-19. Disabling VSync caused user-reported flickering and was reverted. VSync now remains enabled. A native main-thread sample during automated resizing attributed approximately one quarter of samples to OpenGL buffer presentation, with additional time in OpenGL context updates. General settings and the saved workbench had similar resize timings, so this pass changed presentation rather than speculatively rewriting chat layout.

A comparison build used eframe 0.33.3's WGPU Metal renderer with synchronized FIFO presentation and a requested maximum frame latency of one. It retained the same single window, transparent content, native material placement, session persistence, cancellation and resource teardown. No repaint timer, renderer switch during a drag, custom native drawing hook or unsafe code was added.

The operational comparison used 82 native size updates between 1440 by 940 and 1120 by 780, with a 16 ms pause after each setter and restoration of the original size. On the same saved project, OpenGL medians were 11.56 to 11.76 ms and 95th percentiles 20.82 to 25.31 ms across Workbench, Settings and Workbench. After its first pass, Metal medians were 8.08 to 8.38 ms and 95th percentiles 9.73 to 18.47 ms across Settings and repeated Workbench passes. The first Metal Workbench pass had a 100.34 ms 95th percentile. Both renderers had occasional requests above 100 ms. These are native automation request timings, not GPU frame durations, manual cursor-drag latency or proof that flicker is absent.

Idle Workbench CPU increased by 0.00 seconds over a five-second sample. One Settings sample increased by 0.11 seconds. The comparison GUI executable increased from 17.68 to 19.69 MiB, and the complete bundle from 19.90 to 22.18 MiB; the standalone MCP helper remained approximately 0.43 MiB. Observed Metal physical footprint ranged from 193.6 to 208.2 MiB after resizing, compared with 175.7 MiB for the earlier OpenGL process. Process ages and allocation histories differ, so these samples establish no RAM improvement.

The user rejected these tradeoffs. The renderer configuration, manifest, lockfile, shutdown hooks and material-adapter comment were restored to the pre-comparison OpenGL versions. The new direct WGPU dependency and four comparison-only notice folders were removed. VSync remains enabled, with no adaptive disable path. The original manual resize sluggishness remains unresolved; the Metal timing improvement is an experimental result, not a retained optimization.

Formatting, locked all-target/all-feature strict Clippy, the one existing bridge-secret test, the release build and standalone MCP checking without desktop features passed for both the comparison and the rollback. All 259 Rust sources satisfy their size limits and inventory hashes. The five renderer experiment files match their pre-comparison copies, and the final release executable's SHA-256 matches the saved synchronized OpenGL executable. The restored 19.90 MiB bundle passed strict signature verification and reopened the saved project as one window, with zero pending jobs and no startup error. Native compositing and manual pointer resizing were not verified. No tests, fixtures or harnesses were authored.


## Project-switching follow-up

2026-10-19. Switching between saved projects used full software SHA-256 on Apple Silicon and rebuilt a temporary function catalog with separate address and position indexes. Group-only queries also joined the complete function inventory, although the temporary catalog already belonged to the selected project. Temporary stage timings isolated approximately 0.67 seconds of project reading, 0.79 seconds of catalog preparation and another 0.23 seconds of sidebar count/group queries for the 118,864,128-byte saved project. Saving its small project state took less than one millisecond. All timing hooks were removed before the final build.

The existing pinned sha2 dependency now enables its assembly feature, which selects runtime-detected SHA-256 CPU instructions on ARM and retains a software fallback. Full file reads, hash comparisons and cancellation checks remain unchanged. The feature adds the locked transitive sha2-asm 0.6.4 dependency and its bundled MIT notice. No first-party unsafe code was introduced.

The sidebar catalog now uses its integer position as SQLite's row key. It creates the unique address index after importing the rows, eliminating the separate position index and incremental address-index maintenance. Group-only queries use the temporary catalog directly, unfiltered all-function counts reuse its total, and name/search/function queries retain the project-qualified inventory join. Visibility predicates remain shared. Catalog replacement, inferred module assignment, address uniqueness and identity publication remain inside the existing transaction; analysis replacement still invalidates the catalog. No persistent schema, cache, worker thread or UI layout was added.

Operational measurements used the existing authenticated open_file/get_state path, which shares the project switcher's open job. Both builds were restarted and activated. Three alternating switches between existing saved 6,513,200-byte and 118,864,128-byte inputs were recorded for each build. Completion required zero pending jobs to remain stable for 150 ms; that stability interval is excluded from the reported loading time. Project hashes matched the stored identities, so no new analysis was requested.

| Saved project operation | Before median | After median | Change |
| --- | ---: | ---: | ---: |
| Large project appears in UI state | 696.2 ms | 102.3 ms | 85.3% less elapsed time |
| Large project, all loading jobs settled | 2,259.9 ms | 1,506.7 ms | 33.3% less elapsed time |
| Small project, all loading jobs settled | 110.0 ms | 77.0 ms | 30.0% less elapsed time |

The original long-running process took 8.84 to 9.30 seconds to finish the large switch. Restarting alone reduced its baseline substantially; those slower observations are excluded from the improvement percentages. These results measure one pair of saved projects on this Mac, not long-session performance, native menu click latency or all supported input formats. Function grouping and resource enumeration still run when changing projects, so loading is not instantaneous.

The GUI executable decreased from 18,535,312 to 18,427,696 bytes. The full bundle remains approximately 19.90 MiB, with a roughly 0.43 MiB standalone MCP helper. Both compared processes reported a 240 MB physical footprint after switching. Different allocation histories prevent a precise RAM comparison; no RAM reduction is claimed. Existing bounded pages, SQLite cache budgets, OpenGL and enabled VSync are retained.

Formatting, locked strict all-target/all-feature Clippy, the one existing bridge-secret test, the release build and standalone MCP checking without desktop features passed. All 259 Rust sources satisfy their size limits. Read-only database quick_check passed; the saved inventory retained 25 projects, 31 chats, 259 messages and 278,524 functions. Filter/sort variants and transaction-failure paths were reviewed in source but not exercised through pointer automation. No tests, fixtures or test harnesses were authored.


## New-app progress follow-up

2026-10-19. Opening a file previously emitted only a text status during file inspection. The analysis task did not exist until indexing began, so Home had no progress footer during preparation. The shared opening worker now starts its existing indeterminate progress task before releasing the previous engine or inspecting the input. Existing Ghidra stages and measured inventory counts replace that task as work advances. Worker completion still clears it on success, cancellation or failure. No dependency, storage schema, worker thread, UI component or repaint timer was added.

Live verification used the existing authenticated window API and an independently built Prism.app with no saved analysis. Before the change, Home reported Reading file with analysis=null. Afterward, Home reported the Reading file analysis task, followed by Loading binary and live indexing counts of 768 and 11,520 out of 18,852 functions. The workbench framebuffer capture showed the slim footer bar and Stop control. All 18,852 functions and the completion marker were saved; the progress task cleared when indexing finished. These operations used the normal application, with no authored tests, fixtures or harnesses. Stop activation, unreadable-input failures and the Android/provider variants were reviewed in source but not exercised during this follow-up.

Formatting, locked strict all-target/all-feature Clippy, the existing bridge-secret test and the optimized release build passed. All 259 Rust sources satisfy their size limits and inventory hashes. The rebuilt 19.90 MiB macOS bundle passed strict signature verification and reopened normally. The temporary UI-side initialization was removed before the final build; src/worker/open.rs is the only changed production source file.

## Saved-chat paging follow-up

2026-10-19. Saved chats previously exposed only their latest 40 messages. Earlier, Newer and Latest controls now navigate stored history with the existing message, Markdown and activity renderers. Each chat retains its bounded live tail and at most one additional historical page. Replacing that page or returning to Latest releases the historical message data. Live events continue updating the current assistant message; submitting a prompt returns to the live tail. Switching conversation tabs preserves each tab's page, draft and agent session.

Storage uses the existing `(chat,id)` index with exclusive ID cursors and chronological presentation in both directions. Pages retain at most 40 messages and 2 MiB of stored text/activity payload. Text previews retain the existing 1 MiB limit, activity reads are bounded before JSON decoding, and an oversized first entry returns a visible error. An unsuccessful page read preserves the displayed page. Payload limits exclude decoded structure overhead, temporary reads and rendering allocations. Local message IDs are assigned after committed writes and excluded from project JSON; no schema migration or dependency was introduced. Project-file exports still include only the latest retained messages, and embedded JSON history without SQLite records cannot page beyond its retained content.

Formatting, locked strict all-target/all-feature Clippy, standalone MCP checking without desktop features, and the one existing bridge-secret test passed. All 261 Rust files satisfy their source-size limits and match the review inventory. Read-only inspection found 32 chats and 263 messages at verification time; query plans confirmed indexed searches for latest, earlier and newer pages. Page-button interaction, active-agent streaming while browsing, oversized-message failures and database-error recovery were reviewed in source but not exercised through UI automation. No tests, fixtures or harnesses were authored, and no RAM reduction is claimed.

The optimized release build passed. The rebuilt 19.92 MiB macOS bundle passed strict signature verification and reopened as one window on the current saved project, with zero pending jobs and no startup error. The earlier settings-group border removal remains in the shared renderer. This verification does not establish full upstream parity.


## Backend cache and chat follow-up

2026-10-19. Native analysis calls from the UI and MCP now share the existing compressed SQLite view cache. Previously, MCP code requests always started or called Ghidra, even when the UI had already saved the result. Validated requests for pseudocode, assembly, P-code, caller/callee replies and function dossiers now reuse successful saved results. Keys include the executable SHA-256, pinned adapter version, canonical hexadecimal address and result kind; caller-inclusive and local-only dossiers remain separate. Explicit Analyze and successful deeper discovery invalidate that project's native snapshots. Other providers retain their existing paths. No additional in-memory result catalog or always-running worker was introduced.

Cache writes previously recalculated every compressed payload's size, and eviction ordered all rows without an access-time index. SQLite triggers now maintain a singleton payload counter; indexed eviction, insertion and accounting commit together. Existing cache files calculate their initial counter once. Successful reads coalesce recency writes to once per entry per minute. A zero-byte budget avoids compression/insertion. Entry and compressed-payload budgets remain enforced. Unfiltered native inventory pages start directly at their offset, avoiding repeated scans of preceding entries and unnecessary lowercase allocations. Filtered pages preserve their prior semantics. Uncached incoming-reference scans now report progress; they still scan known functions and retain explicit failure/coverage information.

Measurements used the normal authenticated application API, comparing the previous and rebuilt release on the same preserved, independently built Prism MCP executable (SHA-256 `c7d0cec6094c078cb5bbad0263b3bb6fc9244150d3d5d56654edb2cad8aae056`). Six functions were requested three times per build. The first request populated each new cache entry; the last two measured repeat calls. All six response SHA-256 values matched the baseline. These are single-Mac samples, including bridge/serialization time, rather than a general benchmark claim.

| Repeat pseudocode request | Previous range | Rebuilt range |
| --- | ---: | ---: |
| Six sampled functions, two repeat calls each | 1.302–9.954 ms | 0.303–0.731 ms |

For the 2,729-byte reply at `0x10001e060`, a request after the previous analysis worker idled out took 111.083 ms and started a 63.06 MiB RSS native-analysis process. Reopening the same target reset the rebuilt worker; the saved reply then took 1.132 ms with an identical response hash, and process inspection found no native-analysis child. This demonstrates avoided worker allocation for that cached path; it does not establish a 63 MiB reduction in total application footprint. The first uncached caller scan took 3,083 ms before and 2,788 ms afterward; subsequent replies took 1.6–1.7 ms before and 0.8–1.1 ms afterward, with the same response hash. The first-scan timing difference is not attributed to an algorithmic speedup. Uncached decompilation and whole-binary analysis still perform real analysis work.

The accompanying chat work embeds nine adapted Prism-owned playbooks in the executable and app bundle, shares their bodies with `Cow`/`Arc`, and retains bounded custom Markdown loading with a five-second demand cache. Typing `/` in an empty composer and the slash button open the same searchable popup. Selecting a playbook submits its body together with the current draft and attachments. New MCP sessions advertise `prism` and `prism-analysis`; compatibility CLI flags, storage paths and historical transcript tool names remain preserved.

Follow-ups submitted during a running turn use a persistent FIFO queue of at most eight entries and 2 MiB of serialized data. Queue/draft edits commit atomically and restore in-memory items on failed persistence. Successful saved turns advance the queue; failures and Stop pause it. Restored queues remain paused until Send next. Prompt acceptance and removing its queued record are separate operations, so interruption between them can leave a queued item to be sent again after restart. No exactly-once delivery claim is made. Parked chat events temporarily use that chat's identity when starting a follow-up. Agent-advertised slash commands, the mention picker, plan/proposal cards and complete original chat parity remain unimplemented.

Locked formatting, strict all-target/all-feature Clippy, the existing bridge-secret test, standalone MCP checking without desktop features, and the optimized release build passed. No dependencies, tests, dedicated fixtures or harnesses were added. All 273 first-party Rust sources fit their size limits and match the review inventory. The rebuilt 19.98 MiB bundle passed strict signature verification and launched successfully. MCP initialization reported the expected names and 18/14/4 tools in combined/window/analysis modes; all nine playbook files are present in the bundle. Live agent queue progression, queue persistence failures, custom-skill submission and popup keyboard/pointer interaction were reviewed in source but were not exercised through a live agent in this follow-up.

Read-only `quick_check` passed for both native databases. The saved inventory retained 29 projects, 32 chats, 263 messages and 317,906 function rows. Cache accounting matched the actual sum of compressed payload lengths (59,105 bytes across 107 entries), and the eviction query used `views_accessed` as a covering index. No user chats or source analysis were deleted for verification. The original project and workbench view were restored afterward.


## ACP chat and Prism branding follow-up (2026-10-19)

The Rust chat now handles advertised commands, plan entries, context usage, notices and bounded child-agent previews through a focused ACP update translator. Child traffic is kept out of the main reply. Persisted activity accepts older records without progress metadata. The composer shares its keyboard picker behavior between the command palette and project mentions; selected references are resolved before sending or queueing. Stop requests ACP cancellation, preserves a responsive session when configured to keep agents running, and falls back to closing it after a 15-second acknowledgment timeout. Startup and option phases remain separate from prompt execution.

The glass prism logo is owned by Prism and shared by UI surfaces. A compact PNG limits decoded image storage to 256 x 256 RGBA pixels, while macOS retains its multi-resolution icon. The original application's icon files are untouched. Icon regeneration uses the existing macOS `sips` and `iconutil` tools; there are no new Rust dependencies.

The home-composer follow-up removes the landing-style flag throughout the shared composer, layout, toolbar, agent options and permission control. Home, setup and conversation call the same component directly. This removes separate padding, typography and outlined attachment/permission variants without adding a UI abstraction or dependency. Screen-specific placeholder text and the workbench function chip remain contextual content.

This earlier pass did not establish complete Reference chat parity. Live options, idle updates and goals are addressed in the session follow-up below; full child-session navigation and string mentions remain unsupported. Existing checks verify compilation, linting and the available bridge test; no new tests or fixtures were authored. The new ACP update variants have not yet been exercised by a live model turn.


## Persistent chat session follow-up (2026-10-19)

Compared the Rust chat against the original `agents/session.ts`, Codex launch/metadata handling, permission policy and `chat/GoalBar.tsx`. Session workers now poll between turns, deliver live configuration changes through a bounded control queue, match out-of-order replies and expire unacknowledged changes after 30 seconds. Per-agent choices use a separate bounded JSON file, restore model-first on a new connection and retain newer choices when an older request fails. Switching agents retires the old worker and rotates its event key so late messages cannot overwrite the replacement agent's state.

Codex receives the original session/subagent capability metadata, startup conversation instructions and merged MCP environment configuration, preserving configured servers and explicit Prism tool timeouts. Generic ACP agents keep the standard MCP/session path. Permissions recognize Prism's current server names while preserving historical aliases and conservative handling for generic analysis dispatch.

Goal metadata updates feed a compact disclosure bar with status, objective and elapsed time. Pause and Clear use direct session controls, including `/goal pause` and `/goal clear` while a turn is active. Explicit null clears the goal; missing or invalid metadata leaves it alone. Active goals prevent the ordinary idle eviction and prompt time limit. Saved goal snapshots remain visible with a closed-session label, and idle goal changes update the existing activity record without duplicating its message.

All nine built-in playbooks remain embedded and bundled independently. Tool-server references in three copied playbooks now match the separate window and analysis catalogs. The existing custom skill matches its original bytes in Prism's own skills directory. Original agent choices were copied independently without overwriting existing Prism choices. Future upstream imports validate and copy choices only when absent.

Approval handling now reads and atomically stores explicit Always allow answers in Prism's own profile, scoped by project identity. Never rules and the existing whole-file privacy setting take precedence. Storage failures produce a visible notice and fall back to asking. Settings expose the remember toggle and a working Forget all action. Rename, note, comment and bookmark approval requests use compact suggestion cards without bypassing the agent's offered permission choices. Preview serialization stops at a bounded writer rather than allocating a complete large JSON string first. Whole-file detection applies only when an agent requests ACP approval; the provider retains responsibility for its independent tool permissions. Original approval grants were not copied into the new app.

No dependency, test, fixture or harness was added. Full child-session navigation, permission-answer history and string mentions remain incomplete. A pending interactive permission request still waits for its answer before that worker can process other session controls. Goals, interactive approval paths and all advertised option variants require live-agent acceptance beyond the available existing validation.

Locked formatting, strict all-target/all-feature Clippy, the existing bridge-secret test and standalone MCP checking without desktop features passed. All 299 Rust sources fit their size limits and match the refreshed review inventory. Two consecutive live Codex turns on Prism's independently built MCP executable returned the requested phrase and correctly recalled the previous reply, with no tool calls or reported errors. The Skills screen displayed all nine bundled playbooks and the copied custom playbook; the custom file matched its original bytes. This verifies normal conversation continuity and skill discovery, not complete upstream parity.

The final optimized desktop and standalone MCP release builds passed. The rebuilt 20.63 MiB bundle passed deep strict macOS signature verification; its GUI executable is 17.47 MiB and its standalone helper is 0.43 MiB. All nine bundled playbooks match the source resources. After rebuilding the approval changes, a live Codex turn again returned the requested phrase without a tool call or reported error. One GUI instance remained open and the original project was restored with no pending operation or error. No total-RAM reduction is claimed from these checks.

## Desktop icon follow-up (2026-10-19)

The shared glass prism now has an opaque pure-black background. The 1024-pixel sRGB master includes safety padding without a baked platform mask. The existing export script reuses native image resizing and ICNS tools, builds a multi-resolution Windows ICO container, and exports the Linux hicolor sizes. A matching Linux desktop entry uses the same application ID as the native window. Windows icon embedding uses pinned, optional build-only `embed-resource` 3.0.11; its resource compiler must succeed for a Windows desktop build. The dependency is excluded from MCP-only builds and the normal runtime dependency graph. No runtime image decoder features were added; the UI still embeds a single 256-pixel image.

PNG inspection confirmed the master's 1024-pixel RGB image, black outer pixels and sRGB gamma/chromaticities. All 15 ICO frames decoded at their declared dimensions. Native ICNS extraction recovered the complete ten-file 1x/2x icon set, and all nine Linux PNGs decoded at their advertised resolutions. The generator ran successfully, and its Python compilation passed. Windows resource compilation and Linux launcher validation were not run because their platform tools are unavailable here.

Formatting, locked strict all-target/all-feature Clippy, the existing bridge-secret test, MCP-only compilation and both optimized release builds passed. All 300 first-party Rust files, including the new build entrypoint, fit their source-size limits and match the review inventory. Complete changed methods and their immediate dependencies were re-read for responsibility boundaries, error propagation and platform selection. No tests, fixtures or harnesses were authored.

The rebuilt 20.31 MiB macOS bundle passed deep strict signature verification. Its declared ICNS has the same SHA-256 as the exported source. The app relaunched as one GUI instance, retained the original project, and returned to Skills settings with no pending operation or error. Native framebuffer captures verified the new shared artwork. Other operating-system builds and OS icon-cache behavior remain unverified.

## macOS Dock icon shape follow-up (2026-10-19)

The supplied Dock screenshot showed the opaque square extending beyond adjacent rounded icons. macOS now has dedicated rounded black artwork with transparent exterior padding and a larger prism relative to the tile. The built-in image generation tool edited the existing master; the generated source and exact prompt are retained under `assets/branding/prism-logo-macos-v5.*`. The exporter derives both the ICNS frames and a compact 256-pixel runtime PNG from this source.

Inspection of pinned eframe 0.33.3 found a second cause: its application-icon setter replaces the bundle icon with the viewport PNG at startup. The entrypoint now selects the matching rounded PNG on macOS. Windows/Linux continue selecting their prior image. Native option construction is separated from command dispatch without changing CLI routes, the GUI data-lock lifetime, renderer, VSync or window geometry. The application still decodes one compact startup icon; no runtime dependency or image cache was added.

The exporter and Python compilation passed. All ten extracted ICNS frames have transparent outer corners; the compact runtime PNG matches the decoded 256-pixel ICNS frame. The twelve existing Windows, Linux and shared PNG assets remained byte-identical. Formatting, locked strict all-target/all-feature Clippy and the optimized release build passed. All 300 Rust files satisfy their source-size limits and match the refreshed review inventory. The modified methods and immediate dependencies were re-read for platform selection, error propagation and lifecycle handling. No tests, fixtures or harnesses were authored.

The final 20.67 MiB bundle passed deep strict signature verification. Its ICNS matches the export, its executable embeds the macOS runtime PNG, and its linker UUID matches the release artifact. Code signing changes executable bytes, so whole-file hashes do not identify the unsigned release payload. The app relaunched as one GUI instance on the original Unity project with zero pending operations and no startup error. The actual Dock appearance was not captured through the available tools; Windows/Linux builds remain unverified on this Mac.


## Direct attachment picker follow-up (2026-10-19)

The shared composer plus button now opens the native picker directly. The custom attachment popup, manual path/URL input and unused input state were removed. The slash palette reuses the same action. Pinned rfd 0.17.2 already provides `pick_files_or_folders` on macOS; its implementation enables files, directories and multiple selection together. This change needs no new dependency, platform adapter or unsafe code. Other targets retain rfd file selection and folder drag/drop. Existing saved URL references still deserialize, preserving draft and queue compatibility.

Selected files and folders continue through the existing bounded attachment validation and persistence path. Cancelling the picker leaves the draft untouched. Image delivery, duplicate handling, the 32-item cap and targeted composer drops remain unchanged. Missing-location drop guidance now directs the user to save the item and use the plus button. Complete changed methods and their immediate dependencies were re-read for ownership, responsibilities, cancellation and error propagation.

Formatting, locked strict all-target/all-feature Clippy and the optimized release build passed. All 300 Rust files fit the source-size limits and match the refreshed review inventory. No test, fixture or harness was authored. The 20.67 MiB app bundle passed deep strict macOS signature verification and reopened with the original project loaded. A native accessibility inspection did not finish and was terminated; interactive picker selection/cancellation and Windows/Linux builds remain unverified.


## Inline composer commands and capability discovery (2026-10-19)

Typing `/` or `@` no longer removes the marker or transfers editing to a separate search box. The shared composer filters suggestions at the caret, retains the draft and reuses the existing palette row renderer. Completion replaces only the active token, preserves surrounding text and restores a character-based cursor. Slash queries apply at the beginning of the draft; mentions remain restricted to the current project's bounded function, bookmark and note catalog. Empty mention results are cached for the unchanged query. Return, Tab and arrow navigation share the existing picker. Escape closes suggestions without erasing the draft. A live check exposed egui's earlier Tab focus-navigation phase; the input now locks Tab and Escape to the composer only while suggestions are open.

Opening the inline command menu or toolbar palette prepares the selected ACP session in the background, allowing advertised commands to appear before the first message. Preparation never sends `session/prompt`, saves a blank chat or appends an assistant turn. The bounded worker distinguishes preparation from prompt execution and deduplicates concurrent preparations. Stable UI chat keys allow the first saved turn to reuse the prepared session. Existing idle delivery, cancellation, session expiry and per-chat routing remain in their owning modules. Failed preparation reports a notice rather than marking an existing assistant turn finished.

Previously, the ordinary context/history prefix also prevented adapters from recognizing slash commands at the start of a prompt. Slash commands now bypass that prefix. Codex receives project context and restored conversation history through its existing session-instructions extension. Other agents retain pending history for the first ordinary prompt after a command. Direct `/goal pause` and `/goal clear` interception is restricted to Codex; another agent's own goal command passes through to its adapter. The UI advertises only commands the selected agent actually publishes.

Formatting, locked strict all-target/all-feature Clippy, the optimized release build and the standalone MCP-only check passed. The one existing bridge secret-handling test passed. All 303 Rust files satisfy the source-size limits and match the refreshed inventory. Complete modified methods and immediate dependencies were re-read for ownership, focus, cursor boundaries, errors and session lifetime. No dependency, test, fixture or harness was added.

The rebuilt 20.67 MiB macOS bundle passed deep strict signature verification. Native UI inspection confirmed that `/goa` stays visible and Codex advertises `/goal` before a message is sent. Tab completed it to `/goal ` with the composer still focused. Down plus Return selected a current-project function mention while preserving the preceding draft. Escape retained `/goa` and focus while dismissing the menu. These checks did not send a chat prompt or start an autonomous goal. Full goal execution, pause/clear lifecycle acceptance, other agent adapters and Windows/Linux execution remain unverified. The verification draft was cleared and the original project and workbench screen were restored.

## Configured provider and model picker (2026-10-19)

The composer now lists detected local agents and the configured custom command, with provider headings and selectable models instead of the complete registry. Anthropic and ChatGPT retain their agent icons. Settings keeps the installation registry and adds per-agent Show in composer / Hide from composer controls. Search matches provider, agent and model names. The menu reuses the existing popup rows and the same component across home and workbench.

Model names and values come from ACP session configuration or the original Reference model cache. No speculative model names are embedded. The bounded, validated 1 MiB cache is loaded on the catalog worker and refreshed by agent workers; rendering performs no cache filesystem reads. Writes are serialized, atomic and skipped when the catalog is unchanged. Opening the picker prepares only the selected agent, without sending a prompt. An agent without cached choices offers Load models. Original cache import preserves an existing Prism file.

Cached selections use a semantic model key that resolves to the connected adapter's actual option ID. Startup restoration applies the model first and skips unchanged values. Unsupported choices produce an explicit rejection. The provider picker, existing model chip and slash options share pending selection, persistence and rejection handling. Complete modified methods and immediate dependencies were re-read for responsibilities, ownership, errors and session lifetime.

Formatting, locked strict all-target/all-feature Clippy, the optimized release build and the standalone MCP-only check passed. The one existing bridge secret-handling test passed. No dependency, test, fixture or harness was added. The rebuilt 20.70 MiB macOS bundle passed deep strict signature verification.

Native UI inspection confirmed Anthropic and ChatGPT headings with the saved model lists, search filtering for Luna, selection of 6 Luna reflected in the model chip and saved preferences, and hiding Claude removing its entire Anthropic group. Showing Claude again restored the visibility preference. These checks did not send a chat prompt. Installation detection and cached choices do not prove authentication or entitlement; stale selections are validated when connecting. Anthropic execution, full model-response acceptance and Windows/Linux execution remain unverified.

All 306 Rust files satisfy the source-size limits and match the refreshed inventory hashes. The signed bundle's linker UUID matches the release artifact. The original Unity project, workbench screen, agent configuration, model choices and visibility preferences were restored. One GUI instance remains open, with zero pending operations and no startup error.


## Shared composer and model controls (2026-10-19)

Home, setup and workbench conversations now use one compact composer geometry and one fixed control row. Attachment and access controls stay on the left; the combined model/reasoning selector and Send or Stop stay on the right. Long model names shorten to fit the available width, including space reserved for real context usage. The separate agent chip, slash button, horizontal option strip and alternate narrow-panel toolbar were removed. Inline slash commands and mentions retain their existing completion path. The obsolete standalone slash-menu module was deleted.

Agent switching reuses the configured provider/model content inside the combined selector. Other advertised agent options remain accessible there. The model/reasoning popup retains the stepped native slider, real model rows, reset to advertised Medium and top-left Fast mode toggle. Models without reasoning controls share the same Fast mode control above their model list. Unsupported Fast mode remains disabled rather than synthesizing an option. The composer has no separate toolbar controls for model, reasoning or Fast mode. Onboarding uses the same selector. Access choices retain their actual IDs, permission behavior, descriptions and selection handling; full access receives a warning tint.

Complete modified methods and their immediate dependencies were re-read after formatting for responsibilities, sizing, pending choices, cancellation, session ownership and error propagation. No dependency, test, fixture or harness was added. All 309 Rust files satisfy the source-size limits and match the refreshed inventory hashes. Formatting, strict locked all-target/all-feature Clippy, the optimized release build and the standalone MCP-only check passed. The existing bridge secret-handling test passed. The rebuilt 20.72 MiB macOS bundle passed deep strict signature verification.

Native UI inspection confirmed the same four-control arrangement on Home and a 900 by 760 workbench window, with no second toolbar or standalone model/Fast controls. The shared selector exposed configured ChatGPT model rows, the Medium reasoning slider, Fast mode checkbox, reset and remaining agent options. Clicking Fast mode changed its accessible state from off to on and back. No chat prompt was sent. Prior reasoning keyboard verification confirmed updates and persistence; full live-agent response acceptance, unsupported-agent execution and Windows/Linux rendering were not exercised in this follow-up.

The original Unity project, workbench screen, window geometry and agent preferences were restored. One GUI instance remains open with zero pending operations and no startup error.

## Live working-text shimmer (2026-10-19)

The current turn's working label and latest thinking block now share a subtle horizontal text shimmer. The existing Markdown renderer, selection, context menus and accessibility remain in place. Only visible text schedules animation with a 33 ms interval; unfocused windows, completed replies, historical pages and disabled animations retain static rendering. The effect colors a frame-local copy of visible glyph meshes without changing cached layouts. Inline message copy icons and their reserved action row were removed; right-click Copy message remains available.

The three modified source files and new focused shimmer module received a final readability and lifetime review. Formatting, strict locked all-target/all-feature Clippy and the optimized release build passed. All 310 first-party Rust files satisfy physical source-size limits and match the refreshed review inventory. No dependencies, tests, fixtures or harnesses were added. The 20.72 MiB rebuilt macOS bundle passed deep strict signature verification. Native accessibility inspection found zero inline Copy buttons. The original project, workbench screen and window geometry were restored in one GUI instance with no pending operation or error. Live-agent animation appearance and Windows/Linux rendering were not exercised in this follow-up.

## Early model loading and per-chat selection (2026-10-19)

The catalog worker now publishes the bounded saved model cache before installation detection, registry refreshes or icon downloads. Catalog loading starts with the app. The shared composer uses these models immediately, and the provider picker uses the same selected value. Empty or partial option updates no longer erase a saved model list. No network request or adapter startup is needed to display a cached selection.

A focused storage module adds model choices to the existing chat preferences row, keyed by provider or custom command identity. Restoring a chat loads its choices; sending a turn and receiving confirmed option changes save them. Failed writes restore the prior in-memory selection, and option rejection retains the existing recovery path. New chats use the provider's first advertised model, independent of old global model preferences. Session configuration validates the requested model against live choices and falls back explicitly when it is unavailable. Chats predating this metadata have no recorded model and use the first choice. Other agent options retain their existing persistence.

The affected owners are storage (`chat_models`, database migration and chat restoration), agent session configuration and model caching, and the existing UI catalog, composer options, provider picker, chat updates and prompt preparation. Complete changed methods and immediate dependencies were re-read after formatting for responsibilities, pending-choice ordering, storage errors, recovery and session lifetime. No dependency, test, fixture or harness was added.

Formatting, strict locked all-target/all-feature Clippy, the optimized release build, the standalone MCP-only check and the one existing bridge secret-handling test passed. All 312 first-party Rust files satisfy physical source-size limits and match the refreshed review inventory. The rebuilt 20.74 MiB macOS bundle passed deep strict signature verification. Native accessibility inspection confirmed that the composer displayed the cached first ChatGPT model, 6.1 Sol, immediately after launch without opening the selector or connecting an agent. The database migration preserved all 36 chats. The original Unity project, workbench screen and window geometry were restored in one GUI instance with no pending operation or error. Live-agent per-chat model round trips, uncached provider discovery and Windows/Linux execution remain unverified in this follow-up.

## Slash-only Plan and completion acceptance (2026-10-19)

The combined model/reasoning selector excludes the ACP collaboration-mode option by ID and category. Access menus omit Plan choices while the slash palette retains advertised commands and provider modes. Codex keeps its real `/plan` command; no unsupported command or separate mode UI is synthesized.

The composer captures whether completion was open before TextEdit handles the frame. Return selection remains reserved for the popup even when selection or dismissal closes it. Selecting a skill now expands its existing prompt format into the draft, preserving attachments, rather than calling the immediate-send playbook entrypoint. Advertised slash commands and project mentions retain token replacement, focus and cursor restoration. Explicit Run actions in the Playbooks menu and empty agent panel keep their existing behavior. Popup row rendering moved into the existing picker module; no extra component or dependency was added.

Complete modified methods and immediate dependencies received a final readability review covering keyboard consumption, empty results, Shift-Return, character/byte cursor conversion, action routing, pending option choices and error propagation. Formatting, strict locked all-target/all-feature Clippy, the optimized release build, standalone MCP-only checking and the one existing bridge-secret test passed. All 312 first-party Rust files satisfy physical size limits and match the refreshed inventory. No tests, fixtures or harnesses were authored. The rebuilt 20.74 MiB macOS bundle passed deep strict signature verification.

Native keyboard acceptance remains unverified because accessibility automation did not reliably retain composer focus. An automation click activated a playbook on Prism's own helper binary; that turn was stopped, its temporary chat was backed up and removed using the existing storage deletion sequence, and the original 36 chats and 277 messages were preserved. No Unity analysis was used for this agent interaction. Other provider adapters and Windows/Linux rendering were not exercised in this follow-up.

## Conversation width animation (2026-10-19)

The existing panel-layout owner now separates the requested chat width from its animated display width. The chat, composer, divider and adjacent code pane share that display boundary. Native egui interpolation retargets from the current position during a drag and settles within the configured animation duration, capped at 120 milliseconds. Startup initializes directly at the saved width. Disabled animations display the target directly, and narrow windows still clamp the display without overwriting the saved preference. A width change requests the first animation frame immediately; subsequent animation repaints stop at the target.

The complete modified methods and immediate dependencies were re-read after formatting for stable widget identity, clipping, drag capture, persistence and idle repaint behavior. The change adds no dependency, custom animation cache, test, fixture or harness. Formatting, strict locked all-target/all-feature Clippy, the optimized release build, standalone MCP-only checking and the existing bridge-secret test passed. All 312 first-party Rust sources satisfy the physical size limits. The rebuilt 20.74 MiB macOS bundle passed deep strict signature verification.

Native divider interaction changed the saved width from 400 to 551 points. Accessibility geometry showed corresponding composer widths of 348 and 499 points and complementary changes in the adjacent pane. These checks establish native layout and persistence behavior; animation frame pacing, large active conversations and Windows/Linux rendering remain unmeasured. No prompt was sent, and the 36 chats and 277 messages were preserved.

After a normal restart, the original 552.14453-point preference, Unity project, pseudocode view, workbench screen and 1440 by 940 window geometry were restored. One GUI instance remains open with no pending operation or startup error.

## Generational reconstruction and required batching (2026-10-19)

Reconstruction now belongs to focused Rust modules for generation storage, leased module batches, bounded evidence, atomic source publication, immutable source snapshots, compiler commands and streamed reports. Each root is bound to the selected input SHA-256. Matching generations reuse completed source; language or instruction changes preserve evidence only when the analyzer matches. Sources and evidence share compressed content-addressed objects. Expired leases become eligible again, identical submissions are idempotent, and conflicting source writes require the previous file hash. Build receipts retain the precise source revision and preserve independent export/receipt failures.

The existing worker exposes `decompile_batch` and reference reconstruction tools. Decompilation reuses its loaded engine and existing cache. Evidence pages process at most four functions within a bounded serialized budget; pagination does not advance past deferred functions. Each successful new evidence page saves in one transaction. Shared agent context and the canonical `skills/decompile` and `skills/recompile` playbooks require batch decompilation and reconstruction. The UI continues to use the existing chat and progress owners. All nine bundled playbooks now come from the requested `skills/` directory.

The native MCP surface excludes direct `procedure_pseudo_code` discovery and rejects that name through `analysis_call`, directing agents to `decompile_batch` or leased `reconstruction_evidence`. The loaded backend's internal method remains available to the existing UI code view and batch executor. Managed and Android backends retain their advertised bulk interfaces.

Complete modified methods and immediate dependencies were re-read after formatting for coherent responsibilities, ownership, bounded allocation, cancellation, lease expiry, transaction rollback, source conflict handling, snapshot isolation, independent error propagation and UI state preservation. No dependency, test, fixture or harness was added. Public-source findings and implementation boundaries are recorded in `RECONSTRUCTION_RESEARCH.md` and `RECONSTRUCTION.md`.

Executed verification:

- `cargo fmt --all -- --check` passed.
- Locked all-target, all-feature Clippy passed with warnings denied.
- The existing bridge-secret test passed; no new tests were authored.
- The locked standalone MCP-only check and optimized release build passed.
- All 332 first-party Rust files satisfy the physical source-size limits and match the refreshed SHA-256 inventory.
- Packaging produced `build/Prism.app`, 20.89 MiB, and deep strict signature verification passed.
- Read-only inspection of the packaged MCP catalog confirmed 30 combined tools, 25 window tools and 5 analysis tools, including the generation tools and `decompile_batch` in their intended modes.
- All ten packaged skill files matched the canonical directory, including its nine `SKILL.md` files.

No GUI window, live-agent reconstruction cycle, recovered-program execution or proprietary decompilation was used for this follow-up verification. Cross-platform execution, lease recovery under a killed process, source-conflict and export partial-failure behavior, live-agent orchestration and end-to-end compiler repair remain unexercised. No tenfold speedup or behavioral equivalence has been measured. Current limits also include module ordering rather than SCC scheduling, manual snapshot cleanup, no automatic shared-interface invalidation and no managed incremental cache across build snapshots.

## Activity retention follow-up

2026-10-19. Reaching the activity byte budget previously left no room for new tool identifiers or thinking. Reaching the 2,048-block limit also rejected every new block. The limits now retain a rolling window instead: older blocks leave in a prefix drain, contiguous thinking keeps a UTF-8-safe recent tail, and new tool calls continue to enter. Small eviction batches prevent vector growth at the block cap. Tool field bounds and partial-update handling are shared with child-agent previews.

The total main/child preview payload remains bounded at 256 KiB for translated live updates. Plans, goals and child previews use at most 128 KiB, leaving room for main activity. Each child keeps bounded recent text and tools; aggregate pressure releases older child payloads before metadata and protects the child receiving the latest update. Existing child row order remains stable. These limits exclude message text, container overhead, temporary protocol payloads and renderer allocations. Older evicted previews are not archived separately and previously rejected events cannot be recovered.

Dropped-block counters preserve step totals and stable widget identities. Absolute block cursors keep manual history pages anchored during eviction, while Latest steps resumes following new activity. A saved text boundary prevents evicted interim text from reappearing as the final reply. Added persisted counters default to zero for older records. Retention runs after ordinary activity and progress updates, including idle goal updates, and before finishing a turn. Modified methods and immediate dependencies received a final structural readability, ownership and resource review. No new dependency, test, fixture or harness was introduced.

Formatting, strict all-target/all-feature Clippy, the existing bridge-secret test, the standalone MCP-only check, all source-size limits and all 335 source inventory entries passed. No long-running live-agent rollover was exercised, so the existing test does not establish that runtime behavior.

The optimized release build passed. The rebuilt 20.90 MiB macOS bundle, including the standalone MCP helper and canonical skills, passed deep strict signature verification. The GUI was not launched and no existing chat or agent session was modified for verification.

## Compact skill references (2026-10-19)

The slash picker reused the full playbook prompt as editable draft text. It now replaces only the completion token with a compact `$skill-name` reference and restores the caret immediately after it. Selecting a skill still reserves Return for popup acceptance. Explicit Run actions use the same reference and retain their existing send behavior.

The existing playbook catalog resolves references in the outgoing agent copy. Drafts, queued follow-ups and newly saved user messages retain compact references. Referenced bodies are included once per catalog entry, using the current custom override when present. Disabled references produce an explicit error. Matching requires whitespace or a text boundary around the complete reference, avoiding skill-name prefix collisions. Ordinary requests without references and native slash commands bypass skill resolution. No agent session restart, duplicate skill state, database migration or dependency was introduced. Previously expanded saved drafts are not rewritten automatically; legacy sent-message display remains supported.

Complete changed methods and their immediate dependencies were re-read after formatting for token and Unicode cursor boundaries, disabled-skill handling, transport ownership, queue persistence and adapter command routing. All 336 first-party Rust source files satisfy physical size limits and match the refreshed inventory. Formatting, strict locked all-target/all-feature Clippy, the standalone MCP-only check and the existing bridge-secret test passed. No new test, fixture or harness was authored. Live popup interaction and agent execution have not been exercised for this follow-up.

The optimized release build passed. Packaging rebuilt the 20.90 MiB `build/Prism.app` with the canonical skills and standalone MCP helper. Deep strict signature verification passed. No GUI instance was launched or existing conversation modified for verification.

## Selectable agent replies (2026-10-19)

The shared message renderer now replaces an explicit final choice list with wrapped native reply buttons. It recognizes trailing top-level Markdown bullet and numbered lists after a choice question or invitation. Selecting a button submits that option through the existing prompt preparation and agent session. The current draft, attachments and mentions are moved aside and restored independently of send success, then draft persistence is attempted even after a send error. Both failures use existing notifications. No chat state schema or database migration is needed.

Only the latest completed assistant reply in the current conversation is eligible. Historical pages, failed or stopped turns, active turns, pending permissions and queued follow-ups retain normal Markdown. After submission the reply is no longer eligible, preventing repeated activation while the next turn runs. Recognition is deliberately limited to English choice prompts followed by two to eight simple options, a reply of at most 32 KiB and labels of at most 2 KiB each. Nested lists, checklists, images, HTML and code blocks retain their existing rendering. Unsupported forms remain readable and can be answered through the composer.

The implementation directly uses pinned `pulldown-cmark` 0.13.4, already included by the existing Markdown viewer, with default features disabled and desktop-only activation. No additional dependency package, unbounded retained cache, test, fixture or harness was added. Complete changed methods and their immediate dependencies were re-read after formatting for source offsets, list nesting, Unicode-safe slices, current-turn ownership, duplicate activation, draft recovery and error propagation.

Formatting, strict locked all-target/all-feature Clippy, the existing bridge-secret test and the standalone MCP-only check passed. All 338 first-party Rust files satisfy source-size limits and match the refreshed inventory. Live selection and agent delivery have not been exercised; the existing test does not verify this new interaction.

The optimized release build passed. Packaging rebuilt the 20.90 MiB `build/Prism.app` with the canonical skills and standalone MCP helper, and deep strict signature verification passed. No GUI was launched or existing conversation changed for verification.


## Analysis-worker memory and idle shutdown (2026-10-19)

The second `prism` process is the existing `--native-ghidra` worker, isolated from the GUI for cancellation, memory budgeting and crash containment. It uses the desktop executable, without another packaged runtime. The small `prism-mcp` processes are agent bridge instances and were not the large allocation owner. The screenshot's historical process IDs had already exited when inspected, so their exact allocations could not be sampled directly.

The pinned Ghidra object builder retained a second full executable buffer after initializing its loader. The adapter now saves the original description and releases that builder after initialization. The initialized loader, translator, program, symbols and existing per-function teardown remain in use. This adapter does not serialize or reinitialize its native architecture. No third-party source was modified.

The heavier memory reproduction used four large functions in Prism's existing MCP executable, with the previous packaged desktop executable as the baseline. No proprietary binary was decompiled. Although each function's analysis graph was already destroyed after use, macOS retained hundreds of MiB of freed allocation pages. The public pressure-relief call did not materially reduce retention in this workload and was removed. The isolated macOS child now receives `MallocSpaceEfficient=1` before process creation. Apple documents the policy in its [allocator source](https://github.com/apple-oss-distributions/libmalloc/blob/main/src/malloc.c); it favors memory reuse and reclamation over allocation caching. The GUI and other child processes retain their existing allocation settings.

The original four-function reproduction ended at approximately 1 GiB physical footprint and 1,578 MiB RSS. The revised direct worker ended at 41.5 MiB physical footprint and 572 MiB RSS. The final packaged app's actual `decompile_batch` path returned the same four pseudocode strings in 21.1 seconds and ended at 41.0 MiB physical footprint, approximately 96 percent lower retained footprint for this workload. RSS still includes resident pages already made reclaimable, so it does not fall to the footprint value. These are local measurements on macOS 27.2, not universal memory guarantees or an app-wide reduction claim. Peak working memory remains substantial: the packaged batch reached a 913 MiB physical-footprint peak while analyzing a large function. The configured worker memory budget remains enforced; the isolated measurement profile used a 2,048 MiB budget to complete the comparison without changing the user's settings.

Engine ownership now records completion of actual backend operations. The worker waits only for the remaining idle interval and checks expiry even when unrelated jobs keep arriving. Static native tool catalogs and cached storage reads do not refresh that interval. In the single GUI window using a temporary support directory, the native child exited after 61.4 seconds while 21 cached function-list requests continued. Four cached control-flow replies remained identical and did not reopen it. An uncached analysis-info request restarted one child in 0.081 seconds. Existing process-group termination, child waiting and reader-thread joins remain responsible for teardown. The sixty-second reuse grace period, disk analysis caches, cancellation and project switching are preserved.

Live verification uncovered an existing MCP validator omission: every array argument was rejected, including the advertised decompilation batch. The existing validator now checks array shape, minimum and maximum item counts and individual item schemas. The packaged app accepted a four-function batch and rejected empty batches, 65-item batches and non-string procedure items before dispatch. This repairs the advertised interface without introducing another tool or bypassing validation.

Complete changed methods and immediate dependencies were re-read after formatting for coherent responsibilities, explicit ownership, monotonic idle timing, error propagation, bounds, cancellation and deterministic cleanup. No dependency, test, fixture or harness was authored. Executed checks passed: formatting; locked all-target/all-feature Clippy with warnings denied and undocumented unsafe blocks checked; the existing bridge-secret test; the locked standalone MCP-only check; optimized release build; all 338 physical Rust source-size limits and refreshed SHA-256 inventory; and deep strict signature verification of the rebuilt 20.90 MiB macOS bundle.

The temporary GUI was closed before restoring the normal app, preserving one GUI window. The original Unity project reopened with no pending analysis or startup error. Read-only counts remained 33 projects, 37 chats and 285 messages. No live agent turn, proprietary decompilation, Windows/Linux run, memory-pressure stress run or long reconstruction session was exercised. Allocation-policy effects on older macOS versions and peak memory for other targets remain unmeasured.


## Agent question cards (2026-10-19)

Prism now exposes `request_user_input` on its combined and window MCP surfaces. The typed input accepts a short title, a question and up to eight distinct choices, with free text always available. Answers return structured `answered`, `skipped` or `dismissed` results; selected choices include a one-based index, while custom replies use a null index. Agents receive instructions to wait for the answer and never interpret skip or dismissal as approval. The existing view permission category applies to this UI operation. The analysis-only catalog does not advertise the tool.

The shared conversation layout places one question card above the existing composer on Home and in the workbench. It contains a question heading, close action, wrapped numbered choice rows, a custom-response field, Skip and Send. Clicking a choice sends it immediately, preserving the previously requested behavior; typed replies use Send or Enter. Recognized final Markdown choice lists now use this same card. Dismissed inferred choices remain readable as normal Markdown. The ordinary draft, attachments and mentions remain independent. Pending questions are transient and do not change the database schema.

The MCP helper keeps reading protocol messages while a question waits. It bounds active question requests and serializes output. The authenticated window bridge handles a bounded number of concurrent requests so a pending question cannot block other calls. Helpers carry their stable chat identity in the environment, not in model-supplied arguments. Switching tabs keeps a question with its owning chat when background agents remain enabled. Stop, chat disposal, helper cancellation, connection closure and app shutdown release pending response channels. Socket polling preserves partial reads and imposes a one-hour question deadline. No permission question is silently answered by these cleanup paths.

The existing pinned Serde package is now shared by the desktop and standalone helper; no dependency package, test, fixture or harness was added. Complete modified methods and immediate dependencies were re-read after formatting for bounded state, explicit ownership, chat routing, cancellation, output serialization, permission classification, independent cleanup and UI readability.

Executed validation passed: formatting; locked all-target/all-feature checking; strict all-target/all-feature Clippy with warnings denied and undocumented unsafe blocks checked; the existing desktop bridge-secret test (one test); the standalone MCP-only check; the optimized release build; all 345 physical Rust source-size limits and refreshed SHA-256 inventory; and deep strict signature verification of the rebuilt 21.00 MiB macOS bundle. The packaged helper advertises 31 combined tools, 26 window tools and five analysis tools. Only the combined and window catalogs include `request_user_input`.

Live verification used the packaged app and helper with an isolated temporary support directory. Other MCP requests remained responsive while a question waited. Native accessible choice clicks returned the expected one-based indices; typing a custom answer and pressing Enter returned a null index without starting an ordinary agent turn. Skip and close returned their distinct structured statuses. Duplicate choice labels, more than eight choices and whitespace-only questions were rejected before display. A second question in the same chat was rejected while the first remained pending. Protocol cancellation and helper input closure cleared pending questions. App shutdown released a waiting caller with an explicit closing error, and the GUI and helper exited successfully. The final native preview shows all three short options fully; longer content scrolls within the same card.

The isolated window was closed before restoring the normal app. The original target was restored with no pending request, active agent, pending question or startup error. Read-only counts remained 33 projects, 37 chats and 285 messages. No live model-driven invocation, parked-chat switch, narrow workbench interaction, Windows/Linux run or one-hour expiry was exercised. The existing test does not establish those behaviors. Question state remains transient across app restarts.

## Chat readability and appearance (2026-10-19)

The existing Settings → Chat page now owns saved chat appearance controls: System or Monospace text, 12–22 pt text, independent 11–20 pt code, 120–190% line height, 4–24 pt paragraph spacing, 12–48 pt message spacing, and optional block colors. Existing settings receive defaults of 15 pt text, 13 pt code, 150% line height, 10 pt paragraph spacing and 24 pt message spacing. Values are validated before settings load/save, and updates use the existing debounced persistence path. The code-view preferences remain independent. The existing settings page width, controls and hidden scrollbars are reused.

A scoped chat style is passed explicitly through messages, activity history, plans, child previews and tool details. Common Markdown paragraphs, headings, inline formatting, lists, code and quotes use actual text line heights. User messages use compact, content-sized bubbles aligned to the right. Assistant replies remain on the chat surface, code blocks use neutral backgrounds, and quotes, thinking and tool details use quiet leading lines instead of colored panels. The optional colors apply to syntax and small accents; disabling them selects neutral accents and the existing Quiet syntax palette. Inline formatting is laid out as one paragraph so spaces, selections and wrapping continue through code and emphasis. Tool details retain their bounded scrolling area. The existing activity paging, default 16,000-byte preview, explicit full-message expansion, message-copy menu and working shimmer remain available. The live settings preview calls the production message renderer and includes its real, initially collapsed activity disclosure, rather than separate sample thinking and tool cards. Link-containing groups and complex Markdown such as tables, images, footnotes and definition lists remain with the existing CommonMark viewer, preserving its link interaction and accessibility. Its wrapped-line height follows its own layout; chat font sizes still apply. Highlighting reuses the existing lightweight syntax lexer and does not claim language-specific semantic analysis.

Implementation is confined to the conversation settings, shared message/activity rendering and the existing settings-page modules. Six focused source modules were added. No new dependencies, tests, fixtures or harnesses were added. Complete modified methods and their immediate dependencies were re-read after formatting for clear responsibilities, safe UTF-8 preview boundaries, explicit style ownership, validation, persistent defaults and bounded default activity rendering.

Executed verification: locked all-target/all-feature cargo check; strict all-target/all-feature Clippy with warnings denied and undocumented unsafe blocks checked; the existing desktop bridge-secret test (one test passed); the standalone MCP-only check; the optimized release build; formatting; and physical source-size/inventory checks across all 351 first-party Rust files. The macOS bundle was rebuilt at 21.02 MiB and passed deep strict signature verification. No new tests were authored.

After the normal agent completed, GUI review used one isolated temporary support directory and the packaged app. Captures confirmed compact user bubbles, initially collapsed activity, neutral code surfaces, correct spacing around inline code, dark/light appearance, and larger saved text/code settings with colors disabled. The final previews were inspected at a 1,000-point window width. Saved settings were edited only in the isolated profile while its window was closed, then loaded across restarts; native slider actions and expanded activity interaction were not established by this review. No live model turn, narrow workbench conversation panel or Windows/Linux UI was exercised. The preview uses the actual production message and activity components, but the inspected conversation was sample content.

The review window exited before the normal app was restored on the new build. Its original target and Home screen were confirmed with no error, pending operation, active agent or pending question. Production counts remained 33 projects, 37 chats and 287 messages. The temporary profile and private restore records were removed after successful restoration. Production settings and conversation records were not edited by the review commands.

## Prism reasoning colors (2026-10-19)

The existing reasoning selector now paints its filled slider track with a blue, cyan, green, yellow, orange and rose spectrum. One seven-pixel texture is cached in the egui context and sampled proportionally, keeping each color at the same track position as the selection changes. The reasoning title, selected Fast mode icon and keyboard focus ring reuse the existing theme accent instead of hard-coded purple. Filled-track stop markers use a dark translucent color, while unfilled markers retain the theme color. The native stepped interaction, drag preview, commit behavior, model link and reset action remain in their existing components.

The change is confined to `src/ui/agent_options/reasoning.rs` and its slider module. No dependencies, tests, fixtures or harnesses were added. Complete modified methods and immediate dependencies were re-read after formatting for readability, texture ownership, theme handling and preserved interaction. Formatting, locked all-target/all-feature checking, strict all-target/all-feature Clippy, the optimized release build, all 351 source-size limits and the refreshed SHA-256 inventory passed. The rebuilt 21.02 MiB macOS bundle passed deep strict signature verification.

The idle production GUI was closed normally and restored on the new bundle with its original target and Home screen. Read-only project, chat and message counts were unchanged, with no pending operation, active agent, pending question or error. The private recovery record was removed after confirmation. The popup's rendered appearance and native slider interaction were not exercised; Windows and Linux were not run.

## Required MCP batches (2026-10-19)

The worker previously hid and rejected direct `procedure_pseudo_code` calls only when Ghidra was selected. Discovery and execution now use the same provider-independent rule, closing the Rizin and REA bypass. Internal batch readers and interactive UI code views retain their existing provider calls. Reconstruction continues to require leased module evidence and an atomic submission containing every leased unit's result.

MCP initialization in all three server modes and normal desktop agent context now share one batch instruction contract. Every built-in skill that requests pseudocode requires `decompile_batch` or the generation claim/evidence/submit workflow, including one-item batches for focused selections. Failed or unavailable batch commands must be reported rather than bypassed with individual requests, UI exports or independent source writes. Indexed Android methods retain the existing jadx batch reader; managed work requires advertised bulk or paginated type/method operations. These instructions guide agents; they do not prevent an external tool or shell from operating outside Prism's MCP boundary.

The five modified Rust modules and their immediate dependencies were re-read after formatting for provider-independent enforcement, library/desktop visibility, preserved initialization and question handling, bounded context and unchanged internal reader ownership. All 351 Rust source files satisfy physical size limits and match the refreshed SHA-256 inventory. Formatting, locked all-target/all-feature checking, strict all-target/all-feature Clippy, the standalone MCP-only check and the existing bridge-secret test passed. No dependency, test, fixture or harness was added.

The optimized release build passed. Manual MCP requests confirmed the shared instructions and unchanged tool routing in combined, window and analysis modes (31, 26 and 5 tools). The rebuilt 21.02 MiB macOS bundle passed deep strict signature verification, and all nine packaged skills matched their canonical files. A direct pseudocode request to the updated desktop was rejected with both supported batch paths before analysis. No actual decompilation, reconstruction generation, live agent turn, Rizin/REA execution or Windows/Linux run was performed for this follow-up.

The idle production app exited normally and was restored with its original target and Home screen. Project, chat and message counts remained 33, 37 and 287. No error, pending operation, active agent or pending question remained. The private recovery record was removed after successful restoration.

## Rust Markdown library (2026-10-19)

Chat messages now use pinned `egui_commonmark_extended` 0.28.3 with its pulldown parser for CommonMark, GitHub-flavored tables, lists, links and code blocks. It replaces the custom Markdown and inline renderers and the older CommonMark dependency. Table and image widths follow the available chat width; the library owns table wrapping, row measurement and overflow. Saved chat font sizes, line height and paragraph spacing configure the shared viewer. The existing activity renderer and lightweight tool-payload highlighter remain in their existing owners.

When syntax colors are disabled, a borrowed-or-owned display copy removes fenced-code language hints before rendering. Stored messages, copied text and agent context retain the original Markdown. Default message previews remain bounded to a UTF-8-safe 16,000 bytes, with explicit full-message expansion. The viewer does not retain a new per-message cache across history. Optional downloads, math, diagrams and heavy syntax features are disabled. The new dependency's backend and emoji lookup dependencies are recorded in Cargo.lock and included in packaged license notices; no runtime process or webview was introduced.

MCP initialization and desktop session context share response-format instructions requesting valid CommonMark, real paragraph breaks, language-tagged code fences and concise tables. Long function inventories are requested as sections and lists. Existing batch instructions, question handling, editable drafts and slash command routing are preserved. These instructions guide responses and do not guarantee a model will always produce valid Markdown.

Complete modified methods and immediate dependencies were re-read after formatting for clear responsibilities, Unicode-safe source offsets, preserved raw content, explicit style ownership and bounded preview state. No test, fixture or harness was authored. Formatting, locked all-target/all-feature checking, strict all-target/all-feature Clippy with warnings denied and undocumented unsafe blocks checked, the standalone MCP-only check, the existing bridge-secret test and the optimized release build passed. All 350 first-party Rust sources satisfy physical source-size limits and match the refreshed SHA-256 inventory. The rebuilt 21.71 MiB macOS bundle passed deep strict signature verification. Manual initialization requests confirmed both shared instruction contracts in all three MCP modes.

The packaged app's existing Settings → Chat preview was captured and inspected in its production window. Inline code, a syntax-colored code block, the compact user bubble and collapsed activity render through the shared components. This preview does not contain a table. Narrow table layout, native appearance controls, light mode, a live agent turn and Windows/Linux were not exercised for this change.

The idle production app was closed normally and restored on the rebuilt bundle with its original target and Home screen. Read-only counts remained 33 projects, 37 chats and 287 messages. No pending operation, active agent, pending question or startup error remained. Production settings and conversation records were not edited by review commands, and the private restoration records were removed after confirmation.

## Responsive Markdown tables (2026-10-19)

The pinned Markdown library intentionally retains column widths across pane changes, including its previous resolved widths after recomputing a proposal. This left tables narrow after opening a wider conversation view. Its header-only minimum widths also allowed long body identifiers to break into fragments. Prism now opts into a small documented patch of that same library, vendored from the exact 0.28.3 package. The upstream default remains unchanged. Each responsive table refits when its content, typography or physical-pixel pane budget changes; otherwise manual column adjustments persist. Words and inline identifiers set minimum widths using their actual body or monospace font. Tables that cannot fit those minimums retain horizontal scrolling.

The policy uses the existing bounded width fit instead of the expensive height optimizer during resizing. A single signature per table invalidates the existing table state; it does not allocate a new widget identity for each width or clear unrelated egui state. Reserved row heights and rendered rows use the same widths. The shared message renderer enables the policy for Home, the workbench and settings previews. No extra layout, runtime dependency package, process, test, fixture or harness was authored. Existing upstream tests were preserved. The vendor's separate validation lock records its dev dependencies; the root Cargo.lock still controls the production dependency graph. License packaging now includes path dependencies and third-party notices, verified in the signed app bundle.

Validation passed: formatting, locked all-target/all-feature checking, the standalone MCP-only check, strict all-target/all-feature Clippy with warnings denied and undocumented unsafe blocks checked, the existing bridge-secret test, all 92 existing upstream Markdown-library tests, and the optimized release build. The first attempted bridge-test filter matched no tests; the correct existing test was then run and passed. Dependency-package selection initially failed because the vendored crate is not a workspace member; its existing tests passed through its standalone manifest. All 350 first-party Rust sources satisfy the physical source-size limits and match the refreshed inventory. Python packaging scripts compile, and the 21.72 MiB macOS bundle passes deep strict signature verification.

The idle production window was closed normally and restarted with the same target and saved conversation. Captures of that conversation in the narrow workbench panel and wider Home layout confirm refitting columns and intact class identifiers. Review used the existing window and did not submit an agent prompt. The original Workbench screen was restored. Read-only database counts remained 33 projects, 37 chats and 287 messages, with a successful quick integrity check. Continuous native window/sidebar dragging remains unverified because macOS was locked; Windows/Linux, light mode and live streaming were not exercised. No resize-frame timing or universal responsiveness claim is made.

## User-message padding (2026-10-19)

Temporary geometry logging in the existing saved conversation reproduced the later-bubble drift. The pinned Markdown renderer appended paragraph spacing to its horizontal wrapping cursor after the final paragraph, expanding a short bubble approximately 10.1 points beyond its available right edge. That expansion also changed the alignment bounds for following messages. Flooring measured text widths in the viewer made exact-fit short text wrap unnecessarily. The shared bubble renderer now rounds its measured width upward, keeps its content left-aligned inside the right-aligned frame and respects the narrow column's width budget. The vendored viewer skips spacing after the final paragraph; separation between paragraphs retains its existing behavior. The vendor patch record documents this additional integration.

The same four saved user messages were rendered in the 688-point Home column and 368-point workbench column before and after the change. All eight observed bubble bounds now end at the column's right edge, with 14-point horizontal inner padding, 10-point vertical inner padding and the existing 16-point conversation gutter. Short messages lost their unnecessary extra line. The production Settings → Chat preview was captured and visually reviewed. No prompt was sent and no test, fixture, harness, dependency or alternate chat component was authored. Temporary geometry logging and log files were removed. Final source and immediate dependencies were re-read for frame ownership, rounding, terminal Markdown events, bounded previews and preserved copy behavior.

Formatting, locked all-target/all-feature checking, strict all-target/all-feature Clippy with warnings denied, the standalone MCP-only check, the existing bridge-secret test, all 92 existing upstream Markdown-library tests and the optimized release build passed. All 350 first-party Rust files satisfy physical source limits and match their inventory hashes. The rebuilt 21.72 MiB macOS bundle passed deep strict signature verification. An initial temporary instrumentation build used the wrong response field; it was corrected before gathering measurements.

Review reused one application window at a time. The packaged app exited normally before inspection. One idle unbundled instrumentation process was terminated after a name-based Quit missed it; the final instrumentation process exited through a standard process-targeted Quit event. The packaged app was then reopened with the same target and Workbench screen, displaying a saved draft. Read-only database counts immediately before restoration were 33 projects, 37 chats and 287 messages; the later snapshot was 33 projects, 38 chats and 288 messages. Both quick integrity checks passed. Review commands did not send prompts or write conversation records. The initial restored state had no error, pending operation or active agent. Native drag resizing, light mode, live streaming and Windows/Linux were not exercised for this change; macOS was locked during the layout investigation.

## Native analysis persistence (2026-10-19)

The built-in Rust Ghidra worker now saves durable architecture state: symbols,
types, processor context, comments, string knowledge, function/data provenance,
function-start seeds, discovery queues and failure records, and the completed
direct-call index. Restore initializes the image loader and processor translator,
then restores this knowledge without repeating symbol loading or function
discovery. Temporary function IR and translator caches are excluded. Existing
compressed views and reconstruction evidence/generations retain their existing
storage and batch-only MCP paths; an uncached function still requires its actual
decompilation. This change applies to the built-in Rust engine, not external REA,
Java Ghidra or Rizin sessions.

Snapshots live under the support folder's `analysis-state`, keyed by the selected
image's SHA-256, pinned engine revision and schema. They use streamed Zstandard
compression with a checksum, a 512 MiB uncompressed file limit, bounded decoding,
atomic replacement and a per-image operating-system lock. Corrupt or incompatible
snapshots produce an error and remain available for recovery. They are counted
with saved projects in Storage and survive clearing disposable code views. They
are retained without the view cache's LRU eviction; total snapshot disk usage is
not globally capped.

Initial loading, completed indexing, explicit discovery and creation of the call
index force a checkpoint. Other state-changing requests checkpoint at their
completion when at least ten seconds have elapsed since the previous save. Normal
worker release forces a final checkpoint before terminating and reaping the
process, with a private cancellation token and a thirty-second cleanup bound.
Idle engines still expire after sixty seconds. A crash or forced kill retains the
last successfully completed checkpoint, not necessarily work in the current
request. Existing projects acquire a full snapshot the next time their native
engine is opened; previously discarded live state cannot be recovered retroactively.

The pinned dependency is now a documented vendor copy because restoring private
Program bookkeeping cannot be implemented through its previous public API. The
optional serde adapter remaps native symbol IDs by address. Manual round trips
also exposed an upstream empty-scope encoder/decoder mismatch, corrected without
changing decompilation algorithms. Only `program.rs`, `database.rs`, the manifest
and the new snapshot adapter differ from the frozen compiled sources. Processor
specifications, licenses and notices are preserved.

Manual verification used separate headless production worker processes and an
isolated support folder with Prism's own existing 537,232-byte arm64 MCP executable.
The initial 978-function inventory survived restart. Discovery resumed from one
scanned function to two, then to three, retaining its pending queue. A saved caller
index returned the same 33 sites from 16 functions after restart, with no repeated
call-index progress and no scan failures. A concurrent worker was rejected by the
snapshot lock. Making the private snapshot folder unwritable caused an explicit
save error and left the previous file's SHA-256 unchanged; restoring access allowed
the same live state to save and restore successfully. The snapshot including the
call index was 83,274 compressed bytes for this sample; this is not a large-project
memory or performance measurement.

Complete changed methods and immediate dependencies were re-read for ownership,
separated loading/restore/checkpoint responsibilities, partial failures,
cancellation, teardown and temporary allocation lifetime. Formatting, locked
all-target/all-feature checking, strict Clippy with warnings denied, the standalone
MCP-only check, the existing bridge-secret test and the optimized release build
passed. No test, fixture or harness was authored. All 352 first-party Rust source
files satisfy their physical size limits. Cross-platform runtime restore, other
processor families, large binaries, forced corruption or process death during a
save, live UI idle expiry, and actual decompilation/reconstruction after restore
were not exercised in this follow-up. The production GUI and chats were left running.

The rebuilt 21.81 MiB macOS bundle passed deep strict signature verification.
Its packaged notices include the vendored dependency's original license and
notice. The disposable verification profile was removed after the final restore;
the rebuilt application has not replaced the currently running process.

## Decompilation request scope, 2026-10-19

The shared MCP/chat instructions and eight analysis skills previously switched to
reconstruction leases whenever a generation existed. The native worker already
dispatched `decompile_batch` separately from `reconstruction_*`; the unwanted
switch came from agent guidance, not a worker redirect. A manual initialize and
tools/list exchange with the previous packaged helper confirmed that it advertised
the state-based switch and directed existing generations to reconstruction evidence.

The shared instructions, analysis/reconstruction tool descriptions, all eight
analysis skills and the recompile skill now choose their workflow from the current
user request. Plain decompilation, inspection and explanation retain bounded
`decompile_batch` requests even when saved reconstruction work exists. Claim,
evidence and atomic source submissions remain the workflow for explicitly
requested reconstruction or source ports. The explicit reconstruction UI prompt
and worker behavior are unchanged. No dependency or public tool schema changed.

The three modified Rust files and their immediate MCP/chat consumers were re-read
for consistent request scope, unchanged dispatch, batch limits and error handling.
Formatting, locked all-target/all-feature checking, strict Clippy with warnings
denied, the existing bridge-secret test and the optimized release build passed.
All 352 first-party Rust files satisfy their physical size limits and inventory
hashes. No test, fixture or harness was authored.

Manual initialize/tools/list exchanges with the rebuilt combined, window and
analysis helpers confirmed the current-request guidance, corrected descriptions
and removal of the old generation-based switch. All nine packaged skills match
their source hashes. The rebuilt 21.81 MiB macOS bundle passed deep strict signature
verification. No live agent turn was run, so model adherence to these instructions
is not established by the protocol inspection. The running GUI and chats were
left untouched; relaunch the rebuilt application to load the updated instructions.

## Shared function addresses during inventory import, 2026-10-19

The native engine enumerates function symbols, and valid aliases can share a code
address. Storage instead identifies functions by `(project,address)`. Its strict
insert aborted the entire import when a second alias reached that key. Native
paging retained one immutable inventory between requests; the reproduced failure
did not require concurrent indexing or a damaged database.

A manual invocation of the previous release's `--native-ghidra` interface opened
Prism's existing, independently built debug executable in a disposable support
directory. Its inventory contained 128,347 entries at 128,288 unique addresses,
including 38 addresses with multiple entries. At `0x1018d397c`,
`std::fs::File::sync_data` and `std::fs::File::sync_all` shared code. Executing the
production insert against these captured rows in a disposable SQLite database
reproduced `UNIQUE constraint failed: functions.project, functions.address`.
The failed transaction rolled back to zero rows.

Native inventory construction now sorts addresses stably and keeps the first
entry at each address before paging and reporting its count. The underlying
program and saved analysis retain their symbols and aliases. The shared importer
also handles repeated `(project,address)` keys within and across batches through
a targeted conflict clause, retains the first name, and counts actual inserts.
Other SQL failures still propagate. The one-million-row limit applies to unique
stored functions. Existing inventory replacement and its completion marker remain
in one transaction, with all callers propagating import errors before commit.
No database reset, schema migration, dependency or analysis cache invalidation
was introduced.

Executing the updated production insert with the same captured inventory saved
128,288 rows. Repeating it inserted zero rows, the first alias name was retained,
and SQLite `quick_check` passed. This manual SQL verification does not exercise
the desktop's Rust importer end to end. No new test, fixture or harness was
authored, following the standing instruction against test authoring. Unity was
not analyzed in this implementation environment, and the running GUI and user
data were left untouched.

Both complete modified methods and their immediate inventory, paging and storage
consumers were reviewed for deterministic naming, unique counts, statement
lifetime, bounded memory, error propagation and transactional rollback.
Formatting, locked all-target/all-feature checking, strict Clippy with warnings
denied, the existing bridge-secret test and standalone MCP-only checking passed.
All 352 first-party Rust sources satisfy the physical size limits and review
inventory hashes.

The optimized release build and deep strict signature verification passed for the
rebuilt 21.81 MiB macOS bundle. Its native worker restored the existing captured
analysis (`analysis_state.restored=true`) and returned 128,288 functions across
502 pages of 256 entries, with zero repeated addresses and a consistent advertised
total. These returned pages also imported through the original strict insert
without a constraint failure. Restore plus pagination took 119.81 seconds on this
large binary, which remains a separate performance limitation; this fix makes no
restore-speed claim. The disposable capture, database and support directory were
removed after the verification worker exited normally. The production GUI was
not relaunched, and Unity-specific end-to-end indexing remains unverified.

## Cached string state preventing function views

Every uncached function could fail with a bare native `syntax error` when opening
its view started the worker and restored saved analysis. A read-only structural
XML validation of the affected checkpoint found 51 unclosed `string` elements
inside `stringmanage`. No analysis payloads were displayed. The pinned vendor's
string-cache encoder opened each entry and closed only its `bytes` child.

The encoder now closes the entry. The snapshot adapter also recognizes that
specific legacy shape and inserts the missing closures within the string-cache
section before decoding. It preserves all payloads and requires matching entry
and byte-payload counts with no existing entry closures. Valid snapshots remain
unchanged, and unrelated malformed XML remains an error with saved-state context.
Recovery does not remove production checkpoints, rerun discovery, or change any
decompilation algorithm, MCP batching policy, public boundary, or dependency.

The old bundled worker reproduced the exact `-32000: syntax error` while restoring
a private copy of the affected checkpoint. The corrected release restored that
same copy with `analysis_state.restored=true`. An intentionally invalid function
address was rejected before analysis; the normal error-path checkpoint then
exercised the corrected writer. The saved XML was well formed. All 193,192
Program-owned function records, all 51 cached string payloads, the data records,
seeded addresses, call index, and discovery state matched the original. Only
structural validation results and counts were displayed; no target function was
decompiled or decoded. The original checkpoint and running GUI were untouched.
Large-state restoration still took several minutes; this fix makes no speed claim.

Formatting, locked all-target/all-feature checking, strict Clippy, the existing
bridge-secret test, the MCP-only check, and the optimized release build passed.
All 352 first-party source files still match their size and hash inventory; the
snapshot adapter is 174 lines. The rebuilt 21.83 MiB macOS bundle passed deep,
strict signature verification. Modified methods and their immediate XML/string
encoding and checkpoint dependencies received a final readability and lifecycle
review. No tests, fixtures, or harnesses were authored. Interactive function views
in the running GUI were not exercised because that process retains the old build.

## Optional macOS loader mode

Apple's published dyld source at commit
`fd8d0c4d52320ebf64db34f3cb280310d905c5ae` derives `progname` from the
executable launch path and enables `proactivelyUseWeakDefMap` for a `MATLAB`
prefix on macOS. The startup path then creates and fills a weak-definition map
before binding. This is an undocumented C++ weak-symbol optimization, not a
general ten-percent startup guarantee. Sources:
[process configuration](https://github.com/apple-oss-distributions/dyld/blob/fd8d0c4d52320ebf64db34f3cb280310d905c5ae/dyld/DyldProcessConfig.cpp)
and [loader startup](https://github.com/apple-oss-distributions/dyld/blob/fd8d0c4d52320ebf64db34f3cb280310d905c5ae/dyld/dyldMain.cpp).

The macOS bundler now accepts `--matlab-loader`. This packages the release
desktop executable as `MATLAB-Prism`, points `CFBundleExecutable` at that file, and
preserves the Prism display name, identifier, resources and MCP helper name.
A relative `prism` symlink preserves existing executable paths without duplicating
the binary. Native workers continue to use `current_exe`; no wrapper or relaunch
is added. Bundle-size reporting excludes symlinks. Default bundles retain the
ordinary `prism` executable because the measured change was a regression.

On macOS 27.2 build 26B5101f, the existing release desktop executable was launched
with `--native-ghidra` and closed stdin under both names, using temporary hard
links to identical signed bytes. After five warmups per name, 80 paired launches
alternated the order. Normal median/mean times were 8.443/8.546 ms; prefixed
median/mean times were 9.784/9.882 ms, a 15.89 percent median slowdown. The normal
10th/90th percentile samples were 7.600/9.313 ms, versus 8.562/10.625 ms with the
prefix. This measures warm process startup and idle-worker exit, including
process creation, framework initialization and shutdown. It does not measure
cold launch or window readiness and cannot establish a percentage for other Macs
or OS versions. No target project, analysis payload or production state was read.

Both packaging modes completed using the existing optimized release binaries.
The default bundle and separate `build/matlab-loader/Prism.app` passed deep,
strict signature verification and each remained 21.83 MiB without counting the
compatibility symlink twice. In the optional bundle, both executable paths
completed the idle-worker command successfully, and the unchanged MCP helper
returned its normal help text. Bundle metadata retained the Prism identity.
Python syntax compilation, `cargo fmt --all -- --check`, and the existing
`cargo test --locked --all-targets --all-features` checks passed. No new tests,
fixtures or harness files, dependencies, or Rust source changes were introduced.
The packaging operations and immediate callers received a final review for
resource ownership, executable paths, staged publication and error propagation.
No GUI was opened or restarted; Finder/Dock appearance was not exercised.

## Saved-analysis restore range traversal

Profiling a private copy of the saved native checkpoint identified repeated
whole-tail partition-key copying in `Database::property_keys`. Each symbol's
property update cloned all later keys before filtering to the affected range.
The worktree now splits the same start and exclusive-end partitions, then uses
a bounded mutable B-tree iterator to update flags in place. This removes the
temporary key list and traversal of unrelated partitions. The snapshot format,
cache identity, decompilation algorithms and public MCP contracts are unchanged.
The changes are confined to vendor `partmap.rs`, `database.rs` and their patch
record. Existing first-party source files are unchanged.

An optimized profiling build with symbols retained restored the private copy
in 18.536 seconds to the successful open reply, or 18.608 seconds including
normal worker shutdown. The reply reported `analysis_state.restored=true` and
the worker exited successfully. Observed peak worker RSS was 1,062 MiB. This
was a fresh worker with a warm filesystem cache, not a cold-disk measurement.
The copied 10,486,973-byte checkpoint retained SHA-256
`a197cfe6552a51bb096d038f66651b45c069a8c4a6194dd546a1409948bf95f0`.
The earlier comparison workers were stopped before completing, so an exact
before/after speedup ratio is not established.

The follow-up sample is dominated by sorted-vector insertion/memory moves in
`RangeMap::insert_hint`, including inlined insertions under symbol restoration.
That sample identifies the next index-construction candidate; it does not
measure a proposed fix. No block-container, bulk-load or binary-snapshot change
has been implemented. Restore is still not instant, and no first-uncached-view
or complete-export timing is implied by the successful open reply. No target
function body was decompiled or displayed during this measurement.

The complete modified range methods and their partition/address dependencies
were reviewed for exclusive-end behavior, final-address handling, ownership,
error paths and readability. `cargo fmt --all -- --check`, strict locked
all-target/all-feature Clippy, the existing all-target/all-feature tests, the
MCP-only check, and the normal stripped release build passed. The existing test
run contains one bridge test; it does not provide focused coverage of range
semantics. No new tests, fixtures or harnesses were authored. A physical-line
check of all 352 first-party Rust files found no size violations. This Prism
repository has no OpenUnity architecture/source-size checker scripts.

The normal `build/Prism.app` was rebuilt and passed deep, strict signature
verification at 21.83 MiB logical bundle size. The running GUI and the separate
optional MATLAB-named bundle were not replaced or relaunched. Interactive
behavior in that existing process therefore remains unverified for this patch.

The [performance roadmap](PERFORMANCE_ROADMAP.md) captures the five-agent
research, source comparison with original Reference, remaining bottlenecks and
proposed implementation sequence. Those proposals introduce no dependencies
or runtime changes in this research pass.

## Durable decompilation and modular reconstruction

The 2026-10-19 implementation pass used separate agents for saved-state indexes,
durable storage, module contracts, generation identity, cache provenance,
documentation and independent review. Changes stay in the Prism worktree; the
unrelated OpenUnity checkout and dirty original Reference source were not modified.
No dependencies, tests, fixtures or test harnesses were added in this pass.

The vendored range index now stores entries in bounded sorted blocks with a
cumulative rank directory. Blocks start small and split before reaching an
insertion that would exceed 1,024 entries. Local element moves are bounded;
directory updates still cost work proportional to the number of blocks.
Indexing, overlapping ranges, split/merge behavior and live restore queries
received a source review. Snapshot serialization and cache identity are unchanged.

The same private copy of the 10,486,973-byte checkpoint restored in fresh
release workers in 3.37, 2.83 and 2.85 seconds including normal shutdown. Replies
reported `analysis_state.restored=true`. Peak RSS was 934,920,192–935,297,024
bytes, approximately 892 MiB. The earlier successful sample was 18.608 seconds
and 1,062 MiB. These observations suggest a substantial improvement, but the
earlier optimized profiling binary retained symbols while the newer release is
stripped; filesystem caches were not forced cold. This is not an isolated
benchmark of build profile or disk I/O. The checkpoint hash was unchanged:
`a197cfe6552a51bb096d038f66651b45c069a8c4a6194dd546a1409948bf95f0`.
No target function body was decompiled or displayed. Native restoration is
still neither instant nor low-memory; first uncached function latency and
whole-project throughput were not measured.

`src/decompilation/` owns job identity, lifecycle and project-wide OS locking.
`src/storage/decompilation/` owns frozen inventory, bounded result pages and
transactions that commit compressed artifacts with cursor advancement. Jobs
start paused, activate under the lock and require explicit resume after restart.
Foreground work yields between functions; project/configuration changes pause
the export. Reused successes advance only the cursor. Individual artifacts are
bounded to 512 KiB before compression and jobs have an 8 GiB compressed-artifact
quota. These are storage/work bounds, not an application RSS ceiling.

The existing worker runs durable jobs without an additional service. Strict
batch/evidence reads first reuse matching durable artifacts with verified hashes,
then configuration-scoped cached views. They exclude legacy upstream UI fallback
entries. Database failures propagate; corrupt artifacts become cache misses.
The configuration includes provider paths, native revision and analysis options;
it does not fingerprint external executable contents or versions. Old
unqualified view-cache entries intentionally miss once. Clearing that cache does
not erase durable output.

`src/reconstruction/` now persists module dependencies, iterative SCCs and ready
interface/ABI contracts. Claims observe dependency readiness; contract/graph
changes invalidate affected consumers and advance the generation revision.
Language, model, prompt, adapter and validation identities have explicit reuse
boundaries. Unknown model identity conservatively invalidates source on a fork.
Contracts retain their complete payload when copied. Validation actions never
carry forward as new successful validation. Source retained for merge remains
stale until units are regenerated; a successful compiler command alone does not
report a complete reconstruction or behavioral equivalence.

Language commands reuse installed external toolchains. Rust, TypeScript and
Python default to checked results; artifact-producing defaults cover C/C++, C#,
Swift, Go, Maven Java and Zig. Other targets require an explicit command and
appropriate ABI/runtime contract. The UI supplies the selected model identity
and references installed skills without copying their full text into the draft.
Both source and bundled skill copies enforce batch work and modular output.

After formatting, modified methods and immediate dependencies were reviewed for
ownership, transaction ordering, cursor checks, lock lifetime, recovery,
invalidation and readability. The inventory records 376 first-party Rust files,
including `build.rs`, with no physical-line limit violations. Source hashes and
line counts match [RUST_FILE_REVIEW.md](RUST_FILE_REVIEW.md). This Prism repository
does not contain OpenUnity's architecture or source-size checker scripts.

Validation completed successfully:

- `cargo fmt --all -- --check`.
- Strict locked all-target/all-feature Clippy, including undocumented-unsafe-block
  linting.
- Existing locked all-target/all-feature tests: one bridge test passed; other
  targets contain no tests.
- MCP-only `cargo check` with default features disabled.
- Latest-source release builds of `prism` and MCP-only `prism-mcp`.
- Read-only MCP initialization/tool listing, including durable lifecycle actions
  and reconstruction module/dependency/contract tools.
- Normal macOS packaging and deep, strict signature verification. The bundle is
  21.99 MiB logical size at `build/Prism.app`.

Existing vendor tests remain unavailable because the vendored tree omits
`tests/data/bitfield/listsort_expected.txt`. No replacement fixture was authored.
The existing app test does not cover range semantics, durable restart/recovery
or cross-language reconstruction. Live full-job restart, cancellation during
native work, complete 200,000-function throughput and end-to-end generated builds
remain unexecuted acceptance checks. The running GUI was not restarted, so the
new package has not been interactively exercised in that process.


## Analysis-worker RAM reduction

The 2026-10-19 follow-up used two implementation agents and a separate read-only
reviewer. It removes duplicate executable storage, replaces the saved XML DOM
with bounded source-range indexes, retains compact function handles instead of
copied inventory names, and keeps object symbol names as ranges into the shared
executable. Parser and decoder changes preserve the existing snapshot format,
legacy string-element repair, architecture decoding and idle worker teardown.
The `snapshot` feature directly reuses pinned `quick-xml` 0.41.0, already in the
dependency graph; no new package or background service was introduced.

The coordinator and reviewer inspected complete changed methods and immediate
dependencies for buffer lifetime, lock ordering, Program identity, alias ordering,
UTF-8/entity handling, parser bounds, error propagation and resource release.
The four first-party caller modules retain the existing architecture. Seven
focused vendor modules are at most 171 lines; existing vendor roots received
narrow wiring changes. All 376 first-party Rust files satisfy their size limits
and have current inventory hashes. OpenUnity's architecture/source-size scripts
are not present in this separate Prism worktree.

### Release measurements

These are fresh native worker processes restoring the same private copy of the
10,486,973-byte checkpoint for a 118,864,128-byte ARM64 executable. The baseline
was copied from the previous stripped release immediately before this RAM work;
the final binary uses the same release profile and `MallocSpaceEfficient=1`.
No compiler was running during either measurement. OS file caches were not
forced cold. Both replies reported `analysis_state.restored=true`.

| Worker measurement | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| Peak RSS during restore (`time -l`) | 868.75 MiB | 543.56 MiB | 37.4% |
| Peak physical footprint during restore (`time -l`) | 828.77 MiB | 522.36 MiB | 37.0% |
| Retained physical footprint after inventory (`vmmap`) | 476.1 MiB | 410.4 MiB | 13.8% |
| Live malloc allocations after inventory (`vmmap`) | 2,083,452 | 1,515,756 | 27.2% |
| Restore plus normal shutdown | 3.13 s | 3.10 s | Approximately unchanged |

Inventory materialization returned 193,191 indexed functions. The first-page
response exactly matched the baseline, including names, ordering and metadata.
The final live-worker run also peaked near 543.61 MiB RSS / 522.39 MiB physical
footprint. Live-worker wall time includes deliberate waiting and is excluded
from the restore-time comparison. The original user checkpoint retained SHA-256
`a197cfe6552a51bb096d038f66651b45c069a8c4a6194dd546a1409948bf95f0`.
Metadata-only RPC replies were used; no target function body was decompiled or
shown in this implementation session.

The biggest reduction is temporary restore memory. The live native Program,
executable and symbol/type state still consume hundreds of MiB while loaded.
These measurements do not establish a whole-app memory ceiling or memory usage
for a complete 200,000-function decompilation/reconstruction job. Independent
checkpoint sections and further native-state compaction remain future work.
The normal 60-second idle shutdown remains intact; cached views can still be
served without retaining the native worker.

Validation completed: root and vendor formatter checks; strict locked
all-target/all-feature application Clippy; the existing application test suite
(one bridge test passed); MCP-only checking; desktop and MCP-only release builds;
metadata-only saved-state restoration and matching inventory response. No new
tests, fixtures or harnesses were written. Vendor test execution remains blocked
by the absent `tests/data/bitfield/listsort_expected.txt` compile-time fixture.
Live initial analysis, renamed-symbol enumeration, malformed checkpoint behavior
and complete batch workload memory were reviewed in source but not exercised by
a dedicated behavioral test.

The final normal macOS package at `build/Prism.app` is 22.04 MiB logical size
and passed deep, strict code-signature verification. Packaging included current
desktop and MCP-only release binaries and dependency license notices. The live
GUI remains on its previously launched optional bundle; it was not restarted or
replaced during measurement. All temporary profiling workers exited normally.

## Final batch and reconstruction memory profile

The current physical-footprint and durable-workload results are recorded in
[BATCH_MEMORY_PROFILE.md](BATCH_MEMORY_PROFILE.md). The saved-metadata profile
reduced physical restore footprint from 411.0 to 289.4 MiB, while the 1,024-
address durable batch remained effectively throughput-neutral (+0.6%) and
retained one known RSS-budget failure. The production reconstruction profile
reduced native lifetime peak from 83.03 to 62.86 MiB for 256 evidence items.

These are profile-specific measurements. RSS and physical footprint account for
mapped and shared pages differently, and the host samples were always taken
after headless support because the old comparison binary lacked that mode.
The original Arena remains selected after paged and dense alternatives were
slower. Final formatting, strict Clippy, application tests (one existing test),
application/MCP checks, release builds and deep strict macOS signature
verification passed. All 377 first-party sources meet their size limits and
match the refreshed inventory. The 22.07 MiB package retains the measured
desktop executable byte-for-byte. Vendor tests remain blocked by the missing
bitfield fixture; no tests, fixtures or harnesses were added. The live GUI was
not restarted. Full validation details are in the linked profile.

## Decompile only analysed functions

The 2026-10-19 follow-up separates explicit native analysis from inventory reads.
The built-in engine no longer discovers functions when listing entries or
registers unknown entries when reading pseudocode, assembly or local flow data.
Ordinary pseudocode and interactive batches validate the saved SQLite index
before cache access or worker startup. Durable batches and reconstruction
evidence retain their frozen inventory authorization after later reindexing.
Explicit analysis force-checkpoints before its inventory is imported.

The decompile skill and shared MCP instructions require indexed selections,
batch calls and explicit user authorization for additional discovery. The twelve
modified Rust files retain their existing owners and add no dependencies. Their
complete methods and immediate dependencies received a final readability review;
independent review found no remaining blocker in the built-in path.

Real production MCP calls used an isolated copy of Prism's existing analysis of
its own executable. Four indexed functions succeeded in interactive and durable
batches. Both native and persisted inventories remained at 20,104 functions,
with no discovery pass recorded. An unindexed interior address was rejected
before native worker startup; the native assembly endpoint also rejected it.
After restart, all four saved results loaded without starting a native worker.
The operational record is `/tmp/prism-analyzed-only-20261019/verification.json`.

Formatting, strict all-target/all-feature Clippy, the existing application test
(one bridge test), MCP-only checking and desktop/MCP release builds passed.
All 377 first-party Rust files satisfy their existing source-size limits.
An initially mistyped MCP check used a nonexistent `mcp` feature; the corrected
`--no-default-features --bin prism-mcp` check and build both passed. No tests,
fixtures or harnesses were authored. Reindexing during a frozen job and stripped
binary discovery were reviewed in source but not exercised in this operational
run. Optional external providers retain their own initialization behavior; this
change establishes the no-discovery guarantee for the built-in engine. The
running GUI was not restarted.

The updated 22.07 MiB `build/Prism.app` passed deep, strict signature verification
and contains the current decompile skill. Signing changes the embedded signature
size and corresponding Mach-O link-edit size fields; both packaged executable
payloads match their release builds after excluding only those signature fields
and signature data.

## Worker reuse, responsive durable reads, and Reference comparison

The 2026-10-19 follow-up distinguishes synchronized provider errors from broken
RPC exchanges, preserves usable native workers, and pauses durable jobs when
startup or restore fails. Read-only durable list/status/results use short
runtime snapshots and read-only database connections outside the analysis queue.
Worker lifecycle publication, project transitions, and unchanged result limits
were reviewed together. The native snapshot decoder reserves restored generated
function identities before allocating other symbols; already-corrupt snapshots
are not automatically repaired.

Complete changed methods and immediate dependencies received structural,
ownership, error-propagation, cancellation, and cleanup review. A separate
GPT-6.1 Sol/max reviewer found no actionable issue in these changes. The 24
changed or new first-party Rust files retain existing module ownership; the new
runtime module owns only shared context publication. All 378 first-party Rust
files satisfy the existing physical line limits. The source inventory was
refreshed while preserving earlier review notes for untouched files. New vendor
identity modules are separately recorded in `vendor/ghidra-decompiler/PRISM_PATCH.md`.

The operational comparison reused the existing production MCP client with
private support directories and Prism's own executable. For the same 256
indexed functions, both builds produced 256 successful, digest-identical
artifacts and no failures, without expanding the 20,104-function inventory.
Median status response fell from 58.01 ms to 1.45 ms. Observed batch completion
was 6.75 s before and 7.46 s after; this pass does not establish faster native
decompilation or lower batch RAM. Ordinary-error worker reuse and actual child
failure recovery were exercised independently.

After host restart, completed status and four results loaded in 10.6 ms without
starting a sampled native worker. A harder continuation remained incomplete at
61.6 seconds, with 34 successes, no failures, and a 500.39 MiB sampled native RSS
peak; orderly shutdown left 37 committed successes in a paused private job.
This is an explicit remaining performance and resource limit, not a successful
full-job benchmark. Details, sample counts, physical-memory measurements,
artifact hashes, and reproduction paths are in
[REFERENCE_PERFORMANCE_COMPARISON.md](REFERENCE_PERFORMANCE_COMPARISON.md).

Executed successfully:

- `cargo fmt --all -- --check`.
- `cargo clippy --locked --all-targets --all-features -- -D warnings -W clippy::undocumented_unsafe_blocks -W clippy::redundant_clone -W clippy::needless_collect`.
- `cargo test --locked --all-targets --all-features` (one existing test).
- `cargo check --locked --no-default-features --bin prism-mcp`.
- `cargo build --release --locked --jobs 2 --bin prism`.
- `cargo build --release --locked --no-default-features --bin prism-mcp`.
- Physical first-party source-size and inventory validation.
- `python3 scripts/bundle-macos.py --skip-build`.
- `codesign --verify --deep --strict build/Prism.app`.

The vendor implementation agent also ran snapshot-feature compilation and
format checks. No dependency, test, fixture, or harness was authored. The live
GUI and the user's job were not restarted. The rebuilt package remains 22.07
MiB. The three detailed Reference comparison reports are source audits: no menu
frame-latency measurement, completed 200k-function export, reconstruction
throughput claim, or cross-platform validation is implied.

## Optional Reference backend and integrated responsiveness follow-up

Implemented the user's requested `reference-ghidra` Cargo feature with the explicit
`ghidra-rea` provider. Default builds keep the native Rust engine. Feature builds
show Ghidra (Java through REA) in Engines and use configured REA, Ghidra and JDK paths.
The macOS bundler accepts `--reference-ghidra`; external tools remain outside the
22.15 MiB application bundle. No dependencies were added.

Explicit opening prepares the external session once. Durable decompilation
uses frozen saved addresses in provider windows of at most 256 and retained
pages of at most 64, with address/order/coverage checks before persistence.
Reconstruction reuses verified durable external pseudocode. Native and external
results have distinct configuration identities, calculated at preparation or
backend configuration changes rather than on each worker tick. Backend changes,
shutdown, cancellation and unusable sessions preserve committed results and
require explicit preparation before continuing. Stock REA can still analyze
when its archive is unavailable during an explicit reopen; no cache-only
restart guarantee is advertised.

The integrated work also removes model-popup session preparation, caches a
revisioned option snapshot, bounds transcript rendering and permission/request
admission, removes duplicate durable view persistence, and streams native
checkpoints. Complete modified methods and immediate dependencies were reviewed
for ownership, errors, cancellation, state transitions, allocation and structural
readability. Separate Sol 6.1/max implementation and review agents handled the
owners. Final independent reviews found no remaining actionable findings.
The source inventory now contains 418 first-party Rust files, all within the
existing physical limits, with refreshed hashes and preserved earlier notes.

Actual measurements and remaining limits are recorded in
[RESPONSIVENESS_AND_BATCH_MEASUREMENTS.md](RESPONSIVENESS_AND_BATCH_MEASUREMENTS.md).
The native protocol ping delay dropped from 453.13 ms to 0.42 ms under the
measured batch load. Native decompiler throughput and RAM changed little.
The real Reference backend completed 320/320 own-binary functions; the final release
completed 512/512 across a pause and process restart, retaining 65 prior successes.
No complete large-engine export, AI reconstruction speedup or GUI frame-latency
result is claimed. The external process tree can use substantially more RAM
than its configured Java heap alone.

Executed successfully on the final integrated source:

- `cargo fmt --all -- --check`.
- `cargo clippy --locked --offline --all-targets --all-features --jobs 2 -- -D warnings -W clippy::undocumented_unsafe_blocks -W clippy::redundant_clone -W clippy::needless_collect`.
- `cargo test --locked --offline --all-targets --all-features --jobs 2` (one existing test).
- `cargo check --locked --offline --jobs 2`.
- `cargo check --locked --offline --no-default-features --bin prism-mcp`.
- `cargo build --release --locked --offline --features reference-ghidra --jobs 2 --bin prism`.
- `cargo build --release --locked --offline --no-default-features --bin prism-mcp`.
- First-party physical source-size and review-inventory verification.
- Python syntax parsing of the updated macOS bundler.
- `python3 scripts/bundle-macos.py --skip-build --reference-ghidra`.
- `codesign --verify --deep --strict build/Prism.app`.

No new tests, fixtures or test harnesses were authored. Operational measurements
reused the existing temporary profiling client on Prism's own binaries in private
support stores. The live GUI, user settings and user analysis jobs were not
changed, and no proprietary binary analysis was performed.
