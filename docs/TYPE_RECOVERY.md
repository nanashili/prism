# Shared type and structure recovery

`recover_types` is an agent-assisted, provider-independent recovery workflow for
indexed native functions. The agent reads saved batch pseudocode and advertised
assembly/call evidence, then supplies observations. Prism merges those observations
interprocedurally and checks the resulting shared model before accepting it.
This is not a machine-code data-layout analysis pass or a rev.ng integration.

The design follows the useful separation in rev.ng's
[data-layout analysis](https://docs.rev.ng/references/analyses/#analyze-data-layout-analysis):
memory-access information from multiple functions contributes to shared layouts.
Prism's inputs are explicitly agent hypotheses. Internal consistency does not verify
that a cited instruction actually performs the described access.

## Workflow

1. Read existing accepted types with `recover_types` action `read`, supplying the
   open project's SHA-256 as `project`. Optional `address` filters the model to a
   function and its transitive pointee dependencies. `observations` always contains
   the complete accepted evidence snapshot so it can be revised consistently.
2. Gather memory accesses and call-site observations from the saved inventory.
   Use durable `decompile_batch` results and advertised assembly tools; do not
   discover additional functions as a side effect of recovery.
3. Call action `propose` with `evidence`. It merges new observations with accepted
   evidence and replaces the one pending proposal for this project and analysis
   configuration. No accepted consumer changes at this stage. The default
   `update: "merge"` is additive; use `update: "replace"` with a complete revised
   snapshot to retract incorrect observations or rename accepted types. Include
   every consumer that the corrected evidence still supports. An empty replacement
   containing only `pointer_size` withdraws all accepted types and prototypes.
4. Inspect the returned model and conflicts. Action `check` with the returned
   `proposal` ID reruns inference and inventory checks. Correct the incoming
   observations and propose again when necessary.
5. Action `accept` with that exact ID repeats the checks inside an immediate
   transaction. Conflicts, missing functions, stale revisions and superseded IDs
   prevent acceptance. The complete model and every consumer change atomically.
   A failed write preserves both the previous accepted model and the proposal.

Accepted types are namespaced by binary identity and the full analysis
configuration. They persist in the local Prism database. They accompany legacy
and durable decompilation result pages and reconstruction evidence, including
cached results. Each page receives one shared model, rather than separately
copied type definitions per function. Historical durable jobs use their saved
analysis configuration when looking up the accepted model. Raw pseudocode and
its content digest remain provider artifacts; `provider_types_applied` is false.
Types are not currently included in exported project archives or submitted back
to the provider's type system.

## Observation format

Every base is scoped to a function: `{"function":"0x1000","variable":"arg0"}`.
Use the exact canonical addresses from `list_functions`. `site` is a bounded
instruction or call-site reference with enough context for an analyst to check.

```json
{
  "project": "<open project SHA-256>",
  "action": "propose",
  "evidence": {
    "pointer_size": 8,
    "accesses": [
      {
        "base": {"function": "0x1000", "variable": "arg0"},
        "offset": 0,
        "width": 8,
        "kind": "integer",
        "count": 5,
        "stride": 8,
        "site": "0x1010: indexed load, observed loop bound 5"
      },
      {
        "base": {"function": "0x2000", "variable": "node"},
        "offset": 40,
        "width": 8,
        "kind": "pointer",
        "pointee": {"function": "0x2000", "variable": "node"},
        "site": "0x2010: load next pointer"
      }
    ],
    "aliases": [
      {
        "left": {"function": "0x1000", "variable": "arg0"},
        "right": {"function": "0x2000", "variable": "node"},
        "site": "0x2020: passes node unchanged to 0x1000"
      }
    ],
    "names": [
      {"base": {"function": "0x1000", "variable": "arg0"}, "name": "Node"}
    ]
  }
}
```

`aliases` records only pointers with the same origin. A pointer to `object + 8`
must not be equated with `object`. Similar offsets alone never establish aliasing.
Transitive aliases combine every recorded consumer's accesses into one layout.
Names must agree throughout an alias group and cannot name unrelated groups.

Access kinds are `bytes`, `integer`, `float`, `pointer`, and `function_pointer`.
Pointer widths must match `pointer_size` (4 or 8). A data pointer can identify its
`pointee` base; a function pointer can name an indexed `target` function. Two or
more contiguous pointer-sized function slots produce `possible_vtable`, preserving
targets and observations. Such a table can also be an ordinary dispatch table.

An access defaults to one element and a stride equal to its width. Repeated
accesses with explicit stride and count describe a candidate array. Compatible
individual element accesses merge into that field with all provenance retained.
Field extents and arithmetic are bounded and checked. `minimum_size` is the
largest observed end offset, not an allocation size or a proven ABI `sizeof`.
Padding, packing and alignment remain unknown.

`prototypes` records independent call-site or definition observations:

```json
{
  "function": "0x1000",
  "convention": "system_v64",
  "parameters": [
    {"kind": "pointer", "width": 8,
     "pointee": {"function": "0x1000", "variable": "arg0"}}
  ],
  "returns": {"kind": "integer", "width": 8},
  "site": "0x2020: call arguments and return register"
}
```

Supported convention observations are `unknown`, `cdecl`, `stdcall`, `fastcall`,
`thiscall`, `system_v64`, `win64`, and `aapcs64`. A known convention refines
`unknown`; contradictory known conventions, parameter counts/types, return types,
or pointer sizes prevent acceptance. Omit `returns` for an observed void result;
an ignored return value at a caller is not evidence of a void prototype. These
checks do not independently verify register assignments, stack cleanup, target
architecture or variadic arguments.

Overlapping incompatible accesses remain conflicts. Prism does not silently
invent unions, array bounds, inheritance or displaced-pointer relationships.
Interleaved strided layouts may require richer evidence than this model supports.
Only recorded consumers are covered. An unobserved function cannot be assumed
consistent with the model.

Evidence is limited to 1,024 accesses, 512 aliases, 256 prototypes, 256 names and
384 KiB per accepted snapshot. A proposal request is limited to 400 KiB and the
inferred model to 768 KiB. These limits also apply after merging existing evidence.

## Verification

The storage-boundary tests exercise transitive sharing, recursive pointers,
array-element coalescing, possible vtables, prototype reconciliation, persistence,
project/provider isolation, conflicting layouts and ABI observations, superseded
IDs, live inventory changes, invalid extents and a database failure between model
publication and proposal removal. They use a real isolated SQLite store.

Run `cargo test --locked --bin prism storage::type_recovery` and the repository-wide
Rust checks described in the README. These checks establish deterministic behavior of
the agent-evidence workflow, not equivalence to rev.ng's LLVM-based analysis or
correctness of any recovered binary layout.
