# Memory audit and fresh-process benchmarks

This follow-up covers project history, document save/load, chat migration,
Android inventory/source, engine/debugger communication, MCP calls, and retained
code views. It extends the [performance audit](PERFORMANCE_AUDIT.md). The memory
baseline is PR commit `cc2c4f6638c3c691de70ad5ab3a973859c247f3d`, which already
contains the first seven performance improvements.

## What changed

- **Project history:** imported JSON entries are independently shared. Editing a
  color tag copies the small map/tag entry while unchanged metadata stays shared
  across the existing twenty undo snapshots. Serialization remains transparent;
  tests verify previous tag values, unknown metadata, and round trips.
- **Project saves:** borrowed serialization writes metadata into compression
  without constructing another full project/chat JSON tree or cloning imported
  `project.json`. A paired save test compares every decoded metadata entry and
  the binary against the old writer. JSON key order can differ; stored entry
  hashes are recomputed. Atomic publication and original-file protection remain.
- **Archive reads:** decompression, size enforcement, integrity hashing and JSON
  parsing now stream together. Malformed JSON still triggers complete bounded
  integrity verification. This removes the full decoded-byte buffer, but retains
  the parsed value and parser scratch space for the largest string.
- **Archived chats:** a validation pass checks every typed thread before writes;
  a second pass imports one thread at a time. Memory scales with the largest
  thread instead of the whole archive. Existing per-thread database failure,
  order and retry behavior remain. The second parse is a throughput tradeoff.
- **Android inventory:** the reader keeps only fields consumed by inventory
  construction. Source rows, annotations and unknown values are validated and
  discarded as they are read. Missing/null fields, duplicate keys, malformed
  UTF-8/numbers, recursion limits and file-size limits remain covered.
- **Android source navigation:** only matching class/method content is retained
  while the complete input is validated. Field order, duplicate-key behavior,
  first-match and parent-class precedence, fallback declarations and the existing
  display-limit errors are compared against the legacy reader. Candidate method
  bodies/declarations and parser scratch remain; a large single string can still
  determine the peak. No persistent whole-class cache is introduced.
- **Engine/debugger queues:** raw frames wait in a queue limited by both message
  count and allocated bytes; JSON is decoded by the consumer. The ordinary
  queue budget is 8 MiB, with sixteen slots. A protocol-valid oversized frame
  remains admissible alone. This is a queue bound, not a total-process bound:
  one producer frame, the consumer's current frame/decoded value and transport
  buffers are additional. Existing protocol frame limits still apply. Tests
  cover blocked producers, cancellation, process shutdown and fatal parse errors.
- **MCP/tool previews:** request parameters move into pending work without a deep
  clone, received byte buffers are released earlier, and tool previews append
  only the UTF-8-safe prefix retained by the existing display limit.
- **Code views:** line indexes store one 32-bit start offset instead of two
  native-width offsets for ordinary text (75% less index capacity on this host).
  A native-width fallback handles text above 4 GiB without a new content limit.
  Analysis JSON formatting stops at the existing 1 MiB display limit rather than
  allocating the whole pretty-printed result. Truncated provider strings release
  oversized backing allocations. Tests compare exact output, Unicode boundaries
  and truncation notices.

## Results

Medians of five fresh processes per implementation, Linux x86_64, 2026-10-23.
Peak RSS is in MiB; operation time is in milliseconds. These are independent
workloads, not one aggregate app session.

