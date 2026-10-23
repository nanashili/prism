# Third-party notices

Prism's original contributions and modifications are available under the root
[MIT license](LICENSE). Inherited material and dependencies keep their own
licenses. [NOTICE](NOTICE) records the project's origins and modifications.

Copied integration artwork and interface icons live in `resources/upstream/`.
Their original [Apache-2.0 license](resources/upstream/LICENSE-APACHE) is retained.
JetBrains Mono is distributed with its
[SIL Open Font License](resources/upstream/fonts/JetBrainsMono-OFL.txt).
These assets are included in this repository so builds do not require a parent
checkout. The macOS bundler includes the root license, attribution notice,
upstream license, font license, and active dependency notices.

Selected direct Rust dependencies (exact versions are in Cargo.toml and Cargo.lock):

| Library | Purpose | License |
| --- | --- | --- |
| eframe, egui, egui_extras | Native window, OpenGL UI, PNG and SVG loading | MIT OR Apache-2.0 |
| egui_commonmark_extended / egui_commonmark_backend_extended | Native Markdown chat rendering and typography | MIT OR Apache-2.0 |
| object | Mach-O function-start and entry-point metadata | MIT OR Apache-2.0 |
| image | PNG icons and window exports | MIT OR Apache-2.0 |
| serde, serde_json | Settings, documents and protocols | MIT OR Apache-2.0 |
| rusqlite | SQLite project history and paged function indexes | MIT |
| sha2 | CPU-accelerated streaming file integrity | MIT OR Apache-2.0 |
| zstd | Compressed views and documents | MIT |
| tempfile | Atomic file replacement | MIT OR Apache-2.0 |
| rfd | Native file dialogs | MIT |
| shlex | Quoted argument parsing | MIT OR Apache-2.0 |
| base64 | Bounded ACP image attachment encoding | MIT OR Apache-2.0 |
| url | Attachment paths and resource URLs | MIT OR Apache-2.0 |
| dirs | Platform data directory | MIT OR Apache-2.0 |
| getrandom | Local bridge authentication token | MIT OR Apache-2.0 |
| libc | Unix child process group cleanup | MIT OR Apache-2.0 |
| window-vibrancy | Native macOS frosted window material | MIT OR Apache-2.0 |
| liquid-glass-rs | Native macOS glass views | MIT |

The sha2 assembly feature also activates sha2-asm 0.6.4 (MIT). Its package license is collected with the other active dependency notices.

`egui_commonmark_extended` 0.28.3 is vendored with an opt-in responsive Markdown table policy. Upstream notices are preserved; provenance and the scoped changes are recorded in `vendor/egui_commonmark_extended/PRISM_PATCH.md`. The bundle collector includes local path dependencies as well as registry and Git packages.

SQLite is bundled through libsqlite3-sys; SQLite is public domain. Zstandard's bundled C code is BSD-3-Clause OR GPL-2.0; the BSD option is used. The bundle script copies package license notices from the active platform dependency tree into Resources/licenses/cargo and writes an index. Cargo.lock and the individual package notices identify exact versions and applicable terms. Build-only tools and external programs are not distributed in the bundle.

Prism delegates Rust Ghidra analysis to REA's separately built worker. The worker's dependency notices are supplied with its release inputs and copied into the application bundle. The historical `vendor/ghidra-decompiler` source snapshot is not a Prism Cargo dependency and is not included in application bundles.

Rizin, the Java Ghidra application, jadx, LLDB and ACP agents are separate
installed programs. Default builds also use external REA and Node installations.
The optional Rust-provider bundle can include explicitly supplied REA, Node and
worker runtimes with their accompanying notices. Each program retains its own
license and installation requirements.
