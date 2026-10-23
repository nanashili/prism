# Batch and memory profile

Profile date: 2026-10-19. These results describe the current native worker,
durable batch path and production MCP reconstruction path. They are workload
profiles, not a whole-application memory ceiling or a promise of 200,000
functions per hour. Measurements used release binaries, separate private stores,
the same seed checkpoint and address list, and one native worker. No compiler
ran during the measured phases. OS caches were not forced cold.

## How to read the memory numbers

RSS is the resident portion of a process's virtual mappings. It can include
shared and file-backed pages and can count a shared page in more than one
process's RSS. Physical footprint was collected with macOS
`proc_pid_rusage` (`ri_phys_footprint` and `ri_lifetime_max_phys_footprint`).
Unlike RSS, this accounting excludes reclaimable clean file-backed pages.
The mapping can therefore reduce charged footprint without immediately
removing every mapped page from system RAM. These measurements are platform-
specific. RSS and physical footprint must therefore not be added together or
treated as interchangeable.

The host samples were always taken after the headless implementation was
available. The old comparison binary did not have headless mode, so there is
no valid comparative host-memory result. The current host footprint was about
34 MiB in the retained profile.

## Saved Unity metadata restore

The same checkpoint restored 193,191 indexed functions and retained the same
checkpoint SHA-256 (`a197cfe6552a51bb096d038f66651b45c069a8c4a6194dd546a1409948bf95f0`).
The physical footprint and restore time were:

| Measurement | Before | After |
| --- | ---: | ---: |
| Retained physical footprint after inventory | 411.0 MiB | 289.4 MiB |
| Restore time | 2.558 s | 2.631 s |
| Lifetime physical peak | 522.9 MiB | 409.6 MiB |

The footprint reduction is not a speedup: restore time increased by about
2.8%. No Unity code was analyzed in this profile; it measured saved metadata
restore and inventory only.

## Durable Prism batch

The production durable batch requested 1,024 addresses. Both runs used the
same job and produced 1,023 successes and one failure:

| Measurement | Before | After |
| --- | ---: | ---: |
| Batch elapsed time | 150.737 s | 151.654 s |
| Native physical peak | 778.6 MiB | 755.9 MiB |
| Native RSS peak | about 1.0 GiB | about 1.0 GiB |

The elapsed change is approximately +0.6%. All 1,024 durable result payloads
were byte-identical, including the known 1,024 MiB RSS-budget failure at
`0x10002e8b4`. This profile does not establish behavior for a full 200,000
function export or a global RAM ceiling.

The original Arena remains the production storage path. Paged and dense Arena
alternatives took 164.3 and 162.8 seconds respectively, approximately 8–9%
slower than the baseline, so both experiments were removed. That result is specific
to those experiments and is not a repository-wide throughput claim.

## Production MCP reconstruction profile

The profile processed 256 evidence items in 64 pages. It produced 249
successful evidence results and seven errors because the requested addresses
were outside executable sections. The evidence phase improved from 1.90 s to
1.56 s, while the existing Prism source persisted by the workflow remained
3,499,891 bytes. The 36 source files were submitted through `reconstruction_submit`, and an
identical retry was idempotent. Both databases passed `quick_check`. Evidence,
source objects and unit/action states matched after excluding volatile batch
metadata.

| Measurement | Before | After |
| --- | ---: | ---: |
| Native lifetime peak | 83.03 MiB | 62.86 MiB |
| Evidence phase | 1.900 s | 1.560 s |

This was an evidence and persistence profile. It did not run AI generation or
a build, and all functions were blocked for this profile. The seven errors
must remain visible to callers rather than being presented as reconstructed
output.

## Storage and ownership changes

The executable image is shared through `Arc<ImageBytes>`. On Unix, the image
bytes use a privately owned anonymous-file spool with a read-only mapping;
Windows uses the owned-buffer fallback. The previous shared buffer already
avoided a second executable copy; this change replaces the remaining large
heap allocation with file-backed storage. It requires temporary backing-file
space equal to the selected image size, which is released with the image. Reclamation and re-faulting can change
RSS and physical-footprint observations, which is why both views are recorded.
Existing `Vec` APIs and snapshot schema remain unchanged.

Function records no longer duplicate the current symbol name: the current name
is resolved from the symbol table. Reconstruction, source and MCP payloads
borrow data where their lifetimes permit, and hashing streams bytes instead of
creating another full payload copy. These ownership changes explain the
reduced metadata footprint as a group; this profile does not attribute an
independent percentage to each one.

## Reproduction details

The executable being decompiled was Prism's own 19,730,400-byte pre-change
release, SHA-256
`dad3565ac398b75704b31d651e57e87dbfbda35589eb151b8c5bc3bf32e913fc`.
Both batch runs used the same 1,024-address list and private seed store through
`decompile_batch` start/status, not individual decompilation calls. Each saved
1,023 successful artifacts and the same failure, with identical payloads,
digests and statuses. Both durable stores passed `quick_check`.

The current `prism --headless <project>` entrypoint runs the normal Worker,
authenticated bridge and MCP stdio path without a GUI. The baseline native
worker was selected through a private executable replacement after host
initialization; both comparison hosts therefore use the current implementation.
Reconstruction used the same completed-batch seed, four 64-unit claims, four
functions per evidence page, and normal source submission. This measures
reconstruction reuse of cached results, not uncached full-program recovery.

Process RSS and physical footprint were sampled every 0.2 seconds, with the
OS lifetime peak also recorded for each child, including the replacement after
the memory-budget restart. Detailed `vmmap` inspection happened after timing.
Single paired runs cannot establish a universal speed or memory guarantee.
Raw measurements remain locally under `/tmp/prism-batch-memory-20261019/`.

## Limits and follow-up

- The batch run still reached roughly 1 GiB RSS in the native worker.
- The results do not prove a one-hour 200,000-function export or an all-purpose
  RAM limit.
- Host-memory comparisons are invalid because the old host lacked headless
  mode.
- Reconstruction source bytes here were existing persisted Prism source; no
  generated source, compiler validation or behavior verification was performed.
- Further work should measure independent checkpoint sections, cold storage,
  larger durable batches and language-specific generation/build validation.

## Validation and package

Completed successfully on the final sources:

- Root and vendored formatter checks.
- Locked all-target/all-feature `cargo check` and strict Clippy, including
  undocumented unsafe blocks, redundant clones and needless collection checks.
- Existing all-target/all-feature application tests: one bridge test passed.
- MCP-only checking and release build with default features disabled.
- Desktop release build; its SHA-256 matches the measured final executable:
  `054f3b6416ba602cd03b89627f0d0f2a5d09cb50bc6ec6b6347eca48b89ab918`.
- All 377 first-party Rust sources satisfy their applicable physical-line limits;
  the review inventory hashes and line counts were refreshed.
- Normal macOS packaging and `codesign --verify --deep --strict build/Prism.app`.
  The bundle is 22.07 MiB logical size. The running GUI was not restarted.

The existing vendor library tests were attempted but cannot compile because
`tests/data/bitfield/listsort_expected.txt` is absent. No tests, replacement
fixtures or test harnesses were authored. OpenUnity's architecture/source-size
scripts are not present in this separate Prism project. macOS was profiled;
Windows and Linux behavior were reviewed but not run.

Independent review covered full changed methods and their immediate ownership,
transport, snapshot and transaction dependencies. The vendor now directly uses
pinned `memmap2` 0.9.11 and `tempfile` 3.23.0, both already present in Prism's
dependency graph. The headless runner reuses the existing Worker and MCP path;
it does not add an always-running service. All private profiling workers exited.