| Workload | Peak before | Peak after | Reduction | Time before → after |
| --- | ---: | ---: | ---: | ---: |
| 20 color-tag edits/undo states, 12 MiB imported document | 255.17 | 26.87 | 89.5% | 81.948 → 0.047 |
| Save project, 60,000 nested records plus notes/chat | 213.48 | 109.65 | 48.6% | 210.913 → 77.157 |
| Read archive JSON, 24 MiB in 64 KiB strings | 51.84 | 30.33 | 41.5% | 101.744 → 338.205 |
| Import 128 archived chats, 64 MiB total message text | 70.82 | 8.65 | 87.8% | 583.843 → 858.200 |
| Index Android class, 1,000 methods / 80,000 source rows | 90.88 | 5.78 | 93.6% | 98.531 → 134.987 |
| Open last method in that 12 MiB Android class | 89.37 | 3.36 | 96.2% | 117.623 → 143.098 |
| Saturated RPC queue, 32 × 1 MiB dense numeric frames | 278.06 | 25.42 | 90.9% | 818.219 → 712.862 |
| Line index, 1 MiB / approximately 80-byte lines | 3.99 | 3.86 | 3.1% | 0.388 → 0.299 |
| Line index, 1 MiB / two-byte lines | 11.88 | 6.01 | 49.4% | 7.555 → 6.404 |
| Analysis preview, 40,000 records / 1.28 million integers | 114.76 | 98.77 | 13.9% | 95.799 → 25.117 |
| Analysis preview, 1 million integers / 33 array levels | 99.38 | 34.64 | 65.1% | 216.668 → 9.289 |

The large history result measures metadata mutation and retention, excluding
SQLite persistence/UI rendering; it does not claim sub-millisecond end-to-end
annotation latency. Project-save timing includes compression and atomic write.
Archive-read timing excludes fixture archive creation, while its peak RSS
includes that streaming setup. Chat timing includes parsing and database commits.
Android inventory timing includes parsing and function/source inserts, while the
source case reads one method; neither starts jadx. The queue run validates all
32 frames/16,777,216 values and includes framing, parsing and consumption.
Preview timing includes formatting and releasing the input JSON tree, but excludes
its initial parse. Every process's RSS includes that parse.

**CPU tradeoffs:** archive JSON reads take 3.32× as long in this string-heavy
fixture, archived-chat imports 1.47×, Android inventory 1.37×, and Android method
reads 1.22×. Streaming removes retained buffers/trees but uses reader-based JSON
parsing; chats also require a second validation pass to preserve failure behavior.
These reductions prioritize the requested RAM use, and are not claimed as speed
improvements. Project saves are 2.73× faster and bounded previews 3.81×/23.32×
faster in their respective fixtures. Tiny line-index RSS differences are near the
harness floor; the deterministic backing-capacity reduction is 75% (8 to 2 MiB
for the short-line fixture), not a 75% whole-process claim.

Raw measurements and environment/fixture/source/binary hashes:
[memory samples](performance/2026-10-23-memory.json). The recorded HEAD precedes
the follow-up commit; `candidate_src_sha256` identifies the tested Rust sources.

The desktop control used five fresh launches per build/case (thirty total), with
the same helper and software-rendered Xvfb display. Medians in MiB:

| Desktop state | Settled RSS before → after | Peak RSS before → after |
| --- | ---: | ---: |
| Idle Home | 92.29 → 92.04 | 92.29 → 92.04 |
| Local source, roughly 80-byte lines | 97.54 → 96.80 | 100.70 → 100.57 |
| Local source, two-byte lines | 104.75 → 98.17 | 104.92 → 98.33 |

No meaningful idle/ordinary-source improvement is established. The short-line
stress case saves approximately 6.6 MiB (6.3% settled RSS), consistent with the
smaller retained line index. It does not establish savings for a fully analyzed
native project. [Desktop samples and binary identities](performance/2026-10-23-desktop-memory.json)
include every observation; Home/source state and absence of application errors
were checked throughout each run.

## Measurement method

The release profile, VM and Rust toolchain match the performance audit. The
runner generates deterministic external fixtures before measurement, then runs
baseline and candidate in separate fresh processes. Each case uses five samples
per implementation, alternating order, with no concurrent builds or benchmarks.
Baseline algorithms live only in ignored tests; candidate paths call production
code. Results include source, binary, launcher and fixture hashes.

