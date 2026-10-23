# Rust Ghidra through REA

Prism's experimental Rust Ghidra route uses REA's `ghidra-rust` provider and
the external worker built from that REA checkout. Enable it explicitly with
the optional `rea-rust-ghidra` Cargo feature and select `ghidra-rust-rea` in
Engine settings. Default desktop builds use Java Ghidra through REA.

REA's MCP service remains its Node/TypeScript runtime; the selected Ghidra
analysis and decompilation operations run in the Rust worker. This provider
does not replace REA's full runtime with a Rust rewrite.

Java Ghidra through REA is enabled by default with `prism-ghidra-rea`, selected
as provider `ghidra`. The saved `ghidra-rea`
and `ghidra-rust-rea` choices remain distinct. See
[Java provider setup](REFERENCE_BACKEND_PARITY.md#current-provider-selection-2026-10-20).
Prism's embedded Ghidra implementation and worker entrypoint have been removed.

The external provider is experimental. REA supervises sampled worker RSS using
Prism's configured memory threshold, but this is not a hard memory cap. Its
throughput and memory use do not justify promoting it to the default. The historical measurements below
concern an earlier worker implementation and do not establish performance for
this route.

## Build from source

Use a REA checkout containing `src/ghidra-rust` and
`native/ghidra-rust-worker`. Use the Node version pinned by that checkout's
`.nvmrc` (currently 24.18.0), then run these commands from its root:

```sh
npm ci
npm run build:cached
npm run build:ghidra:rust-worker
```

`build:cached` is the package's build entrypoint. It prepares the runtime and
its generated artifacts; selecting the source launcher alone is insufficient.
The worker build script enters its crate directory so rustup selects its
pinned `rust-toolchain.toml`. The native crate has its own Cargo workspace. With Cargo's default target
directory, its release worker is
`native/ghidra-rust-worker/target/release/rea-ghidra-rust-worker` (with `.exe` on
Windows). If the build uses `CARGO_TARGET_DIR` or a target triple, select the
actual resulting executable explicitly.

Enable the optional Rust integration alongside the default Java provider:

```sh
cargo build --release --locked --features rea-rust-ghidra --bin prism
```

To build only the Rust integration, disable defaults and select its features:

```sh
cargo build --release --locked --no-default-features \
  --features desktop,rea-rust-ghidra --bin prism
```

An application build using `--no-default-features` must explicitly enable
`desktop,rea-rust-ghidra` to include this route. Java can also be enabled with
`prism-ghidra-rea`; when both are compiled, Rust still requires selection in
Engine settings. Prism does not build an embedded Rust worker.

In Settings > Engines, select **Ghidra (Rust through REA)** and set the Rust REA
launcher to the checkout's `scripts/rea.mjs`. Leave the Rust worker setting
empty to use that package's default release output, or select its absolute
worker executable path. Empty path settings mean automatic resolution.

## Packaged setup and path resolution

The macOS bundle defaults to Java Ghidra through REA and builds
`desktop,prism-ghidra-rea` with Cargo's default features disabled:

```sh
python3 scripts/bundle-macos.py
```

Configure external REA, Node, Java and Ghidra installations in Settings > Engines.
`--java-ghidra` explicitly selects the same default.

Opt in to Rust packaging with `--rea-rust-ghidra`. This builds
`desktop,rea-rust-ghidra` with default features disabled. Supply every runtime
input explicitly; missing inputs stop the command. The script does not download
tools or copy binaries discovered on the developer's PATH:

```sh
python3 scripts/bundle-macos.py --rea-rust-ghidra \
  --rea-package /absolute/release/rea \
  --node-runtime /absolute/release/node \
  --rust-ghidra-worker /absolute/release/rea-ghidra-rust-worker \
  --worker-notices /absolute/release/rust-worker-notices
```

Prepare the REA release directory with its built `dist`, installed
`node_modules`, process-ownership Swift sources, package metadata, `.nvmrc`,
license notices and `scripts/rea.mjs`. Use a separate release staging
directory when installing production-only dependencies. The Node input must
be a portable distribution containing `bin/node` and its `LICENSE`; its
reported version must exactly match REA's `.nvmrc`. Supply the worker's own
and dependency license notices in `--worker-notices`.

Rust runtime inputs require `--rea-rust-ghidra` and cannot be combined with
`--java-ghidra`. A failed Rust build or missing Rust runtime never selects Java
automatically.

The script copies those explicit inputs to `Contents/Resources/rea`,
`Contents/Resources/node` and the license directory. The packaged worker is
`rea/bin/rea-ghidra-rust-worker`. Required runtime artifacts and direct Node
dependency versions must match the supplied package lock. Copied symlinks must
remain inside their copied directory. The executable inputs must be Mach-O
binaries with arm64 or x86_64 architectures. Node, the worker and `prism-mcp`
must cover the main app's architectures. Detected Mach-O files must resolve
non-system dynamic libraries inside the bundle.
`rea-runtime-manifest.json` records source hashes before signing and bundled
runtime hashes afterward. The script signs code and verifies the staged app
before replacing an existing Prism bundle.

Each successful release build records its selected provider, relative artifact
paths and both executable hashes in `.prism-bundle-build.json` in Cargo's target
directory, outside the app. The script reads Cargo's build artifact output so
a configured target triple cannot make it copy an older executable from a
different output directory. `--skip-build` requires that receipt and verifies
the exact provider, expected release paths and both hashes before staging.
Missing, malformed or mismatched receipts require a bundle command without
`--skip-build`. The staged copies must still match those verified hashes.

No distributable app has been produced or launched for this migration.
Ad hoc signing does not
provide notarization or a distributor identity. Worker protocol compatibility
is validated by REA during session preparation, rather than proved by copying
an executable into the bundle.

The current REA macOS process-ownership helper compiles its included Swift
sources through `/usr/bin/xcrun swiftc`. Apple Command Line Tools or Xcode and
a compatible macOS SDK therefore remain runtime prerequisites. The bundle
does not require Java for the Rust route. The default Java provider
still requires its configured Java and Ghidra installations. Source build
commands above apply to a REA checkout.

Launcher selection uses the first configured or discoverable choice:

1. `PRISM_RUST_REA`, when nonempty.
2. The saved Rust REA launcher in `tools.rust_rea`.
3. A Rust-enabled REA package at `Resources/rea/scripts/rea.mjs` or
   `resources/rea/scripts/rea.mjs` beside the application executable or in its
   enclosing bundle directories.
4. An installed `rea` launcher on Prism's executable search path, resolved to
   its package and accepted only when its built Rust runtime artifacts and
   installed Node dependencies are available.

Worker selection uses `PRISM_RUST_GHIDRA_WORKER`, then the saved
`tools.rust_ghidra_worker`, then `bin/rea-ghidra-rust-worker` in the selected
package, then that package's native release output described above. An explicit
worker must be absolute and point to an executable regular file. No
developer-directory scan or unrelated worker
search occurs. A missing runtime artifact or worker reports the selected path
and the build command needed to supply it. A configured but invalid choice
fails instead of selecting a different provider.

Launch and durable identity calculations share these resolutions. Identity
fingerprints the selected launcher, worker, protocol and required REA runtime
artifacts. A saved Node override takes precedence over
`Resources/node/bin/node`; development installations otherwise resolve Node
from the executable search path. The general `rea` setting remains
independent for other external providers.

Existing Rust REA and Java REA job identity fields remain distinct. Legacy
embedded-worker jobs remain available for inspection; they do not resume
against either REA engine. Rebuilding or moving runtime inputs can require a
new job.

## Saved inventory and durable batches

Open and explicitly prepare the project with the selected provider. REA exposes
a frozen inventory from compatible saved analysis or the input loader's initial
symbols, entry points and function metadata. Decompilation does not discover
additional functions. Select saved addresses with `list_functions`.

Use `decompile_batch` with `action: "start"` even for one function. Supply
`addresses` from that inventory, or a `filter` over saved function names; omit
both to request the complete saved inventory. Keep the returned `job` value.
Use `action: "status"` to follow progress and `action: "results"` with
`include_data: true` to read saved pseudocode. `offset` and `limit` page results.
The job controls also provide `list`, `pause`, `cancel`, `resume` and
`retry_failed`. A request cannot expand the frozen inventory through analysis
or function discovery.

Prism owns job manifests, successful per-function artifacts, cursors and resume
validation. It submits Rust work in windows of at most 64 saved addresses and
commits each window through existing artifact/cursor transactions. Each saved
pseudocode item is limited to 512 KiB. REA owns its prepared native session,
request deadlines and bounded response views; the external worker owns its
loaded program and analysis cache.

Completed job artifacts survive restart. Reopen and prepare the matching
project, then explicitly resume the saved job; jobs do not restart
automatically. Resume validation includes the target, frozen inventory,
provider and runtime identity. A backend rebuild or configuration change can
require a new job instead of reusing incompatible output. Rust cache data uses
Prism's separate `ghidra-rust-rea` support directory.

## Resource supervision

Prism passes `analysis_memory_mib` as `REA_GHIDRA_RUST_MEMORY_MIB` to opt in to
REA's sampled worker RSS supervision. The threshold covers only the external
worker process. REA's Node process, Prism and other processes add memory outside
that threshold.

REA observes RSS when the worker starts, then schedules each next observation
one second after the previous completes. Each observation has a one-second
deadline. Reaching the threshold reports a resource failure; unavailable RSS
observation reports a supervision failure. REA retires the owned worker after
either failure.

This is sampled supervision, not an operating-system allocation limit or hard
cap. Memory can overshoot between observations and while worker shutdown
completes. Backend identity records the threshold, worker-only scope, sampling
policy and absence of a hard limit. Changing the threshold changes the analysis
configuration and can require a new durable job.

## Compatibility limits

The current REA adapter supports macOS and Linux. Windows execution, Java
analyzers, Java database formats, Objective-C and Swift analysis, and complete
function discovery are unavailable through this provider. Function-name search
is limited to the frozen inventory, and object-section permission information
may be unknown. Universal Mach-O inputs require a selected thin image.

The adapter does not support an enforced worker heap ceiling. Successful
pseudocode does not establish semantic equivalence, complete type recovery or
buildable source. No 200,000-function run has been completed or measured for
the current route; throughput, cumulative memory use and restart behavior at
that scale remain unverified. The 64-address batch and 128-attempt Program
epoch are bounds on admitted work, not guarantees of an acceptable working set.

## Historical Prism-hosted flow and memory observations

These retained observations concern the earlier Prism-hosted worker and its
REA branch. They do not validate the external worker in the current REA clone.
The sampled RSS budget described here belonged to that earlier implementation.
The current external adapter supervises worker-only RSS as described above;
neither implementation provides a hard native memory ceiling.

A frozen inventory identifies admissible function starts; it does not establish
complete function bodies or reproduce the Java host's analysis facts. The
native flow engine can follow a call's fallthrough when the callee lacks an
explicit no-return prototype. Registering the next function start does not, by
itself, stop that traversal.

The public Ghidra host contract at commit
`d6192cb3f900f74152a4eeec1aa6758b6143b093` carries these facts separately from
function names. Its
[FunctionPrototype.grabFromFunction](https://github.com/NationalSecurityAgency/ghidra/blob/d6192cb3f900f74152a4eeec1aa6758b6143b093/Ghidra/Framework/SoftwareModeling/src/main/java/ghidra/program/model/pcode/FunctionPrototype.java#L150-L152)
uses the function's explicit no-return property or a call-fixup with no
fallthrough, then encodes the
[no-return attribute](https://github.com/NationalSecurityAgency/ghidra/blob/d6192cb3f900f74152a4eeec1aa6758b6143b093/Ghidra/Framework/SoftwareModeling/src/main/java/ghidra/program/model/pcode/FunctionPrototype.java#L323-L325).
[DecompileCallback](https://github.com/NationalSecurityAgency/ghidra/blob/d6192cb3f900f74152a4eeec1aa6758b6143b093/Ghidra/Features/Decompiler/src/main/java/ghidra/app/decompiler/DecompileCallback.java#L844-L873)
supplies mapped function information through that prototype path. The Rust
engine already consumes callee prototype flow effects in
`vendor/ghidra-decompiler/src/flow.rs` in the historical Prism source snapshot,
but that adapter did not import the Java host's no-return facts. Prism no longer
links this vendored engine; the current worker uses its own pinned dependency.

A focused diagnostic used the existing independently authored, stripped ARM64
Prism executable with SHA-256
`cd8fefb292a7e91a395727ecdf38f3e31b9f3193bc52a13cfb8a6b2d604c2309`.
No proprietary input or pseudocode was inspected. Five saved selectors completed
in 13.788 seconds; one required 13.467 seconds. Its post-cleanup counters recorded
219,792 operation slots, 666,836 varnode slots, 24,451 block slots and 105,058 high
variable slots assigned during analysis. All corresponding live counts were
zero except two retained root blocks. These slot counts describe allocation
history, including removed objects and regenerated graphs; they are not
verified function-body sizes. `push_multi` was tested 5,427,526 times and applied
27 changes; `multicollapse` was tested 5,427,499 times and applied 5,359 changes.
The observed work therefore includes extensive rule scanning, rather than
millions of successful rewrites.

The native process's physical footprint peaked at about 415.2 MiB in that
diagnostic and fell to 42.5 MiB after the batch. This supports release of
transient function state; it does not establish an acceptable worst-case
working set. A later unprofiled attempt on the original 320-selector workload
saved 192 successes before the watchdog paused it at 81.804 seconds, observing
1,283,325,952 bytes against the 1,073,741,824-byte native-family budget. The
remaining window produced no durable results. This workload has not completed
within the configured budget.

Bounded assembly inspection of that same owned input found a call immediately
before another saved entry. The callee had only a default name in the native
inventory, while the existing Java inventory supplied a recovered runtime name.
Missing no-return metadata plausibly permits excessive fallthrough here, but
neither the recovered name nor adjacency proves a no-return fact for the
stripped address. No name guessing, next-entry body cutoff or additional
function discovery was added.

A future metadata bridge needs an explicit, trustworthy source and durable
ownership outside a transient `Funcdata`. Releasing a selected function drops
its `Funcdata`; recreating it would lose a one-time prototype mutation. Verified
facts would need to survive that release and be reapplied before both the raw
referenced-string flow pass and full decompilation. Until such a contract is
implemented and validated, successful output and bounded transport do not
establish correct flow extent or parity with Java analysis.

## Historical prototype configuration and validation

This section records the previous prototype build and validation. All commands,
hashes, source-size inventories, test counts and measurements below refer to
that earlier snapshot and do not validate the current external worker or
justify default promotion. The previous settings were preserved in
`~/Library/Application Support/Prism/settings.before-rust-rea-20261020.json`,
and the feature-enabled macOS app was built at `build/Prism.app`.

The final stripped release includes both optional providers. Its signed bundle
is 23,273,434 bytes (22.20 MiB), with bundle identity `io.github.nanashili.prism` and the
normal `prism` executable name. The bundled main executable is 19,765,216 bytes,
SHA-256 `589dd317f1c8c5cd065bf6f441faa20427899df52f70e062e0b3ec744482a25b`.
`codesign --verify --deep --strict build/Prism.app` passed. The running GUI was
not relaunched. REA and Node remain external rather than duplicated in the bundle.

Validation includes REA compilation and type checking, the module-boundary and
lint checks, 135 existing source tests and 48 existing MCP boundary tests. Rust
checks cover the default, MCP-only and optional Java/Rust feature combinations,
strict all-feature Clippy, existing all-target tests and the release build.
The final feature checks and Clippy passed after the settings description was
updated to disclose the experimental provider's observed resource limitation.
The existing Rust test run contains one bridge-secret test; it does not exercise
this new provider. Provider runtime coverage comes from the measured MCP runs
and source review, with the remaining gaps recorded in the comparison.
The first-party source-size audit covers 434 Rust files, including build.rs.
All 434 review entries match their current file counts and source hashes. No
dependencies were added. REA still reports existing repository lint warnings and two runtime
bundler warnings about ineffective dynamic imports.

The first real Rust run exposed segment and procedure-list response shapes
that did not satisfy REA's public MCP schemas. The adapter now projects those
shapes explicitly, including honest Rust object-section provenance and unknown
memory protection flags. Prism preserves plain-text SDK failure messages and
retires an uncertain prepared session instead of replacing the failure with a
JSON decoding message.

The first initialized batch exposed native working-memory growth above the
1 GiB worker budget before its first result chunk completed. After reducing the
Rust delivery window to 64 and correcting failed-session cleanup, the final
320-function attempt preserved 192 verified outputs before the same budget
stopped the worker. All owned processes exited. These runs establish improved
durability and cleanup, not acceptable Rust throughput or memory parity.

See [the measured comparison](REA_RUST_BENCHMARK_20261020.md) for workload,
coverage, memory, timing, restart/resume checks and their limitations.
