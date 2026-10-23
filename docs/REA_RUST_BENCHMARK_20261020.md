# REA Rust provider measurements, 2026-10-20

On the same frozen host and original 320-address selection, Java completed 320/320 in both cold and warm runs. Rust saved 192/320, then paused after its worker-family RSS watchdog exceeded the requested 1,024 MiB budget. Reopening the paused Rust store preserved all 192 outputs and the checkpoint at offset 192. This establishes partial-result durability, not a successful full Rust throughput run or an improvement over Java.

These are operational observations on an existing, independently authored Prism executable. The existing profiler `/tmp/prism-batch-memory-20261019/profile_batch.py` drove public Prism MCP tools with isolated settings, stores and caches. No tests, test fixtures or test harnesses were authored. No target execution or Unity/proprietary binary analysis occurred.

## Frozen inputs and method

- Primary raw artifacts: `/tmp/prism-rea-rust-20261020/full-interim-1/`.
- Measured host: optimized, symbolized `prism-after`, 22,509,624 bytes; SHA-256 `2fce8d9d525e59a7bb2866f086a62f29b904fb1bf6d1bebc3d61f334db4924c9`. Every primary Java and Rust observation below uses this exact private copy. The stripped shipping executable is recorded separately and was not measured here.
- Target: existing arm64 Mach-O `prism-before`, 554,016 bytes; SHA-256 `cd8fefb292a7e91a395727ecdf38f3e31b9f3193bc52a13cfb8a6b2d604c2309`.
- Original selection: 320 addresses in `addresses.json`, 4,800 bytes; SHA-256 `8e12979f929f176c494ee6421dd16ca394d0bf766b0737fda4cb05cce5fc279c`.
- Environment: macOS 27.2 arm64, 10 logical CPUs, 32 GiB installed memory; Reference-bundled Node 24.18.0.
- Java reference: installed Reference REA 6.5.0, Ghidra 12.1.4 and OpenJDK 25.0.2. Rust adapter: compiled local clone `/path/to/rea-prism`. Durable job configuration records component paths and hashes.
- Both providers request 1,024 MiB and one batch worker. Java heap configuration and the Rust sampled worker-family RSS watchdog enforce different policies; they are not an identical OS hard memory limit. Rust request timeout was 300 seconds, with the overall operational attempt bounded to three minutes. The budget was not raised.

Cold means empty private analysis and Node compile caches. Java warm reuses a preparation snapshot copied before its cold batch, with a fresh Prism database and no completed job artifacts. Rust reopen uses the exact paused store and executable path, with no new decompilation. The OS filesystem cache was not flushed. `PRISM_PROFILE_DECOMPILE` was unset for all primary observations.

Preparation includes the existing profiler's executable copy, process spawn, target preparation and MCP initialize. Batch time includes job creation, execution and 500 ms status polling; result retrieval, metadata and cleanup follow it.

## Same-host observations

| Operation | Preparation seconds | Batch seconds | Saved successes / failed results | Terminal state |
| --- | ---: | ---: | ---: | --- |
| Java cold, original 320 | 17.802 | 7.128 | 320 / 0 | Completed |
| Java warm, original 320 | 7.230 | 7.138 | 320 / 0 | Completed |
| Rust cold, original 320 | 3.279 | 81.804 | 192 / 0; 128 pending | Paused: memory constraint |
| Rust paused-store reopen | 5.590 | No batch requested | Same 192 retained | Paused, unchanged |

The Rust watchdog reported 1,283,325,952 bytes against a 1,073,741,824-byte RSS limit. Complete 64-address windows became durable at 13.297, 74.630 and 75.668 seconds after batch start; the job was observed paused at 81.804 seconds. The next window was not persisted. The full attempt, including result reads and host finish, took 88.148 seconds. Per-function timing was disabled, so the active function at failure is not established.

