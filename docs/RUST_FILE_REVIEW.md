# Rust file review

> Backend snapshot notice, 2026-10-20: macOS and Linux desktop builds now
> select Rust Ghidra through REA by default. Java Ghidra remains available in
> builds that enable `prism-ghidra-rea`. The older Prism-hosted `native_ghidra`
> worker remains removed. See
> [current provider setup and limits](REA_RUST_PROVIDER.md).
> Historical source inventories and measurements below remain unchanged and
> do not validate the current sources.

Reviewed on 2026-10-18, with follow-ups on 2026-10-19, using the invoked rust-skills rules. This inventory covers all first-party Rust source files, including the standalone MCP binary and permission settings. Each file was inspected for ownership, errors, resource lifetime, allocation patterns, work on the UI thread, and maintainability. Changed methods and their immediate dependencies received a final readability review after formatting. Retained files did not need a speculative rewrite.

The storage follow-up adds five focused metadata-scan and settings modules, reuses the existing settings controls, and refreshes its callers. Shared tabs and compact radio controls remain in use. The resize follow-up retains OpenGL with VSync enabled after rejecting the Metal comparison. The project-switching follow-up reduces temporary sidebar index work and unnecessary inventory joins in the two navigation storage modules. The 2026-10-19 chat-history follow-up adds bounded message paging and per-chat navigation, preserves live session state, and keeps database IDs out of serialized projects. The conversation-composer follow-up aligns its toolbar with Codex, adapts the controls to narrow panels, and shares chip measurements. The attachment follow-up adds path-based drafts, capability-aware image delivery and targeted native drops. The 2026-10-19 backend and chat follow-up shares native cached results across UI/MCP, maintains cache accounting incrementally, adds bounded queued follow-ups and bundles Prism-owned playbooks. The inventory now contains 299 Rust files.

The final 2026-10-19 chat-protocol follow-up adds bounded plans, usage, child previews, advertised commands and current-project mentions. Modified methods and immediate dependencies were re-read for ownership, error handling and readability. Prism branding now shares one compact transparent glass-prism image. The home-composer follow-up removes screen-specific styling branches so home, setup and the conversation panel reuse the same component directly. These checks do not establish live-agent parity; limitations remain recorded in `RUST_REVIEW.md`.

Whole-tree checks cover every compiled target. They do not establish complete feature parity or behavioral equivalence for unexercised features. Third-party dependency sources and license notices were preserved, rather than rewritten. Build outputs, operating-system metadata and Python bytecode are excluded.

The persistent-session follow-up adds live option controls, saved per-agent choices, Codex metadata, goal snapshots and idle event delivery. The approval follow-up adds bounded project-specific answers and edit suggestions in the existing composer. Modified methods and immediate dependencies were re-read after formatting for cancellation, late event routing, bounded state and error recovery.

The platform-icon follow-up changes the shared artwork to an opaque black-backed sRGB master. Its new build entrypoint embeds the Windows desktop icon only for that target and propagates resource-compiler failures. The optional build dependency is excluded from MCP-only builds and adds no runtime dependency. The entrypoint and resource-library interface received a readability and error-propagation review. Windows resource compilation remains unverified on this Mac. Including `build.rs`, the Rust source inventory now contains 300 files.

The macOS Dock follow-up selects a separate rounded black icon at compile time for macOS. Native window configuration is separated from command dispatch; PNG decoding still propagates errors, and the data lock still covers the complete GUI lifetime. The existing compact square icon remains selected on Windows/Linux. No additional runtime image cache or dependency was introduced.

The attachment-picker follow-up removes the custom attachment popup and unused location input. The plus button and slash palette share the existing rfd picker. macOS accepts files and folders together with multiple selection; other targets retain file selection and folder drag/drop. Attachment validation, draft persistence and image delivery remain in their existing owners. The changed methods and immediate dependencies were re-read; no native adapter, dependency or test was added.

The inline-completion follow-up retains slash and mention queries in the shared composer, replaces the token at the caret and reuses existing menu rows. Agent preparation discovers advertised capabilities without creating a chat or sending a prompt. Stable UI chat keys preserve the prepared session for the first saved turn. Raw slash commands bypass the ordinary context prefix; Codex session instructions retain project context and restored history. Preparation is bounded and deduplicated, with the existing idle shutdown retained. Modified methods and immediate dependencies were re-read for focus, Unicode cursor boundaries, selection, session lifetime and error propagation. The Rust inventory now contains 303 files.

The configured-model-picker follow-up adds borrowed provider groups, shared searchable model rows and a bounded worker-owned catalog. Settings controls composer visibility without removing registry setup. Model selections resolve against live ACP options and share existing persistence and rejection recovery. Modified methods and immediate dependencies received a final readability review. The current inventory contains 306 Rust files.

The 2026-10-19 composer follow-up retains one compact control layout across screens and panel widths. Agent selection and all model, reasoning and Fast mode controls share the combined selector. The obsolete standalone slash menu was removed while inline commands and mentions remain available. Access menus retain their actual permission contracts. Modified methods and immediate dependencies received a structural readability review. The current source inventory contains 309 Rust files.

The working-text follow-up adds a native glyph-color shimmer for the current turn only. Rendering retains Markdown, text selection and accessibility. Visible text requests at most 30 animation frames per second while focused; completed messages and history pages remain static. Inline copy actions and their reserved row were removed while the message context menu remains. The complete modified methods and immediate dependencies were re-read for clipping, cached galley ownership, pending state and idle repaint behavior. The current source inventory contains 310 Rust files.

The model-loading follow-up preloads bounded cached catalogs before registry refreshes and uses them in the shared composer. A focused storage module preserves model selections per chat and provider; session configuration defaults to the first advertised model while retaining explicit chat choices. Existing option confirmation and rejection recovery remain in place. Changed methods and immediate dependencies received a final readability review. The source inventory now contains 312 Rust files.

The completion follow-up excludes collaboration mode from the combined selector and Plan choices from the access menu, preserving the slash palette. Enter selection keeps commands, mentions and expanded skills in the draft. The shared picker owns row rendering and keyboard acceptance. Modified methods and immediate dependencies were re-read for event consumption, cursor boundaries, focus, action routing and error propagation. The source inventory remains 312 Rust files.

The conversation-width follow-up uses egui's existing interpolation in the panel-layout owner. Chat contents and the adjacent code pane share one animated boundary while persistence retains the requested width. Modified methods and immediate dependencies were re-read for widget identity, clipping, drag capture, narrow-window constraints, disabled animations and idle repaint behavior. The source inventory remains 312 Rust files.

The generational-reconstruction follow-up adds persistent module batches, renewable leases, compressed shared objects, source/evidence reuse, revision-specific builds and bounded reports. Decompilation uses the loaded worker and paginated batches; successful evidence pages save in one transaction. Canonical skills come from `skills/`. Modified methods and immediate dependencies received a structural readability review for resource ownership, optimistic file conflicts, expired leases, cancellation, independent failure reporting and UI state preservation. No dependency, test, fixture or harness was added. The current inventory contains 332 first-party Rust files. Behavioral equivalence and a tenfold speedup remain unverified.

The activity-retention follow-up keeps current tool calls and thinking visible after byte or block caps by evicting older previews. Shared tool bounds, child retention, stable page cursors and reply-text offsets received a final readability and ownership review. No dependency, test, fixture or harness was added. The current inventory contains 335 first-party Rust files. Long-running live-agent rollover remains unverified.

The compact-skill and selectable-reply follow-ups keep skill bodies in the outgoing transport copy and reuse the shared message renderer for reply choices. The latest completed turn alone can submit a choice through the existing session; unsent drafts, attachments and mentions survive sending and error recovery. Markdown source offsets, bounded parsing, button wrapping and independent persistence failures received a final readability review. The current inventory contains 338 first-party Rust files. Live choice interaction remains unverified.


The 2026-10-19 analysis-worker memory follow-up releases redundant input ownership after native initialization, applies the measured macOS allocation policy only to that child, and expires idle engines independently of cached storage requests. Batch-array validation now accepts the already advertised shape while enforcing its limits. Complete modified methods and immediate dependencies received a structural readability, ownership, error propagation and teardown review. All 338 first-party Rust sources satisfy the existing size limits and match this inventory.

The question-card follow-up adds a shared typed MCP contract, cancellable concurrent requests and one transient question per chat. The same numbered-choice and custom-response component handles tool questions and recognized final reply choices above the existing shared composer. Modified methods and immediate dependencies were re-read for cancellation, chat ownership, bounded state, accessibility, permission classification and shutdown. No test, fixture, harness or dependency package was added. Serde is now shared by the desktop and standalone MCP helper. The inventory contains 345 first-party Rust files.

The 2026-10-19 chat readability follow-up adds independent chat font and code sizes, actual line height, paragraph and message spacing, optional syntax colors and quiet accents, and a shared live settings preview. User bubbles fit their content and align right; assistant replies and activity disclosures retain a quiet hierarchy. Complete changed methods and their immediate dependencies were reviewed for ownership, bounded default previews, continuous inline layout, Markdown preservation and readability. The inventory now contains 351 Rust files. Isolated GUI review covers dark/light appearance and larger text at a narrower window width; detailed verification limits remain in `RUST_REVIEW.md`.

The 2026-10-19 Markdown follow-up replaces the custom message and inline renderers with pinned `egui_commonmark_extended`, preserving saved typography, neutral syntax mode, raw message contents and bounded default previews. Agent response instructions are shared across desktop context and MCP initialization. Complete modified methods and immediate dependencies were re-read for Unicode offsets, style ownership, bounded state and readability. Removing the obsolete inline module leaves 350 first-party Rust files; every source satisfies the size limits and matches this inventory. Verification and visual-review limits are recorded in `RUST_REVIEW.md`.

