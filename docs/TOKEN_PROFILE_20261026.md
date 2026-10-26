# MCP token profile — 2026-10-26

This report measures the round-three MCP response reduction and prompt assembly
changes. It uses exact `o200k_base` tokenization from `tiktoken 0.12.0`; it is
not a billing estimate. The reports serialize parsed MCP values into a
deterministic JSON surrogate, so the envelope measurement is not a capture of
transport framing or provider billing.

## Result

The largest baseline response, `analysis_tools`, fell from 844,210 parsed JSON
tokens to 20,159 tokens: 97.6% fewer. Across the same 51-call feature fixture,
response tokens fell from 892,216 to 68,125, approximately 92.36% fewer. The
full serialized MCP result envelope fell from 1,779,885 to 41,348 tokens. The
fixture had 46 successful calls and five specifically checked expected errors;
there were no unexpected successes.

| Measurement | Baseline | Confirmed after | Reduction |
| --- | ---: | ---: | ---: |
| Largest parsed `analysis_tools` response | 844,210 | 20,159 | 97.6% |
| 51-call parsed responses | 892,216 | 68,125 | 92.36% |
| Full serialized/parsed MCP result envelope | 1,779,885 | 41,348 | 97.68% |

The envelope includes both text and structured result data in the deterministic
surrogate. It does not include captured wire framing or claim anything about
paid-agent billing.

## Catalog behavior

The default summary keeps all 199 allowed tool names, full descriptions, and
required argument names. The full catalog remains exact. Named catalog
requests return the exact schema and full detail for up to 32 requested names;
unknown names and disallowed provider or batch-only names remain rejected. No
tool or evidence capability was removed. The default response omits nested
input/output schemas. Clients that read schemas from the default response must
now request `{"detail":"full"}`; adding `"names":["binary_overview"]` selects
one tool without returning every schema. Summary discovery plus that named
schema costs 22,606 tokens, 97.32% less than the previous full discovery call.

The confirmed live acceptance checks are green for the default summary,
required fields, full-catalog exactness, named-catalog exactness, unknown-name
rejection, blocked-provider rejection, batch-only-name rejection, and the
`analysis_call` evidence receipt. The earlier baseline is intentionally RED:
its old binary still rejects the new `detail` argument.

The source-derived prompt fixture applies the same split to Codex prompt
assembly: static guidance is sent once on `session/new`, dynamic state is sent
on each prompt, and later prompts omit the repeated static block. Non-Codex
prompts retain their existing context behavior. Slash commands still bypass
context, reconnect history still reaches session initialization, and attachment
handling is unchanged. Session identity now includes static instructions and
the tool timeout, so changing either setting reconnects the agent.

The fixed source fixture measures 1,362 repeated prompt tokens saved per
ordinary Codex prompt (`1,421` to `59` tokens). Static guidance itself is
`1,362` tokens before and `1,374` after, because the new protocol wording adds
small persistent guidance. This is source-derived fixed-fixture evidence, not a
live paid-agent conversation.

## Hotpath observations

The profiler comparison uses the baseline
`build/profile/round2/features-atomic-final/features.json` and the confirmed
`build/profile/round3-token/features-confirmed/features.json`. The earlier
round-three candidate artifacts are excluded from this comparison. The
`hotpath_before` snapshot is taken after catalog discovery, before the main
feature workload. Its measured values were:

| Discovery phase observation | Baseline | Confirmed after |
| --- | ---: | ---: |
| Application MCP discovery elapsed time | 367.367 ms | 173.450 ms |
| Profiler-reported RSS | 143.4 MB | 92.2 MB |
| Profiler-reported gross allocations | 265.2 MB | 59.8 MB |
| Instrumented worker `remote` time | 86.46 ms | 155.22 ms |
| Instrumented worker `remote` allocations | 5 B | 490.4 KB |

Constructing the summary adds worker work while reducing response construction
and transport outside that scope. These are single workload observations, not
a benchmark series or evidence of a general CPU improvement. After restart,
the final 22-call worker scope took 381.88 ms before and 512.13 ms after;
its allocations were 6.2 KB in both runs. Token reduction is the demonstrated
primary gain.

Hotpath timing includes instrumentation and wall-clock protocol/provider waits.
Timing rows can nest and overlap. Allocation counters describe instrumented
gross allocations, not retained memory or RSS. The hotpath data is useful for
locating the measured catalog and MCP work; it does not establish whole-system
CPU cost.

## Feature, smoke, and verification artifacts

