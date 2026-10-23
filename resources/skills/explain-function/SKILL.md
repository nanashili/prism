---
name: explain-function
description: Explain a selected function's behavior, inputs, outputs, side effects, and role in the call graph from code evidence.
---

# Explain a function

1. Resolve the user's named function or the selection from `current_view` or `get_state`. In Rust, resolve a name to an address through `list_functions` after `analyze` finishes. Open a function the user named if it differs from the current selection. Keep its address or exact managed type and method identity. If there is no selection, ask which function to examine.
2. Read complete available source or pseudocode through MCP batches. In Prism, use a one-item `decompile_batch` for a selected native or indexed Android function and follow `next_offset` until null. A saved reconstruction generation does not change this analysis workflow. Only explicitly requested source reconstruction uses the recompile skill's claim, evidence and submit workflow. For managed code, use advertised bulk or paginated type/method operations. Compare suspicious native branches, calls and data access with advertised assembly or control-flow evidence through `analysis_call`. Do not use direct `procedure_pseudo_code`, UI navigation or truncated `get_state.visible_text` as a decompilation path. Report unavailable or failed batch commands rather than bypassing them.
3. Inspect relevant callers, callees, referenced strings, and imports to establish parameter meaning and the function's role. Do not assign a type, return meaning, or intent based on a name alone. Follow only the edges needed to answer the question.

Explain what it does, how control and data move through it, the inputs and return value, observable side effects and failure paths. Cite addresses or source locations for the decisive evidence. Separate directly observed behavior from an inference and state what remains unknown. Show a short code excerpt only when it clarifies a non-obvious point.

If the user asked for project annotations, add a concise note or rename only after checking the existing name and evidence. Do not navigate to another function or change project labels merely to answer an explanation request.
