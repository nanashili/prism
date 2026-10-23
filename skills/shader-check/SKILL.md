---
name: shader-check
description: Inspect shader or GPU code and, when a recording exists, tie performance findings to measured evidence.
---

# Check shader and GPU work

Separate static shader observations from measured performance.

1. Identify the available material. Prism currently exposes generic shader or resource text but no GPU profiling workflow. Read only the code, metadata, and CPU links actually present; an absent "Loaded by" link does not prove the shader is unused.
2. Explain the shader stage or kernel's inputs, outputs, memory access, dispatch assumptions, and any visible synchronization or precision choices. Use the CPU call site when available to understand dimensions and frequency. In Prism, any native CPU pseudocode must use `decompile_batch`, following `next_offset` until null; one function is a one-item batch. A saved reconstruction generation does not change this analysis workflow. Only explicitly requested source reconstruction uses the recompile skill's claim, evidence and submit workflow. Never substitute direct `procedure_pseudo_code`, UI navigation or visible-text exports; report unavailable or failed batch commands. IR and metadata can suggest costs but cannot establish a runtime bottleneck.
3. Prism has no built-in GPU recording or `gpu_profile` tool. If the user supplies a recording or measurements through an available external tool, use only the timing information that source actually provides. Do not attribute a measured cost to a particular shader without corresponding trace evidence. Reading a recording is distinct from launching the app to make a new one.

Report what the shader does, which CPU path is linked to it, the measured numbers if available, and one prioritized hypothesis or change with its evidence. If no recording or per-kernel timing exists, label performance advice as a hypothesis and identify the measurement needed. Do not prescribe threadgroup memory, extra frames in flight, or pipeline prewarming solely from a common pattern.
