# Current implementation notes

This page documents the current native Prism architecture, provider setup, resource
bounds, and verification scope. Product setup and compatibility details are in the
[project README](../README.md); native REA installation and packaging details are in
[REA_RUST_PROVIDER.md](REA_RUST_PROVIDER.md).

## Architecture

Prism is a Rust desktop application with an eframe/egui interface and a standalone
Rust MCP helper. `src/ui/` owns presentation and interaction. `src/worker/` owns
analysis requests, indexing, views, exports, scheduling, and durable decompilation.
`src/analysis/` adapts native providers and binary formats. `src/mcp/` owns the
stdio protocol, schemas, validation, and endpoint discovery. `src/bridge/` owns the
authenticated local bridge, while `src/protocol/` bounds subprocess transport.

`src/storage/` owns SQLite, caches, atomic files, profile locks, inventories, jobs,
and saved results. `src/reconstruction/` owns generations, immutable objects,
module contracts, dependency scheduling, build commands, receipts, and reports.
The desktop process starts provider subprocesses only when an opened project needs
them; the MCP helper uses the authenticated loopback bridge for window operations.

## Native analysis

REA Rust is the default Ghidra provider through the `rea-rust-ghidra` feature. The
setup flow directs the user to install the native `nanashili/rea-rust` release, then
accepts the `rea` executable in **Settings > Engines**. Prism launches it with `mcp`
and discovers `rea-ghidra-rust-worker` beside it, or uses an explicit worker path.
The provider guide documents release verification, source builds, environment
overrides, platform support, and explicit bundle embedding.

`src/worker/analysis.rs`, `src/worker/indexing.rs`, and
`src/worker/analysis_cache.rs` coordinate provider sessions and saved inventories.
`src/worker/decompile_batch.rs`, `src/worker/durable_decompilation.rs`, and
`src/storage/decompilation/` implement bounded durable function batches. A batch
accepts only addresses in the saved inventory, persists successful artifacts and
cursors, and validates output before publication. A cache miss remains a bounded
request; it cannot expand the inventory implicitly.

Rizin and jadx remain separately selected integrations for their supported formats.
ACP agents are configured independently; ACP presets that use `npx` require the
user's Node/npm installation. Those agent requirements do not affect native REA
analysis.

## Persistence and reconstruction

The SQLite layer uses profile locks, WAL, atomic publication, and checksum-validated
artifacts. Durable decompilation jobs preserve progress and successful results across
normal restarts, while cancellation and failures retain explicit state. The current
owners are `src/storage/decompilation/jobs.rs`,
`src/storage/decompilation/results.rs`, `src/worker/durable_decompilation.rs`, and
`src/worker/decompilation_run.rs`.

Reconstruction generations are implemented under `src/reconstruction/`. Contracts,
SCCs, module dependencies, evidence, immutable objects, build snapshots, and
receipts are versioned by generation. Build and validation outcomes remain separate
from source generation; `behavior_verified` is not inferred from compilation alone.
See [RECONSTRUCTION.md](RECONSTRUCTION.md) for the current MCP workflow.

## Resource limits

Analysis memory is configured per provider session. REA supervises worker RSS with a
sampled threshold and request timeout; this is a process policy rather than an
operating-system memory cap. Saved views, project extraction, protocol frames,
queues, MCP payloads, and reconstruction objects each have bounded budgets, with
SQLite and rendering overhead accounted for separately.

Function discovery and type recovery depend on the selected native release and
input format. Universal Mach-O inputs use a selected thin image. Indirect references,
unsupported processors, and incomplete inventories remain visible as limits. A
runtime or worker revision change can require a new durable job because provider
identity is part of saved analysis configuration.

## Verification

The current native integration check is recorded in
[REA_NATIVE_SMOKE_20261025.md](REA_NATIVE_SMOKE_20261025.md). It covers Prism's
headless MCP interface, tool discovery, durable decompilation, saved pseudocode,
three installed macOS applications, signed bundle execution, profile locking, and
cleanup. The report links raw result and provenance artifacts under
`build/rea-native-smoke/`.

Formatting, compilation, linting, and release checks are run by the repository's
current validation workflow. This page intentionally records architecture and
current limits; generated build output and machine-specific measurements belong in
the smoke report or build receipts.
