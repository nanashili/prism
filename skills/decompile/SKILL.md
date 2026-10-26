---
name: decompile
description: Read or export decompiled source or pseudocode without starting source reconstruction.
---

# Decompile code

Treat a plain `decompile`, inspect or explain request as decompilation, even when a reconstruction generation exists. Resolve the requested function or module from the saved analysis with `current_view` or `list_functions`. Missing functions are limitations to report; do not expand the inventory unless the user explicitly requests analysis or discovery.

For native functions, use Prism's `decompile_batch` workflow. A focused request uses `action: "start"` with a bounded `addresses` array; one selected function is a one-item durable batch. This path works with every native provider, including native Rust REA. A large module or whole inventory uses `action: "start"` once, retains the job id, omits `addresses` and `filter` to select the full indexed inventory, reads status and `results` pages with `include_data: true` through `next_offset: null`, and uses `action: "resume"` after restart. Native Rust REA requires a session prepared by explicitly opening the project; report an unavailable session instead of repeatedly resuming or initiating analysis. Do not recreate an active job, issue one-function calls for a durable export, or poll unchanged status in an immediate loop. Use `retry_failed` only after a diagnosed provider, configuration or input cause changes.

The batch workflow reads only functions already in the saved inventory. Never call `analyze`, `discover_functions`, `procedure_pseudo_code`, UI navigation or shell loops as an export path. If the batch command is unavailable or fails, report the blocker. Use `analysis_call` only for advertised metadata, assembly and call evidence needed to explain uncertainty; it does not authorize discovery.

For multi-function filesystem exports, establish a small module-to-output mapping as work proceeds and write bounded module files from the paged results. Keep related functions in bounded files, preserve observed names, aliases, addresses, input and analyzer identities, and record per-function failures; the durable database stores artifacts but does not create export files automatically. Do not delay export for a whole-program type or call-graph pass, and do not rewrite pseudocode into invented source abstractions. A decompilation export contains pseudocode artifacts, never reconstruction claims, source submissions or build results.

Use the provider's advertised bulk or paginated operations for managed assemblies and Android code. Managed work may use `list_managed_types`, `decompile_managed_type` or `export_managed_project` when advertised; page to completion and retain assembly hashes. Android method IDs are not native addresses: use its advertised method/source operations, while native libraries remain separate native targets.

Present the requested pseudocode or decompiled source in valid Markdown. Preserve branches, side effects and failure paths, mark uncertain operations near the relevant code, and cite the function address and evidence. Decompiled output is not recovered original source or a verified build. Rename or annotate only when the user explicitly requests those edits.

Only an explicit request to reconstruct, port or continue source reconstruction selects the `recompile` skill and its claim, evidence, submit and build workflow. Do not infer reconstruction from a saved generation or from the word `decompile`.
