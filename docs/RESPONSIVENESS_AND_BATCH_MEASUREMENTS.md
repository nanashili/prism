# Responsiveness, persistence, and backend measurements

> Historical measurements captured while Java Ghidra through REA was the
> default. Current macOS and Linux desktop builds select Rust Ghidra through
> REA by default. The Rust provider was not measured in these runs; see
> [current setup and limits](REA_RUST_PROVIDER.md).

Measured on 2026-10-19 on this Mac, using isolated Prism support directories
and Prism's own executables. No Unity job, user analysis store, or live GUI was
modified. The existing temporary profiling client was reused; no test, fixture,
or harness was authored. These are single-run observations, not statistical
performance guarantees.

## Native before/after comparison

Artifacts are under `/tmp/prism-responsiveness-20261019`. The before native host
SHA-256 is `8b24c23b00a541584f86d587a61442985a25d1b24705925edb46be1598dc1bdb`;
the after native host is
`edc7131374c17fefd5258d8aa08fb331bfe2a96fe5ad5bfbcf386ecda08bebce`.
Both analyzed the same Prism binary, SHA-256
`dad3565ac398b75704b31d651e57e87dbfbda35589eb151b8c5bc3bf32e913fc`,
with 20,104 saved functions. This comparison predates the optional Reference
backend and uses the built-in Rust engine on both sides.

| Operation | Before | After |
| --- | ---: | ---: |
| MCP ping queued behind a four-function batch | 453.13 ms | 0.42 ms |
| Same four-function batch | 453.12 ms | 440.45 ms |
| First 256 indexed functions | 6.48 s | 6.22 s |
| A separate 32 harder functions | 62.79 s | 60.81 s |
| Peak native RSS, 256-function run | 155.06 MiB | 155.36 MiB |
| Peak native RSS, harder run | 498.80 MiB | 498.41 MiB |
| Reconstruction evidence/source persistence | 1.75 s | 2.01 s |
| Peak native RSS during that reconstruction run | 102.95 MiB | 99.05 MiB |

The protocol change removes the measured head-of-line delay. It does not make
the decompiler itself 1,000 times faster. Both decompilation selections finished
with all functions successful, no failures, identical result digests, and no
inventory expansion. The small throughput differences are insufficient to
establish a general speedup. Batch RAM is effectively unchanged.

Reconstruction measurement supplied 64 existing Prism source files totaling
3,499,786 bytes and exercised 256 evidence reads across 64 pages. It did not
generate or validate a recovered implementation. Both runs encountered the same
seven existing instruction-boundary errors. Evidence time rose from 1.57 to
1.78 seconds, so this pass does not establish faster reconstruction.

A 2,251-address saved selection was rejected before the admission fix and
accepted in 31.06 ms afterward. The private job was immediately paused after
one committed success. The updated maximum is 16,384 explicit addresses within
the existing request-byte limit; larger inventories use a filtered/all job.

## Persistence and checkpointing

Durable batches now retain pseudocode once in their durable artifacts instead
of writing a second generic view-cache copy. The 256-function comparison had
256 duplicate generic entries before and four after; those four came from the
deliberate legacy four-function request. The harder 32-function durable batch
went from 32 generic duplicates to zero. Digest and inventory comparisons are
recorded in `integrity-comparison.json`.

The native checkpoint writer streams borrowed metadata through JSON escaping,
buffering, and compression. Observed encoding/compression took 36.3–41.3 ms and
whole checkpoint writes took 46.8–52.3 ms for roughly 8.2 MB encoded input. The
reader schema is unchanged. This removes temporary whole-checkpoint strings;
it does not eliminate the synchronous full-program traversal or filesystem sync.
An old native executable successfully decompiled an uncached own-binary
function using a copied new checkpoint store, but that request alone did not
prove the old reader actually restored the checkpoint.

## Java Ghidra through REA comparison

`prism-ghidra-rea` compiles an explicit `ghidra-rea` provider. This uses the
installed Reference REA and Java Ghidra, with one
decompiler worker and a 1 GiB Java heap in this run. The external tools are not
bundled into Prism. See [backend contract and setup](REFERENCE_BACKEND_PARITY.md).

The successful first integration run is
`/tmp/prism-responsiveness-20261019/reference-feature/report.json`, label
`reference-ready`. It analyzed Prism's standalone MCP helper, SHA-256
`cd8fefb292a7e91a395727ecdf38f3e31b9f3193bc52a13cfb8a6b2d604c2309`.
This is a different binary and function selection from the native comparison;
the timings must not be compared as engine speed ratios.

- Initial explicit preparation: 17.18 seconds; 1,172 indexed functions.
- Selected saved functions: 320; successful: 320; failed: zero.
- Durable batch completion: 7.13 seconds, using provider batches of 256 and 64
  and five result pages. Last sampled status request: 4.02 ms.
- Peak Java RSS: 813.81 MiB. Peak Node RSS: 525.91 MiB.
- Peak full process-tree RSS: 1,504.38 MiB, approximately 1.47 GiB.
- Peak summed process physical footprint: 1,445,463,456 bytes, approximately
  1.35 GiB. RSS sums can double-count shared pages; neither metric is a hard cap.

The 1 GiB setting limits Java heap, not combined Java, Node, native decompiler,
and Prism memory. A small saved inventory does not establish a safe heap limit
for large applications. The prepared external session stays alive while the
project is selected; switching projects/backends or shutdown releases it.

### Final release restart/resume check

The integrated release host SHA-256 is
`fd98755ddba072eb226a9f37fc00d2c0a157a0931be3e5b48cc80d599b22115e`.
`reference-feature/lifecycle-report.json` records a 512-function job paused with
65 successful artifacts, followed by orderly process shutdown and a new host.
Reopening took 6.76 seconds, retained all 65 successes, and matched the recorded
backend configuration. Its status call took 1.71 ms. Resuming completed all 512
functions with zero failures in 11.84 seconds; the two committed pseudocode
results sampled before and after restart were unchanged. This confirms durable
continuation on this binary; it does not establish cache-only Ghidra startup.

This run experimentally supplied
`NODE_OPTIONS='--max-old-space-size=256 --max-semi-space-size=8'` to the private
profile processes. Full-tree peak RSS was 883.83 MiB before shutdown and
928.47 MiB after restart. Node peaked at 310.58 MiB and Java at 460.59 MiB in the
second process. The run reused a warm Ghidra archive, used a release Prism host,
and selected more functions than the first run. These differences prevent
attributing the total reduction to the Node cap or claiming unchanged speed.
No production Node heap cap was added from this small-inventory observation;
a 200,000-function inventory may need substantially more Node heap.

## Responsiveness scope and remaining gaps

The implementation also removes session preparation from the model-popup
opening path, reuses a revisioned immutable option snapshot, and bounds visible
transcript rendering. ACP permission admission and ordinary MCP requests are
queued asynchronously with bounded counts/bytes. Read the corresponding
[sidebar](SIDEBAR_RESPONSIVENESS_IMPLEMENTATION.md),
[ACP](ACP_RESPONSIVENESS_IMPLEMENTATION.md), and
[tool scheduling](TOOL_RESPONSIVENESS_IMPLEMENTATION.md) reports for limits.

No native GUI click/frame-latency measurement was performed. Synchronous pipe
writes to a non-reading child and cooperative native cancellation remain
limitations. No complete 200,000-function run, cross-platform runtime check,
or AI reconstruction throughput claim is made. The real Reference path works on
the measured binary; its large-project throughput and resource use remain to
be measured separately.
