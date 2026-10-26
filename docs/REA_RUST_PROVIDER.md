# Native REA provider

Prism uses the `nanashili/rea-rust` native release for the Rust Ghidra route.
The release contains the native CLI, stdio MCP server, Ghidra worker, and skills
in a single platform archive.

The setup flow opens the fork's installation instructions. The user downloads
and installs a release, or builds the source, then browses to its native executable in Prism's
engine settings. Prism launches that executable with `mcp` and sets
`REA_GHIDRA_RUST_WORKER` to the worker beside it. A source checkout can be used
for development, but the launcher and worker must still be executable native
files from the same build.

## Install REA

The fork currently has no published release. Use the source build below until
a native archive is published. The release installer commands apply once a
release is available.

On macOS or Linux, run the upstream installer from a terminal:

```sh
curl -fsSL https://github.com/nanashili/rea-rust/releases/latest/download/install.sh \
  -o install-rea.sh
sh install-rea.sh
export PATH="$HOME/.local/bin:$PATH"
rea --version
```

The installer verifies the release checksum and installs under the user's
account. It does not edit shell profiles or MCP configuration. The stable
launcher is `~/.local/bin/rea`; its versioned directory keeps `bin/rea` beside
`bin/rea-ghidra-rust-worker`. Run `rea setup` only when registering REA with an
independent MCP client; Prism launches it directly.

In Prism, open **Settings > Engines**, choose **Ghidra (Rust through REA)**,
and select the installed `rea` executable if automatic discovery does not find
it. Leave the worker field empty to use the worker next to that executable, or
set an absolute worker path for a development checkout. `PRISM_RUST_REA` and
`PRISM_RUST_GHIDRA_WORKER` provide equivalent process-level overrides.

The source build layout is:

```sh
git clone https://github.com/nanashili/rea-rust.git
cd rea-rust
cargo build --locked --release -p rea
cargo build --manifest-path ghidra-rust-worker/Cargo.toml --locked --release
cp ghidra-rust-worker/target/release/rea-ghidra-rust-worker target/release/
```

## Build Prism

The native provider is enabled by default with the `rea-rust-ghidra` feature. A desktop
build is:

```sh
cargo build --release --locked --no-default-features \
  --features desktop,rea-rust-ghidra --bin prism
```

The build has one Ghidra provider: native `rea-rust` through
`rea-rust-ghidra`.

## macOS application bundles

The macOS bundler builds the native provider by default and leaves REA external
to keep the application small:

```sh
python3 scripts/bundle-macos.py
```

The resulting app discovers the installed `rea` through Prism's executable
search path. To embed a verified release explicitly, pass the extracted
`rea-rust` directory:

```sh
python3 scripts/bundle-macos.py \
  --rea-package /absolute/path/to/rea-rust
```

The directory must contain `LICENSE`, `bin/rea`, and
`bin/rea-ghidra-rust-worker`. Published
archives also include `VERSION`, `release.json`, README, skills, and any
release license files; those files are copied unchanged. An optional
`bin/rea-mcp-server` is accepted and signed when present. The package contains
the native runtime and worker expected by Prism.

The bundle stores the release at `Contents/Resources/rea-rust` and resolves
`bin/rea` there before checking external installations. The worker remains
beside the launcher. Every Mach-O executable must contain arm64 or x86_64 and
all non-system dynamic libraries must resolve inside the app. A runtime
manifest records source and post-signing hashes. The build receipt protects
`--skip-build`; failed staging restores the previous app before publication.

## Analysis behavior and limits

Select and prepare a project before requesting analysis. The native provider
exposes a saved function inventory; `decompile_batch` accepts only addresses
from that inventory and Prism submits at most 64 addresses per durable window.
Jobs keep successful per-function artifacts and cursors across restarts.

Prism passes `analysis_memory_mib` as `REA_GHIDRA_RUST_MEMORY_MIB`. REA samples
the worker's RSS and retires it after a threshold or sampling failure. This is
sampled supervision, not an operating-system memory cap; Prism and the REA
process are outside that worker-only threshold.

The provider supports macOS, Linux, and Windows native releases. It does not provide
complete function discovery or guaranteed type recovery. Universal Mach-O inputs
use a selected thin image. Changing the runtime or worker can require a new
durable job.

Provider measurements are workload-specific and do not promise complete Ghidra
parity.