The responsive-table follow-up enables a documented opt-in patch to the same pinned Rust Markdown library. Tables refit when their pane or content changes, use the actual body and monospace fonts for readable minimum widths, and retain manual column adjustments while the layout remains unchanged. The 350 first-party sources still satisfy the size limits. The newly authored vendor extension, `vendor/egui_commonmark_extended/src/parsers/pulldown/responsive_tables.rs`, is separately reviewed at 49 lines with SHA-256 prefix `b81b83c29fde1224`; unchanged third-party sources are excluded from the first-party inventory. Complete modified methods and immediate dependencies were re-read for width invalidation, row measurement, bounded cache identity, overflow and licensing. Provenance and the limited upstream integration are recorded in the vendor's `PRISM_PATCH.md`.

The user-bubble padding follow-up rounds measured body widths upward, left-aligns the Markdown inside right-aligned frames and removes the minimum width that could exceed a narrow column's budget. The pinned viewer skips spacing after its terminal paragraph, where a horizontal cursor otherwise expands the frame and the following messages' alignment bounds. Complete modified methods and immediate dependencies were re-read for frame ownership, wrapping, terminal-event handling and preserved Markdown, preview and copy behavior. All 350 first-party sources retain valid sizes and inventory hashes; the upstream integration is documented in `PRISM_PATCH.md`.

The native-analysis persistence follow-up adds versioned atomic snapshots and isolated restore ownership. Complete modified methods and their immediate dependencies were reviewed for cancellation, state identity, native ID remapping, failure recovery, temporary allocation lifetime and worker teardown. The current inventory contains 352 first-party Rust files. The pinned Rust decompiler snapshot patch has separate provenance in its vendor directory; runtime verification limits are recorded in `RUST_REVIEW.md`.

The 2026-10-19 roadmap implementation adds durable decompilation identity, lifecycle and storage modules; configuration-scoped cache provenance; dependency/SCC scheduling and contracts; and generation/build identity handling. The coordinator and implementation/review agents inspected changed methods and immediate dependencies for lock ownership, transactional cursor/artifact commits, bounded allocation, invalidation and recovery. This pass refreshed the inventory to 376 first-party Rust files, including `build.rs`; all satisfy the size limits. Retained entries preserve their earlier review outcomes rather than claiming a new full-project review. The vendored range-block change and measured restore improvement are recorded separately in `RUST_REVIEW.md` and its vendor patch record. No tests, fixtures, harnesses or dependencies were added.

The 2026-10-19 RAM follow-up replaces the native session's owned function entries with compact, identity-checked handles and borrows names for bounded output pages. The four changed first-party modules and their immediate dependencies received ownership, invalidation, error propagation and readability review; their inventory hashes were refreshed. All 376 first-party Rust sources retain valid sizes. The vendored compact XML decoder, shared executable buffer and source-backed symbol names have separate provenance in `vendor/ghidra-decompiler/PRISM_PATCH.md`. No tests, fixtures or test harnesses were authored; the snapshot feature directly reuses the existing pinned `quick-xml` dependency.

The worker-reuse and Reference-comparison follow-up reviews 24 changed or new first-party Rust modules and their immediate dependencies. Completed provider errors retain healthy workers; interrupted exchanges retire them. Compact runtime snapshots support read-only durable queries outside the analysis queue. The generated-ID restore repair has separate vendor provenance. An independent Sol 6.1/max review found no actionable issue in these scopes. The refreshed inventory contains 378 first-party files, all within physical line limits; unchanged entries retain their prior review outcomes. Operational results and remaining UI/native performance work are recorded in `REFERENCE_PERFORMANCE_COMPARISON.md`.

The optional Reference backend and responsiveness follow-up contains 418 first-party Rust files. Scoped implementation and independent reviews cover the changed owners; retained rows preserve their prior review outcome. The current measurements and limits are recorded in `RESPONSIVENESS_AND_BATCH_MEASUREMENTS.md` and `REFERENCE_BACKEND_PARITY.md`. File hashes below identify the integrated source reviewed on 2026-10-19.

The 2026-10-20 Rust REA provider follow-up contains 434 first-party Rust files, including build.rs. The feature-gated native worker and Prism integration received implementation and independent source reviews, including frozen inventory, bounded transport, executable and runtime identity, cancellation and checkpoint ownership. Existing provider contracts remain separate. The measurements and executed validation are recorded in `REA_RUST_PROVIDER.md`; source review alone does not establish runtime parity. Unchanged rows retain their prior review outcomes.