Linux `wait4` reports peak RSS for each workload. A small native launcher forks
the workload after its own `exec`, avoiding the inherited resident-memory floor
of the Python fixture generator. RSS includes the test harness, fixture loading,
setup and correctness checks. Operation timing excludes common setup where the
workload explicitly starts its timer afterward; complete process time is also
recorded. RSS is a process high-water mark, not allocation accounting or retained
memory after the operation. Small differences near the harness floor are noisy.

The undo and saturated-queue cases deliberately exercise accumulation. The
deeply nested analysis view exercises JSON indentation expansion. They are
scaling/stress cases, not claims about a typical project. Project-save output is
checked semantically by paired regression tests, since JSON key order may differ.
Other workloads check their output values/counts, and emit semantic hashes where
applicable. The runner rejects mismatched emitted hashes.

## Reproduce

Linux, Python 3.11+, a C compiler and the pinned Rust toolchain are required:

```sh
python3 scripts/benchmark-memory.py
```

Raw samples, workload logs, metadata and fixture hashes are written to
`target/memory/results.json`; generated fixtures remain under `target/memory/`.
Use `--cases` to select workloads and `--runs` to change the sample count. The
optional `--binary` must identify a current release test executable.

For desktop controls, preserve baseline and candidate release binaries
and start an X server (for example Xvfb). The same MCP helper observes both:

```sh
python3 scripts/benchmark-desktop-memory.py \
  --baseline /path/to/baseline/prism --candidate target/release/prism \
  --helper target/release/prism-mcp --display :99 \
  --output target/memory/desktop.json
```

This uses a new private profile for every launch, software rendering, and thirty
observations at 100 ms intervals after the expected screen/content is ready.
Cases cover Home and two local source files: 1 MiB of roughly 80-byte lines, and
1 MiB of two-byte lines. Settled RSS is the median of the last ten observations.
The configured provider is Rizin so local source opening does not require the
default REA executable fingerprint; these cases never invoke Rizin or any other
analysis engine. It measures the desktop PID; the helper, display server, GPU
memory and external engines are excluded.

## Limits and remaining costs

These changes target first-party allocations across multiple features. Their
percentages cannot be added or applied to whole-app RAM. External REA/Ghidra,
Rizin and jadx subprocess working sets require installed engines and real input
projects. The default-provider file-open check stopped at the missing REA
executable; it is not counted as a native-analysis benchmark. Local source views
were exercised with the configuration above. No macOS/Windows
memory result, GUI frame-rate improvement or engine decompilation speedup is
claimed.

Project persistence still serializes the retained project to SQLite on edits.
Parsed graph/reference data may coexist with raw JSON needed for copy/export.
The safetensors model view reads at most 1 MiB of metadata but pretty-prints the
complete value; deeply nested input can expand substantially. Unlike the generic
analysis view, it has no existing formatted-output cap. Changing that output
requires a defined preview/export policy, so it remains a separate finding.
Expanded large chat messages, sidebar flattening, result paging and snapshot
write-transaction duration remain findings in the performance audit. Existing
bounded chat history, streaming binary/string readers and reconstruction object
limits were reviewed without removing their behavior or raising limits. Settings
named cache budgets mostly bound disk storage; lowering those is not a proven
RAM optimization. A single large JSON string/thread/frame can still dominate its
operation even when aggregate buffering is removed.

## Validation

The following checks passed on the final Rust sources:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo check --locked --no-default-features --bin prism-mcp
cargo build --release --locked --bins
cargo test --release --locked --all-targets --no-run
```

Normal tests: **68 passed**. All eleven memory cases passed five baseline and
five candidate runs (**110 fresh processes**), including semantic/value checks.
The original seven release performance benchmarks also passed after the memory
changes to their shared paths; [rerun output](performance/2026-10-23-performance-followup.txt)
is retained without replacing the initial paired series. Final MCP binaries
passed `initialize` and `tools/list` in combined, window and analysis modes.
The Python drivers compile, and the native RSS launcher builds with
`-Wall -Wextra -Werror`.
