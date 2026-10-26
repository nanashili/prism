# Evidence-backed claims

Prism records agent claims separately from analysis output. Every recorded claim must
reference one or more immutable evidence snapshots captured by Prism. Agents cannot
upload arbitrary snapshot contents, assign runtime provenance, or set a claim's status
or confidence. Shared MCP and desktop instructions require substantive target claims
to be recorded, checked, and cited in replies.

This is a deterministic predicate verifier. It checks captured values and exact text
occurrences. It does not infer that an arbitrary behavioral description follows from
a quotation. ACP replies remain free-form text: citation coverage in every sentence
depends on the agent following the instructions; the registry enforces evidence for
claims submitted through its tools.

## Capture evidence

`verification_evidence` is available in every MCP mode. Capture a saved pseudocode
result after a durable decompilation job succeeds:

```json
{"action":"capture","view":"pseudocode","address":"0x14001a20"}
```

The tool returns `data` and a `prism_evidence` receipt containing an identifier and
SHA-256 digest. Pseudocode capture reads an existing durable artifact for the active
provider configuration. It never starts decompilation. Assembly and references are
read from the current provider rather than an evictable view cache.

Successful `analysis_call` replies now have the shape
`{"result": <provider reply>, "prism_evidence": <receipt>}`. The snapshot contains the
provider reply itself, so pointers start at its root rather than the receipt wrapper.
Provider errors do not become evidence. Discover provider names and schemas with
`analysis_tools` as usual.

Use `{"action":"get","evidence":"<64-character identifier>"}` to inspect a snapshot,
its provenance, and whether it is still current. Storage is bounded to 512 KiB per
snapshot and 32 MiB per project; evidence is retained rather than silently evicted.
Provenance is limited to a 32 KiB source key and a 1024-byte subject, and counts
toward the project budget.
If saving a new observation fails, affected old claims reopen and the original
snapshot remains available.

## Propose and check claims

For a pseudocode snapshot containing a call followed by a comparison, submit the
specific, falsifiable facts separately from the interpretation:

```json
{
  "action":"propose",
  "draft":{
    "statement":"The saved pseudocode calls crc32 and compares the computed value",
    "scope":"predicate",
    "checks":[
      {"evidence":"<snapshot identifier>","pointer":"/text",
       "predicate":{"operation":"contains","text":"crc32(input, length)"}},
      {"evidence":"<snapshot identifier>","pointer":"/text",
       "predicate":{"operation":"contains","text":"computed == expected"}}
    ]
  }
}
```

`verification_claim` returns a claim ID and a citation such as `[claim:17]`. Run
`{"action":"verify","claim":17}` to evaluate the cited evidence. `equals` compares
any JSON value with `expected`; `contains` checks exact, case-sensitive text. Pointers
follow JSON Pointer syntax, including `~0` and `~1`; an empty pointer selects the root.
Each claim requires 1–16 checks. All checks are required, rather than counting duplicate
citations as independent corroboration.

A statement such as “this function validates checksums” uses `scope: "interpretation"`.
Its calls and comparisons remain inspectable, but textual matches cannot promote that
broader statement to corroborated. Semantic verification remains required. A predicate
statement must describe exactly its declared checks; declaring that scope does not
give arbitrary prose a logical proof.

| Status | Meaning |
| --- | --- |
| `proposed` | References were validated, but checks have not run. |
| `corroborated` | All declared predicates match current snapshots. |
| `contradicted` | A current captured value differs from a required exact value. |
| `runtime-observed` | Every required exact value was captured from debugger replies. |
| `unresolved` | Evidence is stale, absent, corrupt, inconclusive, or requires semantic review. |

Confidence is categorical: `none` for unchecked or unresolved claims, `limited` for
text matches, `direct` for exact captured values, and `observed` for exact debugger
observations. It describes the evidence for the reported verdict, including a
contradiction. These labels are not probability estimates or model confidence.
A failed text search remains unresolved because incomplete analysis cannot establish
that behavior is absent.

## Runtime observations and reopening

Successful UI debugger requests for evaluation, stack frames, variables, and scopes
are captured when the launched target's SHA-256 matches the opened project. Use
`{"action":"latest_runtime"}` on `verification_evidence` to read the latest observation.
Attaching to an arbitrary PID does not establish binary identity and does not create
these snapshots. Imported trace labels cannot assign runtime provenance to claim
evidence. Runtime predicates must select `/reply` or a descendant; request arguments
and capture metadata cannot establish a runtime observation. Observations apply only
to that execution and become stale on the next
debugger operation or project switch.

Claims automatically reopen as unresolved when supporting snapshots change, when
indexing starts, when provider configuration changes, or when a durable pseudocode
result changes. Unknown provider operations invalidate claims before execution,
including when the operation subsequently fails. Known read operations remain usable
for collecting several evidence sources. Changes made outside Prism can only be
detected when Prism observes them; this is not a live provider subscription.

Use `{"action":"list","offset":0}` to page through claims, following `next_offset`.
`{"action":"get","claim":17}` returns the current claim and eight recent revisions.
After reopening, use `action: "revise"` with the claim ID and a new draft citing fresh
evidence, then verify again. Old snapshots and the complete revision history remain
stored. Re-verifying an old citation never silently substitutes a newer snapshot.

Snapshots and claims live in the local profile's `prism.db`, separately from the view
cache. They survive application restarts; project archives do not yet export them.

## Verification

Storage tests exercise status derivation, project isolation, restart persistence,
configuration changes, runtime invalidation, durable-result invalidation, corrupted
evidence, and partial write failure. Run `python3 scripts/check-rust.py` for the
repository's feature matrix. These deterministic checks do not establish live
provider or interactive debugger behavior on any platform.

On 26 October 2026, a rebuilt development executable passed the local macOS REA
smoke harness with native REA 6.7.0: one function each from the Chrome launcher and
The Unarchiver produced successful saved pseudocode. A separate real MCP session
passed evidence capture, limited corroboration of a literal excerpt, unresolved
semantic interpretation, provider-reply receipts, reopening after a failed unknown
provider operation, immutable stale evidence, and restoration after process restart.
The isolated profile and receipts are under `build/verification-smoke/`, including
`rea-smoke.json`, `claims.json`, and executable hashes in `provenance.json`.
This acceptance run did not exercise the interactive debugger.