| File | Lines | SHA-256 prefix | Review outcome |
| --- | ---: | --- | --- |
| [build.rs](../build.rs) | 17 | `28f6f03b5f5eb2f5` | target-aware desktop icon embedding; explicit errors and bounded Cargo change tracking |
| [src/agent/background.rs](../src/agent/background.rs) | 135 | `c385e837803019d4` | bounds preparation and prompt requests; deduplicates discovery without allocating duplicate session context |
| [src/agent/background/worker.rs](../src/agent/background/worker.rs) | 147 | `29efa70546622ee9` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/catalog.rs](../src/agent/catalog.rs) | 166 | `a8e76bdd5261b96c` | owned text and registry data |
| [src/agent/codex.rs](../src/agent/codex.rs) | 88 | `0ed7fe53d8f9e2f7` | supplies project context and restored history through session instructions while preserving server configuration |
| [src/agent/download.rs](../src/agent/download.rs) | 50 | `4e4789ae0d59e10d` | retained after review |
| [src/agent/install.rs](../src/agent/install.rs) | 95 | `45822aa433f7d7bf` | retained after review |
| [src/agent/launch.rs](../src/agent/launch.rs) | 118 | `931783ae65f09656` | retained after review |
| [src/agent/mod.rs](../src/agent/mod.rs) | 19 | `ad666176363c9642` | focused model catalog module wiring |
| [src/agent/models.rs](../src/agent/models.rs) | 97 | `629d6b26905a55d2` | bounded validated model catalog; serialized atomic worker writes and non-replacing import |
| [src/agent/options.rs](../src/agent/options.rs) | 148 | `89e9c9f318a348d7` | resolves cached models against live ACP option IDs; nullable cache metadata accepted |
| [src/agent/permission.rs](../src/agent/permission.rs) | 21 | `c8068c25210df375` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/permission/pending.rs](../src/agent/permission/pending.rs) | 119 | `658d216d3e1654c3` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/permission/policy.rs](../src/agent/permission/policy.rs) | 61 | `7c44f83a5321ec08` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/permission/request.rs](../src/agent/permission/request.rs) | 105 | `80dc11c7c0c1562b` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/permission/whole_file.rs](../src/agent/permission/whole_file.rs) | 31 | `4c70fc4b65db447a` | recognizes whole-file requests before automatic approval |
| [src/agent/permission_policy.rs](../src/agent/permission_policy.rs) | 119 | `54939286021e9910` | classifies the question card with existing view permissions; preserves explicit permission levels |
| [src/agent/picks.rs](../src/agent/picks.rs) | 128 | `81be355df4677b86` | shared command identity preserves per-agent option persistence |
| [src/agent/prompt_content.rs](../src/agent/prompt_content.rs) | 90 | `86db53e1c41a27ee` | keeps slash commands at the start of the prompt; retains capability-aware attachment delivery |
| [src/agent/remembered.rs](../src/agent/remembered.rs) | 84 | `28df039be9f9c126` | bounded atomic approval storage serializes updates from concurrent chats |
| [src/agent/session.rs](../src/agent/session.rs) | 96 | `790e265f78ed38d1` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/session/configuration.rs](../src/agent/session/configuration.rs) | 151 | `49e1c6431d9e78b1` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/session/control.rs](../src/agent/session/control.rs) | 152 | `686c68d01a4cde2f` | resolves live model controls and skips redundant requests; preserves acknowledgment errors |
| [src/agent/session/exchange.rs](../src/agent/session/exchange.rs) | 113 | `c2e0344aaf2d9828` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/session/startup.rs](../src/agent/session/startup.rs) | 120 | `998d40bfa9d70db1` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/session/turn.rs](../src/agent/session/turn.rs) | 104 | `ab7564a38beaed68` | bounded asynchronous permission admission and correlated replies without prompt input retention |
| [src/agent/tool_servers.rs](../src/agent/tool_servers.rs) | 37 | `1a4ab85040f7f45c` | retained after review |
| [src/agent/tool_updates.rs](../src/agent/tool_updates.rs) | 47 | `13a764f6d919ec01` | shares bounded JSON previews without allocating complete tool payloads |
| [src/agent/updates.rs](../src/agent/updates.rs) | 235 | `a978300e8633a3b0` | publishes merged configuration and refreshes worker-owned model catalogs with explicit errors |
| [src/analysis/android.rs](../src/analysis/android.rs) | 171 | `e1b77cb013e79b4a` | retained after review |
| [src/analysis/android_output.rs](../src/analysis/android_output.rs) | 88 | `be06db58e9ec61ea` | retained after review |
| [src/analysis/engine.rs](../src/analysis/engine.rs) | 106 | `3452ce30fa46bc32` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/analysis/engine/batch.rs](../src/analysis/engine/batch.rs) | 40 | `8670688e066f1075` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/analysis/file_inventory.rs](../src/analysis/file_inventory.rs) | 126 | `065996e4808dc909` | retained after review |
| [src/analysis/graph.rs](../src/analysis/graph.rs) | 106 | `12782f1481ae6f8f` | rendering and bounded history |
| [src/analysis/inventory.rs](../src/analysis/inventory.rs) | 80 | `d2512380d256af84` | retained after review |
| [src/analysis/local.rs](../src/analysis/local.rs) | 113 | `ced579ad985ad9f2` | retained after review |
| [src/analysis/mod.rs](../src/analysis/mod.rs) | 23 | `15c30f860c7a09eb` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| src/analysis/native_ghidra/arguments.rs | 34 | `ce45aa5ddc80cb69` | retained after review |
| src/analysis/native_ghidra/arm.rs | 144 | `2fa50db156640aa0` | retained after review |
| src/analysis/native_ghidra/checkpoint.rs | 132 | `7725b51e37d46a46` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/checkpoint/json.rs | 87 | `becb34dbbf36a495` | streamed checkpoint encoding with checked atomic persistence and unchanged reader format |
| src/analysis/native_ghidra/checkpoint/write.rs | 139 | `6f5eb0c347c24cb9` | streamed checkpoint encoding with checked atomic persistence and unchanged reader format |
| src/analysis/native_ghidra/client.rs | 107 | `8a7a8b1efa007be6` | preserves healthy worker reuse; skips checkpointing an interrupted native protocol |
| src/analysis/native_ghidra/decode.rs | 143 | `e8d497ea3ebe89f6` | retained after review |
| src/analysis/native_ghidra/discovery.rs | 137 | `701ff7f76bc86bcf` | persists and validates bounded discovery continuation and failures |
| src/analysis/native_ghidra/dossier.rs | 134 | `6676492a7b94710f` | compact function handles and borrowed names; reviewed identity, ordering, failure propagation and resource lifetime |
| src/analysis/native_ghidra/flow.rs | 196 | `095116d8975de6f3` | native analysis |
| src/analysis/native_ghidra/function.rs | 82 | `9c2d38c42cc9fb41` | analysed-only decompilation; reviewed inventory boundaries, checkpoint persistence and error handling |
| src/analysis/native_ghidra/function_code.rs | 104 | `87cd99762ba95a15` | native analysis |
| src/analysis/native_ghidra/image.rs | 182 | `0f6a98d33178b0dc` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/inventory.rs | 151 | `b6fe8f0cde4b1003` | analysed-only decompilation; reviewed inventory boundaries, checkpoint persistence and error handling |
| src/analysis/native_ghidra/macho.rs | 104 | `c69a4b2bcfd10f72` | retained after review |
| src/analysis/native_ghidra/mod.rs | 28 | `07222cad25fd019e` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/open.rs | 166 | `1c713fc518095cdc` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/rea_worker.rs | 69 | `bad8b78d658c62b8` | feature-gated worker wiring; explicit protocol, checkpoint and parent-monitor lifecycle |
| src/analysis/native_ghidra/rea_worker/batch.rs | 195 | `eb290da1a8d4eea1` | frozen selection, bounded result admission and opt-in per-function timing |
| src/analysis/native_ghidra/rea_worker/identity.rs | 106 | `10a060b95b6ff5d8` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/rea_worker/inventory.rs | 91 | `4e868e3334bcce79` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/rea_worker/profiling.rs | 67 | `1be4fa8339462b5e` | opt-in scalar timing with original decompile and reporting errors preserved |
| src/analysis/native_ghidra/rea_worker/parent_lifetime.rs | 88 | `ea859a9397074baa` | owned parent monitoring, immediate self-exit on parent loss and joined normal cleanup; independent review accepted |
| src/analysis/native_ghidra/rea_worker/decompile_counts.rs | 98 | `edf9233695e6bb9b` | bounded numeric action and arena observations without raw code or mutation of analysis |
| src/analysis/native_ghidra/rea_worker/protocol.rs | 148 | `0727ab88c5a89d46` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/rea_worker/state.rs | 152 | `328bdd06e5933789` | frozen inventory, bounded worker protocol, explicit resource ownership and interval checkpoint review |
| src/analysis/native_ghidra/references.rs | 72 | `492995e57d19292f` | compact function handles and borrowed names; reviewed identity, ordering, failure propagation and resource lifetime |
| src/analysis/native_ghidra/service.rs | 117 | `4849edcb0b23b484` | analysed-only decompilation; reviewed inventory boundaries, checkpoint persistence and error handling |
| src/analysis/native_ghidra/session.rs | 133 | `50b02e1700115e34` | saved-function-only execution, optional observation and mandatory scratch cleanup with combined failures |
| src/analysis/native_ghidra/tools.rs | 119 | `0e3073efe40c978d` | analysed-only decompilation; reviewed inventory boundaries, checkpoint persistence and error handling |
| [src/analysis/rea.rs](../src/analysis/rea.rs) | 180 | `1a1b08708d4d576e` | feature-gated provider wiring with Rust-only healthy-idle lifecycle cleanup |
| [src/analysis/rea/batch.rs](../src/analysis/rea/batch.rs) | 188 | `cc987f418518f552` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/batch_parse.rs](../src/analysis/rea/batch_parse.rs) | 162 | `b6b1e777417ca524` | feature-gated REA launch, explicit session preparation, bounded batch pages and verified provider identity |
| [src/analysis/rea/batch_state.rs](../src/analysis/rea/batch_state.rs) | 50 | `2dcb10621f7ee7b3` | feature-gated REA launch, explicit session preparation, bounded batch pages and verified provider identity |
| [src/analysis/rea/batch_transport.rs](../src/analysis/rea/batch_transport.rs) | 82 | `2f26378b9fb49ef2` | feature-gated REA launch, explicit session preparation, bounded batch pages and verified provider identity |
| [src/analysis/rea/connection.rs](../src/analysis/rea/connection.rs) | 38 | `9a323793509cac94` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/identity.rs](../src/analysis/rea/identity.rs) | 125 | `a9a4037ca9fddb1d` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/identity/fingerprint.rs](../src/analysis/rea/identity/fingerprint.rs) | 33 | `ff2ac76ceedd36c0` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/java_runtime.rs](../src/analysis/rea/java_runtime.rs) | 114 | `219c0527e8c62e58` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/launch.rs](../src/analysis/rea/launch.rs) | 114 | `23fa2aba5f0f9226` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/reply.rs](../src/analysis/rea/reply.rs) | 114 | `2d60546ef258b8e4` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/rust_package.rs](../src/analysis/rea/rust_package.rs) | 75 | `7aaa293fc70faa13` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/rea/shutdown.rs](../src/analysis/rea/shutdown.rs) | 70 | `545defcc4e6a8054` | correlated close acknowledgement, bounded polling and explicit cleanup failures; independent review accepted |
| [src/analysis/rea/rust_runtime.rs](../src/analysis/rea/rust_runtime.rs) | 30 | `d207ab0166edde65` | feature-gated REA selection, separate artifact identity and owned retained batch lifecycle review |
| [src/analysis/references.rs](../src/analysis/references.rs) | 75 | `2ce01cd61a163a2a` | retained after review |
| [src/analysis/resources.rs](../src/analysis/resources.rs) | 40 | `eabed5986c2106ea` | retained after review |
| [src/analysis/rizin.rs](../src/analysis/rizin.rs) | 137 | `5eaa49dcae52d95f` | reports DAP exchange synchronization for safe worker disposal |
| [src/analysis/rizin_inventory.rs](../src/analysis/rizin_inventory.rs) | 56 | `6fd63c8ff0b69d2c` | retained after review |
| [src/analysis/rizin_tools.rs](../src/analysis/rizin_tools.rs) | 85 | `96c79e03d132e28d` | retained after review |
| [src/bin/prism-mcp.rs](../src/bin/prism-mcp.rs) | 6 | `04a3cebbfea488dd` | retained after review |
| [src/bridge/admission.rs](../src/bridge/admission.rs) | 66 | `ddaadd0c5ae76bac` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/bridge/incoming.rs](../src/bridge/incoming.rs) | 102 | `159b8a52e99bccd9` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/bridge/listener.rs](../src/bridge/listener.rs) | 98 | `eb1ef47e189aacf6` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/bridge/mod.rs](../src/bridge/mod.rs) | 9 | `11c5bc4a7bc22d98` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/bridge/readonly.rs](../src/bridge/readonly.rs) | 40 | `1e5ff49384130740` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/bridge/request.rs](../src/bridge/request.rs) | 131 | `38873e922f6b800c` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/bridge/server.rs](../src/bridge/server.rs) | 158 | `6dd295e4a765dd62` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/bridge/waiting.rs](../src/bridge/waiting.rs) | 69 | `b816c5a5525753d6` | bounded admission, independent durable reads, request deadlines and lifetime cancellation |
| [src/debug/mod.rs](../src/debug/mod.rs) | 3 | `2f9d63be82f6ec8c` | retained after review |
| [src/debug/session.rs](../src/debug/session.rs) | 103 | `1cc5a8d9e29f921d` | retained after review |
| [src/decompilation/identity.rs](../src/decompilation/identity.rs) | 140 | `278ba585f8ef4db8` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/decompilation/inspection.rs](../src/decompilation/inspection.rs) | 110 | `d99fd68f71ee6a8a` | shares validated job identity and bounded read-only inspection |
| [src/decompilation/job.rs](../src/decompilation/job.rs) | 156 | `575868f3804e4ad6` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/decompilation/mod.rs](../src/decompilation/mod.rs) | 11 | `76bc6855d05d0c2a` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/decompilation/ownership.rs](../src/decompilation/ownership.rs) | 30 | `6fd5c33f958c952a` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/decompilation/window.rs](../src/decompilation/window.rs) | 47 | `12c2f745fceab319` | explicit bounded request window with existing durable cursor and page ownership |
| [src/error.rs](../src/error.rs) | 7 | `4ac0b00fdd0d56cb` | retained after review |
| [src/events.rs](../src/events.rs) | 159 | `06c92d2faa1f0654` | reviewed narrow wiring for bounded request or optional executable resolution |
| [src/executable_paths.rs](../src/executable_paths.rs) | 80 | `fd70eb2a9f407e1a` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/headless.rs](../src/headless.rs) | 82 | `bb7c0fb415ac6f2f` | shares worker runtime with the headless bridge without a second analysis engine |
| [src/lib.rs](../src/lib.rs) | 2 | `d9059b414e23f590` | retained after review |
| [src/main.rs](../src/main.rs) | 104 | `fbd0e9431ba96066` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/mcp/catalog.rs](../src/mcp/catalog.rs) | 52 | `05cb4d182ef3a81c` | advertises questions only for window-capable modes; keeps schema construction narrow |
| [src/mcp/catalog/analysis.rs](../src/mcp/catalog/analysis.rs) | 32 | `afcf9dc738a59a36` | Request-scoped batch guidance; decompilation stays separate from explicit source reconstruction. |
| [src/mcp/catalog/decompilation.rs](../src/mcp/catalog/decompilation.rs) | 37 | `e774ed9fa649b53a` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/mcp/catalog/reconstruction.rs](../src/mcp/catalog/reconstruction.rs) | 128 | `f1cf6a12c2b64d37` | Request-scoped batch guidance; decompilation stays separate from explicit source reconstruction. |
| [src/mcp/catalog/reconstruction_modules.rs](../src/mcp/catalog/reconstruction_modules.rs) | 36 | `f1cf6387c4e9cc48` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/mcp/catalog/window.rs](../src/mcp/catalog/window.rs) | 105 | `bd841e4fa456451a` | retained after review |
| [src/mcp/cli.rs](../src/mcp/cli.rs) | 42 | `1e12c5b063184f50` | passes owned output into the concurrent MCP session |
| [src/mcp/configuration.rs](../src/mcp/configuration.rs) | 28 | `ed2f65241c43fd08` | retained after review |
| [src/mcp/decompilation_arguments.rs](../src/mcp/decompilation_arguments.rs) | 58 | `6affb016e1f23cce` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/endpoint.rs](../src/mcp/endpoint.rs) | 122 | `d851a135b51f9105` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/instructions.rs](../src/mcp/instructions.rs) | 51 | `4337837259f7dfb4` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/mod.rs](../src/mcp/mod.rs) | 47 | `45a13173d94d6b4c` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/question.rs](../src/mcp/question.rs) | 82 | `c71104284ddeaa03` | validates bounded distinct choices and text; distinguishes answered, skipped and dismissed results |
| [src/mcp/scheduling.rs](../src/mcp/scheduling.rs) | 60 | `4aa89835e380feaf` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/session.rs](../src/mcp/session.rs) | 121 | `5ea79b9f88fb0d42` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/session/message.rs](../src/mcp/session/message.rs) | 41 | `7c2dedc7604b0caa` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/session/pending.rs](../src/mcp/session/pending.rs) | 207 | `c781f58eb5536af4` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/session/pending/output.rs](../src/mcp/session/pending/output.rs) | 35 | `a847942a85538c80` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/transport.rs](../src/mcp/transport.rs) | 74 | `5518b1ae39d03ff9` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/mcp/validation.rs](../src/mcp/validation.rs) | 116 | `28e55ed99d224827` | bounded asynchronous MCP requests and shared durable selection validation |
| [src/migration/analysis.rs](../src/migration/analysis.rs) | 109 | `d78840938b115da5` | incremental inventory import |
| [src/migration/assets.rs](../src/migration/assets.rs) | 51 | `6b284cdc57fb8d91` | imports bounded model catalogs without replacing existing Prism choices |
| [src/migration/chat_activity.rs](../src/migration/chat_activity.rs) | 116 | `7d96f6b1a40c0260` | imported message identities assigned only by local storage |
| [src/migration/chats.rs](../src/migration/chats.rs) | 107 | `f1448c54c15e650b` | retained after review |
| [src/migration/database_backup.rs](../src/migration/database_backup.rs) | 46 | `53383afc37b613f7` | retained after review |
| [src/migration/mod.rs](../src/migration/mod.rs) | 61 | `587e127886739cf4` | retained after review |
| [src/migration/projects.rs](../src/migration/projects.rs) | 165 | `bd72c4d29f87a2b6` | retained after review |
| [src/migration/snapshot.rs](../src/migration/snapshot.rs) | 200 | `59f899bb2e0ea700` | retained after review |
| [src/migration/types.rs](../src/migration/types.rs) | 25 | `3078e98e3b37b35b` | retained after review |
| [src/model.rs](../src/model.rs) | 156 | `242960f3fadfe50c` | exports goal contracts while preserving serialized project compatibility |
| [src/model/activity.rs](../src/model/activity.rs) | 151 | `0f10ae1a4edb2fec` | accepts live updates after retention limits; preserves completion status and backward-compatible offsets |
| [src/model/activity/retention.rs](../src/model/activity/retention.rs) | 83 | `4b2b4f8dd028eb91` | bounded rolling previews with UTF-8-safe tails, stable block offsets and batch eviction before vector growth |
| [src/model/activity/tool_step.rs](../src/model/activity/tool_step.rs) | 41 | `258233623b077c12` | shared bounded tool fields preserve partial-update state and explicit completion timestamps |
| [src/model/agent_updates.rs](../src/model/agent_updates.rs) | 68 | `4de56573d74f7812` | defines explicit goal, session-closure and rejected-option events |
| [src/model/approval.rs](../src/model/approval.rs) | 119 | `09cee2ab70668642` | bounds approval details and maps project edits to suggestion cards |
| [src/model/attachment.rs](../src/model/attachment.rs) | 88 | `96f9f5ff3361eb23` | bounded path and URL references; escaped message links |
| [src/model/goal.rs](../src/model/goal.rs) | 64 | `d6aaf608cb508b2c` | validates goal metadata and computes a connected active clock or closed snapshot |
| [src/model/json_preview.rs](../src/model/json_preview.rs) | 55 | `8920dca4a2321386` | stops JSON serialization at the preview budget and preserves UTF-8 boundaries |
| [src/model/queued_prompt.rs](../src/model/queued_prompt.rs) | 36 | `a45051e4896a5de9` | bounded persistent queue validates decoded prompts and attachment references |
| [src/model/turn_progress.rs](../src/model/turn_progress.rs) | 142 | `96525e63dadb20b0` | reserves main activity capacity; retains the latest child preview without reordering existing child rows |
| [src/model/turn_progress/child.rs](../src/model/turn_progress/child.rs) | 56 | `28707c6f1d9110b3` | rolling child text and tool previews continue after count and byte limits |
| [src/project/archive.rs](../src/project/archive.rs) | 191 | `17abcec8e89a8a44` | retained after review |
| [src/project/archive_json.rs](../src/project/archive_json.rs) | 69 | `e1f72c30c56b4654` | streamed project metadata |
| [src/project/document.rs](../src/project/document.rs) | 126 | `70f66c20656ebed5` | streamed project metadata |
| [src/project/document_save.rs](../src/project/document_save.rs) | 116 | `8a0fb4a7e56c1151` | streamed project metadata |
| [src/project/files.rs](../src/project/files.rs) | 150 | `505dadfb7c793f9a` | retained after review |
| [src/project/mod.rs](../src/project/mod.rs) | 9 | `0f7465b26f48c5c8` | streamed project metadata |
| [src/project/patch.rs](../src/project/patch.rs) | 69 | `8fefa65286482db6` | retained after review |
| [src/protocol/framing.rs](../src/protocol/framing.rs) | 111 | `9212c33bdafb61b1` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/protocol/json_array.rs](../src/protocol/json_array.rs) | 54 | `877a29f6ab85f7e3` | incremental inventory import |
| [src/protocol/memory.rs](../src/protocol/memory.rs) | 53 | `a30ba7ea4afc5699` | unsafe invariants |
| [src/protocol/mod.rs](../src/protocol/mod.rs) | 10 | `3ee5be07b17544e9` | exports response matching at the protocol boundary |
| [src/protocol/process.rs](../src/protocol/process.rs) | 145 | `3a575b8b26c9b621` | unsafe invariants |
| [src/protocol/response.rs](../src/protocol/response.rs) | 127 | `3d3b79003f71feeb` | bounded RPC framing, reply correlation and pending request lifecycle |
| [src/protocol/rpc.rs](../src/protocol/rpc.rs) | 196 | `83fe8649f04bbbb5` | bounded RPC framing, reply correlation and pending request lifecycle |
| [src/reconstruction/actions.rs](../src/reconstruction/actions.rs) | 58 | `b86c2f37f4ef3977` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/reconstruction/api.rs](../src/reconstruction/api.rs) | 99 | `7c21626935c9d7a7` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/build.rs](../src/reconstruction/build.rs) | 139 | `47b98928ba9fdc7d` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/build_commands.rs](../src/reconstruction/build_commands.rs) | 104 | `06c9deaa408a1768` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/contracts.rs](../src/reconstruction/contracts.rs) | 119 | `b0b551e6a6bf8ebf` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/reconstruction/control.rs](../src/reconstruction/control.rs) | 101 | `a61884090d1df0ca` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/create.rs](../src/reconstruction/create.rs) | 22 | `3d74df4194c9b73d` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/database.rs](../src/reconstruction/database.rs) | 82 | `0ee9ed33fc01ad37` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/evidence.rs](../src/reconstruction/evidence.rs) | 105 | `bd386b58795f481b` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/generation.rs](../src/reconstruction/generation.rs) | 196 | `d0fd2cfb4d5c8c41` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/generation_copy.rs](../src/reconstruction/generation_copy.rs) | 96 | `7e0f62b233661712` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/reconstruction/language.rs](../src/reconstruction/language.rs) | 40 | `4e5b47de4c49d595` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/reconstruction/mod.rs](../src/reconstruction/mod.rs) | 32 | `db02cdb743dc2d82` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/module_inspection.rs](../src/reconstruction/module_inspection.rs) | 46 | `fcf44afd7e44c008` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/reconstruction/modules.rs](../src/reconstruction/modules.rs) | 214 | `af711908455016da` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/reconstruction/objects.rs](../src/reconstruction/objects.rs) | 96 | `52859e08b938b240` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/queue.rs](../src/reconstruction/queue.rs) | 142 | `cfae73d380e788dc` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/report.rs](../src/reconstruction/report.rs) | 42 | `f1c38490fe426189` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/scc.rs](../src/reconstruction/scc.rs) | 84 | `9a04870a60f68b0f` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/reconstruction/schema.rs](../src/reconstruction/schema.rs) | 133 | `6e6ab720402268ac` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/snapshot.rs](../src/reconstruction/snapshot.rs) | 123 | `5f8eef52a7520416` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/status.rs](../src/reconstruction/status.rs) | 156 | `8d0c07a32ec5d5a0` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/submission.rs](../src/reconstruction/submission.rs) | 90 | `fa1c9088c1a0c09c` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/reconstruction/submit.rs](../src/reconstruction/submit.rs) | 224 | `faa9192a24d7ab1a` | reviewed generation ownership, bounded storage, transaction failure propagation and lifecycle contracts |
| [src/settings.rs](../src/settings.rs) | 175 | `52308352bfd1e59b` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/settings/analysis_backend.rs](../src/settings/analysis_backend.rs) | 56 | `4be63981fe591d6b` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/settings/chat_appearance.rs](../src/settings/chat_appearance.rs) | 52 | `b1c8c62dbea8af4e` | validated persistent chat typography, spacing and block-color preferences with backward-compatible defaults |
| [src/settings/conversation.rs](../src/settings/conversation.rs) | 51 | `2a34bcd08304f3d0` | owns the chat appearance preferences through the existing settings contract |
| [src/settings/desktop.rs](../src/settings/desktop.rs) | 53 | `cce39cc064b030fc` | retained after review |
| [src/settings/permissions.rs](../src/settings/permissions.rs) | 98 | `092349ed3ed23cfc` | retained after review |
| [src/settings/tools.rs](../src/settings/tools.rs) | 83 | `933879b868ccc2c5` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/settings/views.rs](../src/settings/views.rs) | 51 | `cfa5f365546ee4e4` | retained after review |
| [src/storage/artifacts.rs](../src/storage/artifacts.rs) | 164 | `c59163c46c172096` | retained after review |
| [src/storage/atomic.rs](../src/storage/atomic.rs) | 40 | `2cb23223ff72ba08` | retained after review |
| [src/storage/cache.rs](../src/storage/cache.rs) | 128 | `886c05ea2caa279c` | transactional payload accounting, indexed eviction and coalesced recency writes |
| [src/storage/chat_list.rs](../src/storage/chat_list.rs) | 206 | `183890df7ffe986a` | chat deletion includes queued follow-ups in the existing transaction |
| [src/storage/chat_models.rs](../src/storage/chat_models.rs) | 74 | `33f761fbf58d9c8f` | Reviewed |
| [src/storage/database.rs](../src/storage/database.rs) | 202 | `edd5b31f81b264a6` | bounded read-only inspection connection avoids migrations and writer ownership |
| [src/storage/decompilation.rs](../src/storage/decompilation.rs) | 7 | `daef119f33c22ae2` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/storage/decompilation/inventory.rs](../src/storage/decompilation/inventory.rs) | 78 | `c12622e1be9252ca` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/storage/decompilation/inventory/window.rs](../src/storage/decompilation/inventory/window.rs) | 36 | `223626a0da7af02f` | validated bound SQL limit; preserves saved selection and successful-artifact skips |
| [src/storage/decompilation/jobs.rs](../src/storage/decompilation/jobs.rs) | 231 | `3e3cfba71dacd214` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/storage/decompilation/pages.rs](../src/storage/decompilation/pages.rs) | 93 | `95b488c04d554256` | reviewed roadmap ownership, persistence, bounded paging and error propagation |
| [src/storage/decompilation/results.rs](../src/storage/decompilation/results.rs) | 55 | `9d3b4cf502a1c4a9` | verified durable artifacts and bounded missing-inventory selection without duplicate view writes |
| [src/storage/decompilation/results/record.rs](../src/storage/decompilation/results/record.rs) | 234 | `0fd14a023da6d9e9` | verified durable artifacts and bounded missing-inventory selection without duplicate view writes |
| [src/storage/functions.rs](../src/storage/functions.rs) | 105 | `5989c22dcdffe518` | analysed-only decompilation; reviewed inventory boundaries, checkpoint persistence and error handling |
| [src/storage/history.rs](../src/storage/history.rs) | 145 | `a9a97b038b9197f0` | updates idle goal activity in place without duplicating saved messages |
| [src/storage/message_page.rs](../src/storage/message_page.rs) | 94 | `3cab040f6b42763b` | indexed bidirectional pages with bounded payload reads and explicit oversized-entry failure |
| [src/storage/mod.rs](../src/storage/mod.rs) | 22 | `458ee4df31ae1658` | wires streamed reconstruction inventory into the existing storage owner |
| [src/storage/navigation/classify.rs](../src/storage/navigation/classify.rs) | 127 | `0c89d3271aa3803e` | retained after review |
| [src/storage/navigation/index.rs](../src/storage/navigation/index.rs) | 110 | `181e2243e1019859` | row-position primary key and bulk address indexing; transactional replacement retained |
| [src/storage/navigation/mod.rs](../src/storage/navigation/mod.rs) | 69 | `5dfb335884c1459a` | retained after review |
| [src/storage/navigation/preferences.rs](../src/storage/navigation/preferences.rs) | 52 | `f281ff377effd441` | retained after review |
| [src/storage/navigation/query.rs](../src/storage/navigation/query.rs) | 210 | `c795f928ba97fcb4` | group and count queries reuse the current temporary catalog; shared visibility predicates |
| [src/storage/reconstruction.rs](../src/storage/reconstruction.rs) | 62 | `38763f08ea8779c2` | streams bounded indexed function inventory directly into generation storage |
| [src/storage/resources.rs](../src/storage/resources.rs) | 134 | `17a6660701ee0beb` | retained after review |
| [src/storage/upstream.rs](../src/storage/upstream.rs) | 145 | `0f40139d691c7a03` | incremental inventory import |
| [src/storage/usage.rs](../src/storage/usage.rs) | 155 | `b01dcbb3ccf0f095` | counts durable native analysis with saved projects rather than disposable views |
| [src/ui/access_menu.rs](../src/ui/access_menu.rs) | 107 | `33e4ef2f9f0a9f2e` | preserves real permission choices while limiting Plan selection to slash commands |
| [src/ui/actions.rs](../src/ui/actions.rs) | 123 | `cfa92cff8761ea4d` | routes direct goal controls before the normal follow-up queue |
| [src/ui/activity_view.rs](../src/ui/activity_view.rs) | 119 | `7988d60e6cd0a1eb` | bounded activity rendering and session/menu state invalidation |
| [src/ui/activity_view/progress.rs](../src/ui/activity_view/progress.rs) | 146 | `af884069c3c552a5` | bounded activity rendering and session/menu state invalidation |
| [src/ui/activity_view/shimmer.rs](../src/ui/activity_view/shimmer.rs) | 99 | `3498089dd1bb422c` | focused visible-text sweep preserves cached layouts and native message interaction; animation schedules only during the current turn |
| [src/ui/activity_view/steps.rs](../src/ui/activity_view/steps.rs) | 140 | `3b9aa835bd8828c8` | bounded activity rendering and session/menu state invalidation |
| [src/ui/agent_catalog.rs](../src/ui/agent_catalog.rs) | 196 | `ddb7e11a651cf6be` | bounded activity rendering and session/menu state invalidation |
| [src/ui/agent_dock.rs](../src/ui/agent_dock.rs) | 108 | `01a57593db0c2fe3` | shared enabled playbook catalog and submission path |
| [src/ui/agent_dock/history.rs](../src/ui/agent_dock/history.rs) | 233 | `948ecb57c7a302de` | retained after review |
| [src/ui/agent_dock/inspector.rs](../src/ui/agent_dock/inspector.rs) | 230 | `5ca00565cd58b625` | retained after review |
| [src/ui/agent_dock/inspector/notes.rs](../src/ui/agent_dock/inspector/notes.rs) | 135 | `3fd3d2067b1f62f0` | retained after review |
| [src/ui/agent_dock/inspector/request.rs](../src/ui/agent_dock/inspector/request.rs) | 103 | `f2f4796201c73ef3` | bounded activity rendering and session/menu state invalidation |
| [src/ui/agent_dock/tabs.rs](../src/ui/agent_dock/tabs.rs) | 101 | `db1c3855f2b5527b` | conversation tabs use shared strip |
| [src/ui/agent_logos.rs](../src/ui/agent_logos.rs) | 62 | `846b686d4afb38d4` | retained after review |
| [src/ui/agent_option_rows.rs](../src/ui/agent_option_rows.rs) | 145 | `0092eb698d6fb1f1` | shared choice rows; dynamic text height, selection and warning icons without overflowing descriptions |
| [src/ui/agent_options.rs](../src/ui/agent_options.rs) | 181 | `77fb8f7437942a3c` | revisioned immutable menu snapshot and deferred session preparation |
| [src/ui/agent_options/menu_cache.rs](../src/ui/agent_options/menu_cache.rs) | 66 | `5bfd0887ee7473c9` | revisioned immutable menu snapshot and deferred session preparation |
| [src/ui/agent_options/model_selection.rs](../src/ui/agent_options/model_selection.rs) | 108 | `f5bf14882d1de036` | revisioned immutable menu snapshot and deferred session preparation |
| [src/ui/agent_options/model_selector.rs](../src/ui/agent_options/model_selector.rs) | 202 | `fcc76a156689298c` | revisioned immutable menu snapshot and deferred session preparation |
| [src/ui/agent_options/reasoning.rs](../src/ui/agent_options/reasoning.rs) | 167 | `b4cc07512a09a70a` | compact reasoning header; shared theme accent, capability-aware Fast mode, Medium reset and model breadcrumb |
| [src/ui/agent_options/reasoning/slider.rs](../src/ui/agent_options/reasoning/slider.rs) | 140 | `2a89fd6f83cd596e` | native stepped slider; cached seven-pixel prism spectrum, theme-aware focus and bounded drag preview with commit on release |
| [src/ui/agent_picker.rs](../src/ui/agent_picker.rs) | 91 | `9f024f380e3b0f9f` | bounded activity rendering and session/menu state invalidation |
| [src/ui/agent_picker/menu.rs](../src/ui/agent_picker/menu.rs) | 145 | `367760a4b2902755` | provider headings and bounded searchable model rows reuse existing popup components |
| [src/ui/agent_picker/providers.rs](../src/ui/agent_picker/providers.rs) | 65 | `9f716248fd29f4f9` | borrows installed visible provider catalogs; deduplicates configured commands |
| [src/ui/analysis_progress.rs](../src/ui/analysis_progress.rs) | 83 | `2dd0c8493217b372` | retained after review |
| [src/ui/annotations.rs](../src/ui/annotations.rs) | 237 | `f01e73914b55c24f` | retained after review |
| [src/ui/app.rs](../src/ui/app.rs) | 228 | `e87aedda7a90ea4e` | bounded activity rendering and session/menu state invalidation |
| [src/ui/appearance.rs](../src/ui/appearance.rs) | 180 | `90727c06561482fb` | clarifies code-view sizing independently of chat appearance preferences |
| [src/ui/attachment_strip.rs](../src/ui/attachment_strip.rs) | 64 | `6f0a6df5dbb167fe` | bounded removable reference chips without image decoding |
| [src/ui/bench_chrome.rs](../src/ui/bench_chrome.rs) | 185 | `23670f95a775f937` | retained after review |
| [src/ui/bench_sidebar.rs](../src/ui/bench_sidebar.rs) | 73 | `aae553c8758a9749` | retained after review |
| [src/ui/bench_sidebar/filter_menu.rs](../src/ui/bench_sidebar/filter_menu.rs) | 83 | `62090fcba93e56be` | retained after review |
| [src/ui/bench_sidebar/functions.rs](../src/ui/bench_sidebar/functions.rs) | 180 | `ed77802321a10b0d` | retained after review |
| [src/ui/bench_sidebar/parts.rs](../src/ui/bench_sidebar/parts.rs) | 159 | `355022c3ea7317b4` | retained after review |
| [src/ui/bench_sidebar/preferences.rs](../src/ui/bench_sidebar/preferences.rs) | 42 | `205d24f84a4b22a4` | retained after review |
| [src/ui/bench_sidebar/resource_rows.rs](../src/ui/bench_sidebar/resource_rows.rs) | 174 | `82b56228b699f902` | retained after review |
| [src/ui/bench_sidebar/resources.rs](../src/ui/bench_sidebar/resources.rs) | 131 | `3f832a2069f65722` | retained after review |
| [src/ui/bench_sidebar/rows.rs](../src/ui/bench_sidebar/rows.rs) | 204 | `c6af36628f44c806` | retained after review |
| [src/ui/bench_sidebar/state.rs](../src/ui/bench_sidebar/state.rs) | 48 | `3246a5f0c1f75976` | retained after review |
| [src/ui/bench_sidebar/views.rs](../src/ui/bench_sidebar/views.rs) | 79 | `af31da02b0b408fd` | retained after review |
| [src/ui/bench_sidebar/work.rs](../src/ui/bench_sidebar/work.rs) | 221 | `6f69001fe7539ba3` | retained after review |
| [src/ui/bench_status.rs](../src/ui/bench_status.rs) | 66 | `a19b71097ca22b1e` | shows generation counts through the existing analysis status area |
| [src/ui/branding.rs](../src/ui/branding.rs) | 4 | `24a5a13e9a7c3fc5` | one compact transparent Prism image shared by branded UI surfaces |
| [src/ui/capture.rs](../src/ui/capture.rs) | 99 | `3368fa74d823bdee` | bounded activity rendering and session/menu state invalidation |
| [src/ui/chat.rs](../src/ui/chat.rs) | 58 | `d0d24716c8158099` | bounded activity rendering and session/menu state invalidation |
| [src/ui/chat/choices.rs](../src/ui/chat/choices.rs) | 41 | `f11905e6894b9c45` | guards current completed replies and preserves draft, attachments and mentions around existing prompt delivery; reports independent persistence failures |
| [src/ui/chat/goal.rs](../src/ui/chat/goal.rs) | 160 | `3653ef1c51ceb214` | limits direct pause and clear interception to Codex; other agents receive their own commands |
| [src/ui/chat/permission.rs](../src/ui/chat/permission.rs) | 106 | `0fe7556e5f5cb245` | bounded activity rendering and session/menu state invalidation |
| [src/ui/chat/questions.rs](../src/ui/chat/questions.rs) | 185 | `6d3a29ea9ba8cd77` | owns one transient question per chat; preserves drafts and routes answers to the caller or existing prompt flow |
| [src/ui/chat/questions/card.rs](../src/ui/chat/questions/card.rs) | 101 | `4c367bdfb73eb93a` | shares the native question card, custom reply field and distinct answer actions above the composer |
| [src/ui/chat/questions/rows.rs](../src/ui/chat/questions/rows.rs) | 54 | `bc5a7c592b007250` | renders wrapped numbered choice rows with hover, focus and accessibility metadata |
| [src/ui/chat/queue.rs](../src/ui/chat/queue.rs) | 143 | `1d03b840ba8c0be5` | preserves mention context in existing persisted follow-up queues |
| [src/ui/chat/transcript.rs](../src/ui/chat/transcript.rs) | 109 | `4d7dfb85258f8fa0` | visible-row layout with stable identity, bounded geometry and scroll anchoring |
| [src/ui/chat/transcript/identities.rs](../src/ui/chat/transcript/identities.rs) | 22 | `21f4dbeace75c3e3` | visible-row layout with stable identity, bounded geometry and scroll anchoring |
| [src/ui/chat/transcript/rows.rs](../src/ui/chat/transcript/rows.rs) | 97 | `8bb0434901b6bc54` | visible-row layout with stable identity, bounded geometry and scroll anchoring |
| [src/ui/chat/transcript/viewport.rs](../src/ui/chat/transcript/viewport.rs) | 172 | `017fe98370f4becd` | visible-row layout with stable identity, bounded geometry and scroll anchoring |
| [src/ui/chat/updates.rs](../src/ui/chat/updates.rs) | 146 | `8cd0acfb41db219f` | bounded activity rendering and session/menu state invalidation |
| [src/ui/chat_actions.rs](../src/ui/chat_actions.rs) | 159 | `54f6469a75019273` | retained after review |
| [src/ui/chat_history.rs](../src/ui/chat_history.rs) | 112 | `82e8bf6b95e49930` | restores goal snapshots without claiming a live session |
| [src/ui/chat_list.rs](../src/ui/chat_list.rs) | 186 | `1759dda68fa6cbc8` | retained after review |
| [src/ui/chat_list/header.rs](../src/ui/chat_list/header.rs) | 118 | `38008484d6c8825e` | retained after review |
| [src/ui/chat_prompt.rs](../src/ui/chat_prompt.rs) | 174 | `9fa1c28759629057` | passes the stable UI chat identity into MCP helper configuration |
| [src/ui/chat_session.rs](../src/ui/chat_session.rs) | 191 | `456a6949de7f291c` | Shared Markdown response instructions; existing batch workflow, commands and context delivery preserved. |
| [src/ui/chat_tabs.rs](../src/ui/chat_tabs.rs) | 73 | `4dd3e25b1547c0c1` | retained after review |
| [src/ui/chat_tabs/operations.rs](../src/ui/chat_tabs/operations.rs) | 186 | `a04e8b7d08431a1b` | releases pending agent questions when configured tab switching cancels a turn |
| [src/ui/chat_tabs/project.rs](../src/ui/chat_tabs/project.rs) | 51 | `165517dbe34a63e7` | bounded activity rendering and session/menu state invalidation |
| [src/ui/chat_tabs/routing.rs](../src/ui/chat_tabs/routing.rs) | 47 | `a608571573ef9f86` | parked follow-ups start with their own chat event identity |
| [src/ui/code.rs](../src/ui/code.rs) | 121 | `a9936f100f8005db` | retained after review |
| [src/ui/code_fonts.rs](../src/ui/code_fonts.rs) | 66 | `c49ddd67701c5fd1` | rendering and bounded history |
| [src/ui/commands.rs](../src/ui/commands.rs) | 114 | `ecedf988f90d53db` | retained after review |
| [src/ui/composer.rs](../src/ui/composer.rs) | 103 | `32d1961e88fac177` | one compact shared composer; unified fill and geometry across home, setup and workbench |
| [src/ui/composer/completion.rs](../src/ui/composer/completion.rs) | 230 | `e41a31c3eb8d7a82` | inserts a compact skill reference at the caret without expanding instructions or sending |
| [src/ui/composer/input.rs](../src/ui/composer/input.rs) | 91 | `d4cc4dd3838c6e3b` | captures popup state before editing to prevent Return selection from sending |
| [src/ui/composer/layout.rs](../src/ui/composer/layout.rs) | 56 | `0cef56a599789978` | one-row minimum draft; bounded growth and common attachment and context spacing |
| [src/ui/composer/mentions.rs](../src/ui/composer/mentions.rs) | 80 | `f9684e3ffccb91e3` | shares bounded current-project references and selected mention context without a separate search popup |
| [src/ui/composer/palette_items.rs](../src/ui/composer/palette_items.rs) | 114 | `121d8adb71cfe233` | advertised slash actions without an immediate-send skill dispatcher |
| [src/ui/composer/picker.rs](../src/ui/composer/picker.rs) | 68 | `91363acdfa309c48` | shared popup navigation, row rendering and selection without prompt submission |
| [src/ui/composer/query.rs](../src/ui/composer/query.rs) | 44 | `e9310489279c9228` | maps character cursors to Unicode-safe token ranges; restricts slash queries to the start of drafts |
| [src/ui/composer/usage.rs](../src/ui/composer/usage.rs) | 43 | `baf5a21752b2ec7e` | context usage ring with optional reported cost |
| [src/ui/composer_attachments.rs](../src/ui/composer_attachments.rs) | 64 | `cdcb675f27c6dc2b` | shared native picker selects files and folders on macOS; retains bounded attachment validation and persistence |
| [src/ui/composer_drop.rs](../src/ui/composer_drop.rs) | 82 | `6bdd689ced8480fa` | retains targeted native file and folder drops; missing-path guidance uses the plus button |
| [src/ui/composer_toolbar.rs](../src/ui/composer_toolbar.rs) | 73 | `ef38ddba754d33f9` | clears pending agent questions through the existing stop action |
| [src/ui/debug.rs](../src/ui/debug.rs) | 92 | `7f8e63002eec5eab` | retained after review |
| [src/ui/file_icons.rs](../src/ui/file_icons.rs) | 131 | `ffef1a27f0ab6b30` | retained after review |
| [src/ui/footer_agents.rs](../src/ui/footer_agents.rs) | 85 | `c50d5fa49ffab910` | shared playbook submission path |
| [src/ui/footer_exports.rs](../src/ui/footer_exports.rs) | 165 | `8007f3791ecde31c` | retained after review |
| [src/ui/footer_preferences.rs](../src/ui/footer_preferences.rs) | 125 | `1d9ae0ffb93da625` | retained after review |
| [src/ui/footer_rows.rs](../src/ui/footer_rows.rs) | 119 | `1612bebb14ef96fc` | retained after review |
| [src/ui/function_bar.rs](../src/ui/function_bar.rs) | 144 | `72f9d80571f6805d` | binary tabs use shared strip |
| [src/ui/function_menu.rs](../src/ui/function_menu.rs) | 139 | `849112dc96051ecc` | retained after review |
| [src/ui/graph.rs](../src/ui/graph.rs) | 221 | `0e98c174ffa5a1aa` | rendering and bounded history |
| [src/ui/home.rs](../src/ui/home.rs) | 123 | `c5acc701ecf5a5cf` | uses the shared conversation view for pending questions even before the first message |
| [src/ui/home_chat.rs](../src/ui/home_chat.rs) | 85 | `1260bdb4b2310858` | retained after review |
| [src/ui/icons.rs](../src/ui/icons.rs) | 68 | `cb735758ae85c63e` | includes the approval hand icon in the existing icon registry |
| [src/ui/key_combo.rs](../src/ui/key_combo.rs) | 112 | `b3012b8fe87024e7` | retained after review |
| [src/ui/keyboard.rs](../src/ui/keyboard.rs) | 164 | `b62efc9a810b48e8` | retained after review |
| [src/ui/keyboard_controls.rs](../src/ui/keyboard_controls.rs) | 218 | `2aa08325fd9bab45` | retained after review |
| [src/ui/keyboard_edit.rs](../src/ui/keyboard_edit.rs) | 99 | `fc2c6083fe27a1c2` | retained after review |
| [src/ui/landing_content.rs](../src/ui/landing_content.rs) | 124 | `8880dee5178b82f2` | retained after review |
| [src/ui/message.rs](../src/ui/message.rs) | 111 | `c4eca0c4a55085d9` | bounded source cache and visible Markdown rendering |
| [src/ui/message/blocks.rs](../src/ui/message/blocks.rs) | 27 | `800657e2a953e323` | Existing bounded tool-code formatting retained; duplicate Markdown code renderer removed. |
| [src/ui/message/body_cache.rs](../src/ui/message/body_cache.rs) | 143 | `e4710b7b4aea439f` | bounded source cache and visible Markdown rendering |
| [src/ui/message/choices.rs](../src/ui/message/choices.rs) | 121 | `0ab8efeaa3dc7165` | retains bounded final-choice recognition; shares parsed options without a second inline button layout |
| [src/ui/message/layout.rs](../src/ui/message/layout.rs) | 49 | `a7942a8198d60afd` | bounded source cache and visible Markdown rendering |
| [src/ui/message/markdown.rs](../src/ui/message/markdown.rs) | 85 | `68fb29f9911ab364` | bounded source cache and visible Markdown rendering |
| [src/ui/message/style.rs](../src/ui/message/style.rs) | 110 | `a525585df6cdc1bb` | Saved conversation typography and activity accents; Markdown blocks delegated to the Rust library. |
| [src/ui/mod.rs](../src/ui/mod.rs) | 100 | `022056f1de5ecaa1` | thin UI declarations retain the dedicated access menu module |
| [src/ui/mode_bar.rs](../src/ui/mode_bar.rs) | 156 | `711f9a3ce6f0c541` | retained after review |
| [src/ui/native_file_icon.rs](../src/ui/native_file_icon.rs) | 65 | `cea350a0852126b9` | retained after review |
| [src/ui/notifications/card.rs](../src/ui/notifications/card.rs) | 178 | `3fc7a62c27da3a1e` | retained after review |
| [src/ui/notifications/mod.rs](../src/ui/notifications/mod.rs) | 6 | `fa6408a8fe0e415b` | retained after review |
| [src/ui/notifications/runtime.rs](../src/ui/notifications/runtime.rs) | 82 | `38280957d933e3b3` | retained after review |
| [src/ui/notifications/state.rs](../src/ui/notifications/state.rs) | 177 | `df542708a4fcae36` | retained after review |
| [src/ui/notifications/view.rs](../src/ui/notifications/view.rs) | 179 | `718610e79f9baf98` | retained after review |
| [src/ui/panel_layout.rs](../src/ui/panel_layout.rs) | 88 | `f96fe2fcc352d753` | shared animated panel boundary; preserves requested width, clipping and bounded repaint lifetime |
| [src/ui/permission_menu.rs](../src/ui/permission_menu.rs) | 88 | `e5fdb8ad70838da5` | running-code Auto Ask and Blocked menu retains real shell permission values |
| [src/ui/permission_settings.rs](../src/ui/permission_settings.rs) | 105 | `8f92f3341ab55fa2` | retained after review |
| [src/ui/playbooks.rs](../src/ui/playbooks.rs) | 99 | `a9924c247c95b278` | keeps catalog ownership and legacy message display; invocation lives in a focused module |
| [src/ui/playbooks/invocation.rs](../src/ui/playbooks/invocation.rs) | 55 | `e8379e281b7c72b6` | resolves compact skill references only for transport; respects disabled skills and adapter commands |
| [src/ui/playbooks/source.rs](../src/ui/playbooks/source.rs) | 141 | `921f0e96a60200af` | embeds the canonical project-owned skills directory |
| [src/ui/popup.rs](../src/ui/popup.rs) | 140 | `838cad3ba3e2fedf` | retained after review |
| [src/ui/project_switcher.rs](../src/ui/project_switcher.rs) | 118 | `873827788c053c53` | retained after review |
| [src/ui/receive.rs](../src/ui/receive.rs) | 234 | `a16e7e06b520cbe2` | bounded activity rendering and session/menu state invalidation |
| [src/ui/reconstruction.rs](../src/ui/reconstruction.rs) | 102 | `a17db26083da77e1` | reviewed shared-chat workflow, project routing, editable state, progress and error propagation |
| [src/ui/reconstruction/actions.rs](../src/ui/reconstruction/actions.rs) | 126 | `06b630ae2f4fcea0` | reviewed shared-chat workflow, project routing, editable state, progress and error propagation |
| [src/ui/reconstruction/progress.rs](../src/ui/reconstruction/progress.rs) | 141 | `a542fdcf14dc21dd` | reviewed shared-chat workflow, project routing, editable state, progress and error propagation |
| [src/ui/reconstruction/prompt.rs](../src/ui/reconstruction/prompt.rs) | 50 | `494acbedbc6b4da9` | reviewed shared-chat workflow, project routing, editable state, progress and error propagation |
| [src/ui/references.rs](../src/ui/references.rs) | 66 | `7a3e2de2dab52099` | retained after review |
| [src/ui/screen_navigation.rs](../src/ui/screen_navigation.rs) | 38 | `84893fa41f8f73c2` | retained after review |
| [src/ui/settings.rs](../src/ui/settings.rs) | 180 | `6284abd55028cfee` | shared content width and hidden scrollbars; storage page dispatch |
| [src/ui/settings_actions.rs](../src/ui/settings_actions.rs) | 169 | `ccab1007e8e4be73` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/ui/settings_controls.rs](../src/ui/settings_controls.rs) | 230 | `47605130593d58ee` | shared thin-ring radio indicator |
| [src/ui/settings_fields.rs](../src/ui/settings_fields.rs) | 138 | `3c2da6b8ba7b04e4` | retained after review |
| [src/ui/settings_files.rs](../src/ui/settings_files.rs) | 105 | `c09bead71851320e` | feature-gated backend configuration using existing settings controls |
| [src/ui/settings_navigation.rs](../src/ui/settings_navigation.rs) | 204 | `1c82969c1d458ad9` | refresh storage on entry; hidden navigation scrollbar |
| [src/ui/settings_page_row.rs](../src/ui/settings_page_row.rs) | 94 | `df75cd4efb82dee9` | retained after review |
| [src/ui/settings_pages/about.rs](../src/ui/settings_pages/about.rs) | 47 | `51146f06bfc503b1` | retained after review |
| [src/ui/settings_pages/agents.rs](../src/ui/settings_pages/agents.rs) | 120 | `dc4a23bc2f2f677b` | per-agent visibility controls reuse settings rows and preserve registry setup |
| [src/ui/settings_pages/chat.rs](../src/ui/settings_pages/chat.rs) | 163 | `ff692787fb9af87b` | reuses production message bubbles and collapsed activity in an aligned settings preview; retains existing chat controls |
| [src/ui/settings_pages/code.rs](../src/ui/settings_pages/code.rs) | 137 | `22d149366b17ed9b` | retained after review |
| [src/ui/settings_pages/engines.rs](../src/ui/settings_pages/engines.rs) | 41 | `4404a736202c4d52` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/ui/settings_pages/engines/choices.rs](../src/ui/settings_pages/engines/choices.rs) | 133 | `40a13f799ed1aafe` | feature-gated provider selection; experimental Rust resource limits disclosed in settings |
| [src/ui/settings_pages/engines/profile.rs](../src/ui/settings_pages/engines/profile.rs) | 142 | `850019a821477405` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/ui/settings_pages/general.rs](../src/ui/settings_pages/general.rs) | 57 | `dd301d9cf889aa02` | retains general and privacy settings after moving chat settings to their own module |
| [src/ui/settings_pages/mod.rs](../src/ui/settings_pages/mod.rs) | 58 | `e7384033de10c757` | routes the existing Chat settings page to its focused module |
| [src/ui/settings_pages/model.rs](../src/ui/settings_pages/model.rs) | 202 | `3b85b01bf6ccd147` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/ui/settings_pages/paths.rs](../src/ui/settings_pages/paths.rs) | 73 | `5963a8e2a0305f82` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/ui/settings_pages/permissions.rs](../src/ui/settings_pages/permissions.rs) | 84 | `67d6aa2feb34fd2d` | enables project approvals and explicit forgetting in existing settings controls |
| [src/ui/settings_pages/presets.rs](../src/ui/settings_pages/presets.rs) | 99 | `e08ddd9a64662123` | permission presets use shared radio indicator |
| [src/ui/settings_pages/render.rs](../src/ui/settings_pages/render.rs) | 201 | `b679dbabdb3c649c` | delegates chat preview rendering through the existing settings layout |
| [src/ui/settings_pages/row.rs](../src/ui/settings_pages/row.rs) | 186 | `6e31037d7f9871fd` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/ui/settings_pages/row_control.rs](../src/ui/settings_pages/row_control.rs) | 148 | `4306f773bf8ec350` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/ui/settings_pages/skills.rs](../src/ui/settings_pages/skills.rs) | 47 | `8a66eb42c0c80941` | built-in and custom skills share settings search and enable controls |
| [src/ui/settings_pages/storage.rs](../src/ui/settings_pages/storage.rs) | 41 | `9beaeaf3e649b434` | device storage cards and preserved export and layout controls |
| [src/ui/settings_preview.rs](../src/ui/settings_preview.rs) | 119 | `34a81c97baf20b38` | retained after review |
| [src/ui/settings_rows.rs](../src/ui/settings_rows.rs) | 87 | `20da7ad0dbb2cf16` | retained after review |
| [src/ui/settings_search.rs](../src/ui/settings_search.rs) | 58 | `fcd7b7cd640aa342` | remove unused recent-count context |
| [src/ui/settings_storage/chats.rs](../src/ui/settings_storage/chats.rs) | 157 | `6e361bf224c90877` | cached 50-row review pages with existing confirmation controls |
| [src/ui/settings_storage/measurement.rs](../src/ui/settings_storage/measurement.rs) | 110 | `9dc1ca5bcf267d39` | single bounded background scan with explicit cancellation |
| [src/ui/settings_storage/mod.rs](../src/ui/settings_storage/mod.rs) | 66 | `2e2d8cd5a8cb7ac5` | focused storage view state and action dispatch |
| [src/ui/settings_storage/overview.rs](../src/ui/settings_storage/overview.rs) | 185 | `44957e72a07c3784` | shared-width storage bar and responsive category legend |
| [src/ui/setup.rs](../src/ui/setup.rs) | 217 | `771dd1def864fd91` | onboarding reuses the combined selector and shared composer instead of a separate agent chip |
| [src/ui/shell.rs](../src/ui/shell.rs) | 72 | `a2c9636bd738a370` | retained after review |
| [src/ui/shortcut_actions.rs](../src/ui/shortcut_actions.rs) | 55 | `d3ff57d71712a842` | releases pending agent questions through the existing stop shortcut |
| [src/ui/shortcut_catalog.rs](../src/ui/shortcut_catalog.rs) | 132 | `1c4388d6bb6c6d6e` | retained after review |
| [src/ui/sidebar.rs](../src/ui/sidebar.rs) | 158 | `48dd575d68d82963` | retained after review |
| [src/ui/sidebar_footer.rs](../src/ui/sidebar_footer.rs) | 139 | `d734ec18bf61a546` | retained after review |
| [src/ui/sidebar_rows.rs](../src/ui/sidebar_rows.rs) | 178 | `316f9ce449bb4ebd` | reuse existing relative timestamps in storage review |
| [src/ui/split_panes.rs](../src/ui/split_panes.rs) | 126 | `05664b3c73d20da7` | retained after review |
| [src/ui/state.rs](../src/ui/state.rs) | 95 | `eb7353597b81a133` | bounded activity rendering and session/menu state invalidation |
| [src/ui/syntax.rs](../src/ui/syntax.rs) | 120 | `7bfbef6e62e2a892` | retained after review |
| [src/ui/tab_strip.rs](../src/ui/tab_strip.rs) | 135 | `8a839a4aa95a23f4` | shared binary and conversation tab component |
| [src/ui/theme.rs](../src/ui/theme.rs) | 168 | `159674b6e39bf889` | retained after review |
| [src/ui/tool_step.rs](../src/ui/tool_step.rs) | 177 | `71b21d6dfcfc6c55` | uses configurable chat sizes for activity titles and colored code-sized tool detail blocks |
| [src/ui/view_navigation.rs](../src/ui/view_navigation.rs) | 84 | `a2c76bf356f7932c` | retained after review |
| [src/ui/view_pane.rs](../src/ui/view_pane.rs) | 97 | `b00bad7e71648521` | retained after review |
| [src/ui/view_tab.rs](../src/ui/view_tab.rs) | 81 | `1492a46dae89a15f` | retained after review |
| [src/ui/whats_new.rs](../src/ui/whats_new.rs) | 190 | `6b2937efd1e170c1` | retained after review |
| [src/ui/widgets.rs](../src/ui/widgets.rs) | 217 | `afdaa7f64796ce75` | shared tinted chips retain natural-width measurement, text truncation and accessibility |
| [src/ui/window_commands.rs](../src/ui/window_commands.rs) | 148 | `e5de528758d20ab1` | bounded activity rendering and session/menu state invalidation |
| [src/ui/window_effects.rs](../src/ui/window_effects.rs) | 75 | `f191171959ec8634` | retained after review |
| [src/ui/window_state.rs](../src/ui/window_state.rs) | 55 | `ccc429e2957973c7` | reports bounded question presence and chat identity without exposing pending text |
| [src/ui/workbench.rs](../src/ui/workbench.rs) | 114 | `f6d8a01b9eeb0eff` | retained after review |
| [src/ui/workbench_footer.rs](../src/ui/workbench_footer.rs) | 184 | `3cf893e480da684f` | retained after review |
| [src/ui/workbench_tabs.rs](../src/ui/workbench_tabs.rs) | 154 | `fd3cb4df496cfa76` | retained after review |
| [src/worker/analysis.rs](../src/worker/analysis.rs) | 185 | `32939bd2a0279aa6` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/worker/analysis/view.rs](../src/worker/analysis/view.rs) | 110 | `a7f3ec98d80b68e8` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/worker/analysis_cache.rs](../src/worker/analysis_cache.rs) | 192 | `a9b98f5ff74a0d94` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/worker/analysis_cache_identity.rs](../src/worker/analysis_cache_identity.rs) | 18 | `4c7c35c37c2f6a57` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/decompilation_run.rs](../src/worker/decompilation_run.rs) | 84 | `8ae18f44f427f57e` | pause and release errors preserved; terminal sessions discarded without idle retention |
| [src/worker/decompilation_run/retained.rs](../src/worker/decompilation_run/retained.rs) | 151 | `46d9bb4343bcfb2e` | Rust64 and Java256 windows; committed item results precede the next request |
| [src/worker/decompile_batch.rs](../src/worker/decompile_batch.rs) | 72 | `d59902d0cc900200` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/worker/durable_decompilation.rs](../src/worker/durable_decompilation.rs) | 140 | `7981ad494d92e386` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/worker/execute.rs](../src/worker/execute.rs) | 105 | `96cc9a62b090dd68` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/exports.rs](../src/worker/exports.rs) | 122 | `e9e475d875a4c647` | retained after review |
| [src/worker/indexing.rs](../src/worker/indexing.rs) | 62 | `ddf2b68611b0aa83` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/inspection.rs](../src/worker/inspection.rs) | 32 | `ff8e1a2629fd907e` | retained after review |
| [src/worker/job.rs](../src/worker/job.rs) | 79 | `d74f072b2238c652` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/listing.rs](../src/worker/listing.rs) | 53 | `22b0c6a6173ab378` | retained after review |
| [src/worker/mod.rs](../src/worker/mod.rs) | 31 | `e2b7fdc077dda840` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/open.rs](../src/worker/open.rs) | 88 | `bb2f3789a50b8c88` | Rust REA integration reviewed for explicit feature gates, existing provider behavior and resource lifetime |
| [src/worker/port.rs](../src/worker/port.rs) | 23 | `9d50d775b71cfbc2` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/reconstruction.rs](../src/worker/reconstruction.rs) | 164 | `c5e2e265f122317d` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/remote.rs](../src/worker/remote.rs) | 83 | `fd6505ec18b99c2f` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/request_execution.rs](../src/worker/request_execution.rs) | 49 | `e38f12289694ba1a` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/request_lifetime.rs](../src/worker/request_lifetime.rs) | 101 | `7a0a298e7299daeb` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/runtime.rs](../src/worker/runtime.rs) | 91 | `d1f64bf96ca2d63a` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/scheduling.rs](../src/worker/scheduling.rs) | 115 | `84a80d9374a9d8a7` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/sidebar.rs](../src/worker/sidebar.rs) | 115 | `30223fa2ab7ce772` | retained after review |
| [src/worker/thread.rs](../src/worker/thread.rs) | 120 | `82566045c16bbcd4` | explicit prepared context, cooperative scheduling, verified cache reuse and bounded durable execution |
| [src/worker/views.rs](../src/worker/views.rs) | 85 | `86cbe789773ed266` | retains shared view/cache behavior after removing the obsolete reconstruction builder |
