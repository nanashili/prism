# REA GUI acceptance check, 25 October 2026

The rebuilt `build/Prism.app` opened successfully and used the installed
[`nanashili/rea-rust`](https://github.com/nanashili/rea-rust) runtime for static
analysis of three local applications. Seven sampled functions produced nonempty
pseudocode in the visible workbench. Assembly, hex, split panes, annotations,
and saving and reopening a project also passed the checks below.

This is a partial setup acceptance result. The complete setup wizard remains
unverified, and the native worker does not currently supply control-flow blocks
for the Graph view.

## Method and runtime

The signed app ran with a fresh support directory at
`build/ui-acceptance-20261025/profile`; the existing user profile was preserved.
Prism's built-in window commands drove the visible GUI and captured its rendered
window. This run did not use the headless interface.

The initial setup screen was observed. macOS then rejected simulated clicks with
`osascript is not allowed assistive access`. The user selected continuation with
Prism's built-in UI commands. Those commands can navigate settings and the
workbench, but cannot complete the setup wizard's individual steps or operate
the native executable picker.

The Engines settings page showed the native REA provider selected by default,
with automatic launcher and worker discovery. The runtime was already installed
before this GUI check; downloading it, choosing an executable through the picker,
and completing the wizard's agent and shortcut steps were not exercised.

The fresh profile automatically found `~/.local/bin/rea`. Prism's child process
and its worker both resolved to the matching installation under
`~/.local/share/rea-rust/versions/6.7.0+189f3474bfc5/aarch64-apple-darwin/bin`.

| Artifact | SHA-256 |
| --- | --- |
| `Prism.app/Contents/MacOS/prism` | `55f1b8821d87cc6275ab228df2f305534b7121a755e98328d62c0bc851d3b270` |
| `rea` | `22d71d01fca7ee33159ea5625706428d4da515a701d1191c866b2d39efe77188` |
| `rea-ghidra-rust-worker` | `175dc42ac921ee6f090231bb891a46a8a0fb40e976c5243195d5039cf07bd463` |

The runtime identifies as version 6.7.0 from commit
`189f3474bfc5dfd6f3ed2689dfa765c7a4e9f770`. The
[runtime record](../build/ui-acceptance-20261025/runtime.json) contains the
resolved paths, hashes, and Prism's owned process tree.

## Workbench checks

Each application was opened using its `.app` bundle path. Prism resolved its
native executable, populated the function list, and opened sampled functions in
the visible pseudocode pane. The target applications were read for static
analysis; their executables were not launched.

| Application | Functions in returned list page | Nonempty pseudocode samples | Assembly and hex |
| --- | ---: | ---: | --- |
| The Unarchiver | 300 | 3 | Passed |
| Google Chrome native launcher | 1 | 1 | Passed |
| CodexBar | 300 | 3 | Passed |

The function list command returns at most 300 entries per page. These counts do
not represent the complete application inventories. Chrome's framework was not
sampled. Assembly and hex were checked on one selected function per application.
These checks establish successful delivery and display of results, without
assessing the semantic accuracy of all recovered code.

The Unarchiver additionally passed these checks:

- Two functions opened in split panes, with pseudocode on one side and assembly
  on the other.
- A function was renamed to `ui_test_dispatch_once`, annotated, and bookmarked.
- The project was saved as `Unarchiver-ui-test.prism`.
- After opening Chrome and CodexBar, the saved project reopened successfully.
  The rename, note, and bookmark matched the saved values, and pseudocode loaded
  again using the renamed function.

All recorded workbench states settled with no pending requests and no reported
error. Prism was left open on the reopened Unarchiver project.

## Graph limitation

The Graph tab opened for each application, but each sampled function returned
zero control-flow blocks. The UI correctly displayed its provider empty state.
The native provider's function dossier explicitly marks CFG recovery as
unavailable because the worker does not recover a control-flow graph.

Graph tab navigation passed; actual control-flow graph generation did not pass
acceptance. The raw harness log's phrase “Graph rendered” describes opening and
capturing the tab. The classified JSON results record Graph as `unavailable`
and each application as `passed_with_limitations`.

## Evidence

Local evidence is retained in the ignored
`build/ui-acceptance-20261025` directory:

- [Classified results](../build/ui-acceptance-20261025/results.json)
- [Engine settings screenshot](../build/ui-acceptance-20261025/screenshots/03-engine-settings.png)
- [Split panes screenshot](../build/ui-acceptance-20261025/screenshots/unarchiver-split.png)
- [CodexBar pseudocode screenshot](../build/ui-acceptance-20261025/screenshots/codexbar-pseudocode.png)
- [Graph empty state screenshot](../build/ui-acceptance-20261025/screenshots/codexbar-graph.png)
- [Reopened project screenshot](../build/ui-acceptance-20261025/screenshots/unarchiver-reopened-project.png)
- [Saved project](../build/ui-acceptance-20261025/Unarchiver-ui-test.prism)

No Rust source changed during this GUI check. All 447 Rust source hashes still
match the inventory from the preceding 20 successful formatting, lint, test,
and documentation checks. Those automated checks and their scope are recorded
in the [Rust review](RUST_REVIEW.md) and
[coverage inventory](../build/rust-review/coverage.json). The separate
[native integration check](REA_NATIVE_SMOKE_20261025.md) records installation
and headless runtime validation.
