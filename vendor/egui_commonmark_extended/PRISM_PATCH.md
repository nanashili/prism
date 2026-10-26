# Prism layout patches

Source: the crates.io `egui_commonmark_extended` 0.28.3 package, whose Cargo VCS metadata identifies commit `ba6230b3b4ee35edf9fddbbc8c87329e15e23e15`, path `crates/egui_commonmark/egui_commonmark` in <https://github.com/aydiler/md-viewer>.

The package manifest, original manifest, source, README and license notices were copied unchanged before applying these changes. Upstream tests are preserved, with call sites updated for the typed APIs below.

## Changes

- `src/lib.rs`: adds an opt-in `responsive_tables(TableLayout)` viewer option, defaulting to `TableLayout::Fixed` and forwarded to all three render entrypoints.
- `src/parsers/pulldown.rs`: enables content-aware minimum column widths and the responsive policy for Markdown tables when requested. Discards the last resolved widths only when that policy reports a layout change, so row measurement and rendering use the same new widths.
- `src/parsers/pulldown.rs`: omits paragraph spacing after the document's final paragraph. The renderer uses a horizontal wrapping layout; adding space without a following line advances X and makes compact message frames overflow. Spacing between paragraphs remains unchanged.
- `src/parsers/pulldown/responsive_tables.rs`: contains Prism's small policy extension. Body words and inline-code fonts set readable minimum widths. A single signature per table tracks content, typography and the physical-pixel width budget. A changed signature resets the existing table state and uses the library's bounded column fit. Unchanged layouts retain manual column adjustments.
- `Cargo.lock`: records the standalone upstream-test validation dependency resolution. Prism's production build uses the repository root lock instead.

The table patch avoids rerunning the library's height optimization during every resize frame and avoids width-dependent widget IDs, unbounded per-width caches, private state access from Prism and global egui state resets. Tables whose minimum widths exceed the pane remain horizontally scrollable through the library's existing viewport. Lists, links, code, images and the default table policy keep upstream behavior. HTML tables are outside this option; Prism chat uses Markdown tables.

## Rust API and maintainability review

- `src/options.rs`: replaces Boolean builder modes with documented, non-exhaustive choices for table layout, image alt text, image URI schemes, strong fonts, frontmatter, and rendering scope. Defaults retain their previous behavior. These are source API changes; Prism's callers and the crate's examples use the named choices.
- `src/lib.rs`: documents the public viewer API and return values, marks the builder as `must_use`, and explicitly re-exports the supported macros.
- `src/parsers/pulldown.rs`: groups table measurement inputs in `TableMeasurements` and names the nested Markdown row/cell types. Internal list, parsing, and rendering options use the same named choices.
- `src/parsers/latex_delimiters.rs`: maps normalized offsets with unsigned source/normalized anchors, avoiding lossy signed conversions. Existing delimiter and source mapping tests exercise the behavior.
- Maintained sources use the shared strict Clippy policy, with narrow, explained expectations for context-heavy renderer signatures. Formatting and idiomatic iterator/borrow cleanups preserve rendering behavior.

The large parser files remain attributed third-party sources with the local changes described here. Their tests run both with Prism's parser feature and with every stable additive crate feature.

Licenses: MIT OR Apache-2.0, with upstream `LICENSE-MIT`, `LICENSE-APACHE` and `THIRD_PARTY_NOTICES` preserved and collected in app bundles. Review this patch when upgrading the pinned library.