The confirmed feature workload is recorded at
`build/profile/round3-token/features-confirmed/features.json`: `status: ok`,
empty errors, 51 calls, exact 321/321 function/index coverage, synthetic
execution evidence, prototype-shaped type recovery, and no static CFG blocks
for graph correlation. It is a bounded feature fixture; it does not provide a
live debugger trace.

The candidate macOS smoke artifacts are retained at
`build/profile/round3-token/desktop-final/profile.json` and
`build/profile/round3-token/native-final/output.json`. They were produced with
the earlier `round3-token-final` candidate binary. They are useful acceptance
evidence for the recorded desktop/native workloads, but this report does not
claim they used the confirmed round-three binary hash. Rust changes after that
candidate run were test/lint-only.

The desktop smoke visited 13 views and 15 settings pages and retained a
500-message chat across reopening. Native smoke decompiled seven functions
across The Unarchiver, Chrome, and CodexBar using REA
`6.7.0+189f3474bfc5`. Runtime binary hashes are recorded in the final
verification artifact. Those checks do not establish every interactive state.

The confirmed Rust matrix is 24/24 checks passed, with 509 Rust files and 876
project files, at
`build/rust-review/round3-token-confirmed/coverage.json`. The inventory is
automated coverage, not proof of manual review of all 509 Rust files. Manual
review was scoped to the changed prompt/catalog/session paths and their
immediate dependencies.

## Provenance

| Artifact | Role |
| --- | --- |
| `build/profile/round3-token/baseline-confirmed/metrics.json` | RED baseline metrics from the prior `round2-atomic-final` binary |
| `build/profile/round3-token/after-confirmed/metrics.json` | GREEN confirmed metrics from the round-three feature binary |
| `build/profile/round3-token/features-confirmed/features.json` | 51-call feature fixture and hotpath snapshots |
| `build/profile/bin/round2-atomic-final/binaries.json` | Baseline binary manifest |
| `build/profile/bin/round3-token-confirmed/binaries.json` | Confirmed binary manifest |
| `build/profile/round3-token/prompt-source-metrics.json` | Source-derived prompt split and fixed-fixture counts |
| `build/rust-review/round3-token-confirmed/coverage.json` | 24-check Rust verification and automated inventory |
| `build/profile/round3-token/final-verification.json` | Final artifact hashes, runtime identity, and source-inventory freshness |

The baseline metrics source is
`build/profile/round2/features-atomic-final/features.json`; the confirmed
metrics source is `build/profile/round3-token/features-confirmed/features.json`.
Their recorded source hashes are retained in the two metrics files.

The confirmed measurement and verification commands are:

```sh
python3 scripts/profile-features.py \
  --prism build/profile/bin/round3-token-confirmed/prism \
  --output build/profile/round3-token/features-confirmed
build/profile/round3-token/tokenizer-env/bin/python scripts/profile-tokens.py \
  --report build/profile/round3-token/features-confirmed/features.json \
  --baseline-report build/profile/round2/features-atomic-final/features.json \
  --live-prism build/profile/bin/round3-token-confirmed/prism \
  --live-target build/profile/round2/features-atomic-final/fixture-v2 \
  --output build/profile/round3-token/after-confirmed
RUSTC_WRAPPER= python3 scripts/check-rust.py \
  --report-dir build/rust-review/round3-token-confirmed
```

## Findings fixed

| Priority | Rule and location | Consequence | Fix |
| --- | --- | --- | --- |
| P2 | Resource efficiency: `src/worker/analysis_tools.rs::project_provider_tools`, `src/mcp/catalog/analysis.rs::definitions` | Nested schemas made ordinary catalog discovery dominate context size. | Return all names, guidance, and required argument names by default; retain exact named/full schemas on demand. |
| P2 | Explicit context lifetime: `src/ui/chat_session.rs::prompt_context`, `src/agent/prompt_content.rs::render_text`, `src/agent/codex.rs::custom_instructions` | Static guidance was repeated in every Codex prompt. | Retain it at session creation and send current dynamic state per prompt; preserve the other agent and attachment paths. |
| P2 | Configuration correctness: `src/agent/session.rs::SessionIdentity`, `src/agent/codex.rs::configure_launch` | A changed tool timeout could silently retain the previous launched MCP configuration. | Include timeout and static guidance in session identity; a behavioral regression test verifies reconnect selection. |

These findings describe the measured corrections and do not claim universal
application correctness.

## Limits

The evidence is macOS-local and uses synthetic feature execution observations,
prototype-only type recovery, and a fixture without static CFG blocks. It does
not cover live debugger or Frida traces, paid-agent conversations, Windows or
Linux behavior, or every provider/UI state. Prompt savings are fixed-fixture
source measurements, and MCP envelope sizes are deterministic parsed-value
surrogates rather than network or billing measurements.