| Run and phase | Family peak RSS MiB | Family peak physical footprint MiB | Observed family CPU seconds |
| --- | ---: | ---: | ---: |
| Java cold preparation | 1,215.359 | 1,190.985 | 36.395 |
| Java cold batch | 1,413.828 | 1,337.595 | 10.528 |
| Java warm preparation | 919.313 | 876.640 | 16.227 |
| Java warm batch | 1,128.813 | 1,016.266 | 12.190 |
| Rust cold preparation | 572.766 | 428.558 | 1.826 |
| Rust cold batch | 1,449.625 | 1,627.062 | 81.421 |
| Rust paused and retained | 25.281 | 23.782 | 0.030 |
| Rust reopen preparation | 503.875 | 437.809 | 2.523 |

Whole-profile peaks include metadata and allocation inspection and are excluded from the phase values above. Java completed-retained family RSS peaks were 1,408.063 MiB cold and 1,132.219 MiB warm. The Rust native worker's individual batch RSS peak was 1,309.344 MiB; REA Node's was 390.250 MiB. Individual peaks are not simultaneous and must not be added. The sampler and watchdog observe independently, so their recorded peaks can differ.

## Coverage, saved results and restart integrity

Java exposed 1,172 public inventory rows and Rust exposed 981. All Rust addresses occur in the Java inventory; Java has 191 additional addresses. All original 320 selectors occur in both inventories. Java cold/warm inventories are identical, as are Rust cold/reopen inventories. These differences prevent an inventory-parity claim.

Java returned exactly 320 unique, nonempty outputs in each run; all stored SHA-256 output digests recomputed correctly and all 320 cold/warm address-to-digest pairs match. Its result pages contain 100, 100, 100 and 20 rows. Rust returned exactly the first 192 selected addresses, all unique and nonempty with matching digests. Its result pages contain 84, 100 and 8 rows because byte and row limits both apply. These are artifact-integrity checks; semantic correctness remains unverified.

The Rust durable database contains 320 selection items, 192 successful result rows, zero failed result rows, 264,830 artifact bytes and `next_offset=192`. The paused job ID is `af97a0b77ef2b13c1d3d6c030b8b1d13874619d3bc82249f79278cc4f0c46c38`. After reopening at the same executable/support paths, `configuration_matches=true`; all 192 address/digest pairs, counts, timestamps and the resource error remain unchanged. Result offset 191 returns `0x10000f3dc`; offset 192 returns no saved row. The next selected address is `0x10000f40c`, with 128 selectors remaining.

The saved schema-1 snapshot identifies the target hash and 981 seeded addresses, with `automatic_discovery=false` and null discovery/call data. Its SHA-256 is unchanged after reopening. Public session metadata reports the `saved-or-loader-v1` policy and idle provider health. It does not expose the native `restored=true` flag; the public `snapshot_replay=false` configuration field is not a cache-hit observation. Explicit native restore metadata remains unverified. No discovery, resume or retry was requested. The existing resume operation schedules execution immediately, so actual resumed execution of the known-failing remainder was deliberately left unverified. Unknown-address rejection was not exercised in this measurement window.

The original cold logs, samples, summary and result pages were preserved before the same-path reopen. Their hashes remain unchanged; the reopen has separate output files. Read-only SQLite inspection after teardown independently confirmed the paused job and 192 rows.

## Teardown

The Rust failure's host-finish measurement, including a retained sample and `vmmap`, took 1.936 seconds. Host exit was zero, and every observed PID was absent at the first check 4.7 ms after finish. This verifies failure cleanup on the measured build. Reopen finish took 3.744 seconds; all observed PIDs were absent at its first 5.1 ms check. No manual process signal was needed for these runs.

Java cold/warm finish measurements took 5.626/4.749 seconds. Some Java/demangler children briefly remained after the host exited; the entire observed PID sets were absent by 0.425/0.426 seconds after finish. PID/start-time accounting and settled `ps` observations are preserved in each summary. A successful five-address run on this host also measured `close_binary` at 350.63 ms; the later paused-store reopen logged successful `close_binary` at 149.28 ms.

## Historical diagnostics

Earlier records remain separate from the same-host comparison. `/tmp/prism-rea-rust-20261020/final/` contains the earlier Java pair on host `63a6980e…`; its timings are superseded above. `accepted/` contains the older `f1a01891…` Rust attempt that paused with zero saved results and slower cleanup. The earlier result-schema initialization failure is retained under `final/initialization-failure-1/`.

