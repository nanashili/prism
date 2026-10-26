# Native REA integration check, 25 October 2026

Prism was tested on this arm64 macOS system using
[`nanashili/rea-rust`](https://github.com/nanashili/rea-rust), version 6.7.0,
from commit `189f3474bfc5dfd6f3ed2689dfa765c7a4e9f770`. The fork had no
published release at the time of the check, so both native executables were
built from that checkout with the locked release profile.

| Executable | SHA-256 |
| --- | --- |
| `rea` | `22d71d01fca7ee33159ea5625706428d4da515a701d1191c866b2d39efe77188` |
| `rea-ghidra-rust-worker` | `175dc42ac921ee6f090231bb891a46a8a0fb40e976c5243195d5039cf07bd463` |

The release layout was installed under
`~/.local/share/rea-rust/versions/6.7.0+189f3474bfc5/aarch64-apple-darwin`,
with `~/.local/bin/rea` pointing to its native executable. Prism's saved
provider is `ghidra-rust-rea`; the worker override is empty so discovery uses
the worker beside the canonical launcher.

## Application samples

Each test initialized Prism's headless MCP interface, listed its tools and
functions, started a durable decompilation job, waited for completion, and
read the saved results. Every selected function succeeded with nonempty
pseudocode. The installed apps were read for static analysis, not executed.

| Application | Installed version | Functions sampled | Result |
| --- | --- | ---: | --- |
| The Unarchiver | 4.3.9 | 3 | Completed |
| Google Chrome native launcher | 155.0.8059.86 | 1 | Completed |
| CodexBar | 0.67.0 | 3 | Completed |

These samples verify native launch, function discovery, decompilation, and
durable result delivery. They do not assess complete application coverage or
the semantic accuracy of recovered code. Chrome's framework was not sampled.

The checks passed first with the development binary and then with the signed
`build/Prism.app` release bundle, using engine settings copied from the saved
configuration into an isolated support directory. Automatic discovery also
passed with no configured executable path. An executable text file selected as
REA was rejected before execution.

The final rebuilt application passed another Chrome launcher check. A second
signed bundle containing the native release passed the same check; its saved
runtime record confirmed that both executables were loaded from
`Prism-with-REA-check.app/Contents/Resources/rea-rust/bin`, with no external
runtime fallback. The normal bundle continues to use the separately installed
release selected during setup.

Raw results, pseudocode, app hashes, and runtime provenance are retained
locally in `build/rea-native-smoke/results.json` and
`build/rea-native-smoke/provenance.json`. Final rebuild and embedded-package
checks are in `final-build.json` and `embedded.json` in that same directory.
That build directory is ignored by Git.

## Reproduce

After installing the fork's native CLI and matching worker:

```sh
python3 scripts/bundle-macos.py
python3 scripts/smoke-rea.py \
  --prism "$PWD/build/Prism.app/Contents/MacOS/prism" \
  --support "$PWD/build/rea-native-smoke/state" \
  --output "$PWD/build/rea-native-smoke/results.json" \
  '/Applications/The Unarchiver.app/Contents/MacOS/The Unarchiver' \
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
  '/Applications/CodexBar.app/Contents/MacOS/CodexBar'
```

The harness fails if any selected function lacks successful saved pseudocode,
if a request fails, or if process cleanup fails.

## Validation evidence

After the project-wide Rust review, the rebuilt signed app again produced seven
successful nonempty saved pseudocode results across the same three applications.
The harness also requires a zero exit status after closing MCP input, so a
shutdown failure cannot pass acceptance. Current results and binary hashes are
in [`rea-smoke.json`](../build/rust-review/rea-smoke.json) and
[`rea-provenance.json`](../build/rust-review/rea-provenance.json).

The final Rust validation passed all 20 formatting, strict Clippy, test, and
rustdoc checks across the application, MCP-only, desktop-only, and Markdown
feature combinations. Application tests passed 29 cases with one ignored
network integration case; the full stable Markdown feature set passed 97 unit
tests and 16 documentation examples, with one ignored example. The
[Rust review](RUST_REVIEW.md) records the design findings and verification scope.

The profile
lock check confirmed that a second client exits with status 1 while the first
client remains responsive, then reopens successfully after the first client
releases the lock. Cleanup completed successfully, including the saved result
and runtime records described above. The raw cleanup summary is available at
[`cleanup-results.json`](../build/rea-native-smoke/cleanup-results.json), and the
profile-lock evidence is available at
[`profile-lock.json`](../build/rea-native-smoke/profile-lock.json).
