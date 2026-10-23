---
name: name-functions
description: Rename selected or high-value unnamed functions after checking their behavior and existing project labels.
---

# Name functions

Name only functions whose role is supported by code. A descriptive but uncertain name can be worse than an address.

1. Establish the requested scope and inspect existing names and notes. Use `list_marks` for annotations and `get_state` and `list_functions` for the current selection and indexed names. Do not overwrite a user or prior agent name without a reason.
2. For a broader naming request, page `list_procedures` or `list_functions` and prioritize unnamed entry points, exports, and functions reached from distinctive strings or imports. Stop when the remaining candidates are generic runtime helpers or lack evidence. In Prism, call `analyze` and wait for `get_state.pending` to clear before `list_functions`; Rizin's `list_procedures` is only a partial sample. Discover analysis methods with `analysis_tools` and invoke them through `analysis_call`.
3. Group native or indexed Android candidates into bounded `decompile_batch` requests and follow `next_offset` until null; a single candidate is a one-item batch. A saved reconstruction generation does not change this analysis workflow. Only explicitly requested source reconstruction uses the recompile skill's claim, evidence and submit workflow. Managed code uses advertised bulk or paginated type/method operations. Never use direct `procedure_pseudo_code`, UI navigation or visible-text exports as a substitute; report unavailable or failed batch commands. Check enough callers, callees or assembly to establish each candidate's responsibility. Choose a valid, concise C identifier describing observed behavior. Avoid names that assert an unverified purpose, such as `steal_credentials`, from one string or API call.
4. Call `rename_function` only for supported names. In Prism, pass the function address in the `function` argument for renames and notes. Add a one-sentence `add_note` when it preserves a useful reason or an uncertainty that the name cannot express. Keep track of successful edits separately from suggestions awaiting review or failed edits.

Report the functions changed with addresses and evidence, the number actually renamed, and any important candidates left unnamed with the reason. A naming pass does not establish complete reconstruction coverage.