`diagnostic-1/` used host `dbc283aa…`, the original 320 selection, a 30-second timeout and opt-in timings. It timed out without a persisted window; its bounded stderr retained 59 completed timing rows. The worker outlived public timeout and disappeared 9.44 seconds after host/Node cleanup. `diagnostic-2/` used host `a7cdc05c…` and only original positions 59–63; all five outputs completed in 13.284 seconds. These earlier runs do not establish current failure cleanup or full-workload performance.

`diagnostic-3/` used the same `2fce8d9d…` host as the primary runs, the same five selectors and a 30-second timeout, with numeric diagnostics and native `MallocSpaceEfficient=1`. It completed 5/5 in 13.788 seconds. Address `0x100004b98` took 13,467.185 ms. `push_multi` was checked 5,427,526 times and applied 27 times; `multicollapse` was checked 5,427,499 times and applied 5,359 times. Assigned arena slots included 219,792 operations and 666,836 varnodes; their live counts were zero after Program cleanup, before Funcdata release. These counters establish substantial native work and allocation, without establishing its cause.

Diagnostic 3 family batch peaks were 787.938 MiB RSS and 784.700 MiB physical footprint. Native physical footprint peaked at 420,463,456 bytes in samples and fell to 44,532,384 bytes in completed-retained samples. A successful post-batch `vmmap` reported 42.5 MiB physical footprint and a 415.2 MiB lifetime peak. Its symbolized five-second sample, strict counters and allocation summary remain under `diagnostic-3/`. Instrumented, narrowed selections are excluded from full-workload throughput claims. The outer Prism transport buffers capped stderr until EOF; retained diagnostics do not establish live progress delivery.

## Final package

The final release uses the same provider/native implementation, with one later
settings-description change marking Rust REA experimental. It restores the
normal release stripping and ad-hoc macOS signing. The pre-bundle executable is
19,880,672 bytes, SHA-256
`c23d7026e2d1b04b81820604cf5b31e99c034eed9f5bde2b8148c8033d2bd6b4`.
The signed bundled executable is 19,765,216 bytes, SHA-256
`589dd317f1c8c5cd065bf6f441faa20427899df52f70e062e0b3ec744482a25b`.
The complete bundle is 23,273,434 bytes (22.20 MiB), excluding external REA,
Node, Java and Ghidra installations. Signature verification passed. These
package sizes are disk sizes, not RAM measurements; no batch benchmark was
repeated on the stripped shipping executable. The separate provenance record
is `/tmp/prism-rea-rust-20261020/final-package.json`.

## Raw artifacts and limits

The primary directory contains `measurement.json`; each `reference-cold`, `reference-warm`, `rust-cold` and `rust-reopen` label has a summary, resource JSON, memory JSONL, inventory, result pages and stderr log. `all-resources.json`, `inventory-comparison.json`, `reference-output-comparison.json` and `rust-cold-component-resources.json` capture the derived observations. Durable state is recorded in `durability-metadata.json`, `saved-database-metadata.json`, `saved-snapshot-metadata.json`, `after-reopen-database-metadata.json`, `reopen-cache-metadata.json` and `reopen-artifact-preservation.json`. Original cold artifacts are also under `cold-attempt-preserved/`. The root `report-artifact-manifest.json` records raw observation hashes.

Sampling requests 200 ms intervals; observed primary median intervals were about 246–254 ms. Short-lived children and intervening peaks can be missed. Family RSS and physical footprint are simultaneous sums over observed processes; shared mappings may be counted more than once, and no unique-page attribution is claimed. CPU uses observed cumulative user/system counters and this host's Mach timebase of 125/3 ns per tick. It is a lower bound; per-phase CPU assigns positive counter changes to the current sampling phase. `cpu-timebase.json` records a comparison with this reporting process's POSIX CPU accounting.

One small executable and one cold/warm pair do not establish a general speedup, Java parity, recovered-source correctness, large-target capacity or statistical performance claim. Rust did not complete the original workload within its configured memory policy. A successful full Rust cold/warm throughput comparison remains unavailable.
