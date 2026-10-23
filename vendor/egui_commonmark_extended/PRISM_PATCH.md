# Prism layout patches

Source: the crates.io `egui_commonmark_extended` 0.28.3 package, whose Cargo VCS metadata identifies commit `ba6230b3b4ee35edf9fddbbc8c87329e15e23e15`, path `crates/egui_commonmark/egui_commonmark` in <https://github.com/aydiler/md-viewer>.

The package manifest, original manifest, source, README and license notices were copied unchanged before applying these changes. Upstream tests are preserved. No new test or fixture is authored here.

## Changes

- `src/lib.rs`: adds an opt-in `responsive_tables(bool)` viewer option, defaulting to false and forwarded to all three render entrypoints.
- `src/parsers/pulldown.rs`: enables content-aware minimum column widths and the responsive policy for Markdown tables when requested. Discards the last resolved widths only when that policy reports a layout change, so row measurement and rendering use the same new widths.
- `src/parsers/pulldown.rs`: omits paragraph spacing after the document's final paragraph. The renderer uses a horizontal wrapping layout; adding space without a following line advances X and makes compact message frames overflow. Spacing between paragraphs remains unchanged.
- `src/parsers/pulldown/responsive_tables.rs`: contains Prism's small policy extension. Body words and inline-code fonts set readable minimum widths. A single signature per table tracks content, typography and the physical-pixel width budget. A changed signature resets the existing table state and uses the library's bounded column fit. Unchanged layouts retain manual column adjustments.
- `Cargo.lock`: records the standalone upstream-test validation dependency resolution. Prism's production build uses the repository root lock instead.

The table patch avoids rerunning the library's height optimization during every resize frame and avoids width-dependent widget IDs, unbounded per-width caches, private state access from Prism and global egui state resets. Tables whose minimum widths exceed the pane remain horizontally scrollable through the library's existing viewport. Lists, links, code, images and the default table policy keep upstream behavior. HTML tables are outside this option; Prism chat uses Markdown tables.

Newly authored library code is limited to the responsive policy, its integration and the terminal paragraph guard above. The existing large upstream files remain third-party sources, not migrated first-party modules.

Licenses: MIT OR Apache-2.0, with upstream `LICENSE-MIT`, `LICENSE-APACHE` and `THIRD_PARTY_NOTICES` preserved and collected in app bundles. Review this patch when upgrading the pinned library.
