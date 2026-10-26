# Project standards

Apply the `rust-review` and `rust-best-practices` skills to the entire project. On the
maintainer's machine their entry points are:

- `/Users/tihan-nico/.agents/skills/rust-review/SKILL.md`
- `/Users/tihan-nico/.agents/skills/rust-best-practices/SKILL.md`

These standards cover every maintained Rust file, including build scripts, binaries,
tests, platform modules, feature-gated modules, and modified vendored sources. Review
non-Rust code and configuration for the same ownership, error handling, resource safety,
and readability principles using their language's idioms. Preserve third-party license
text and provenance. Assets and generated build output are not Rust source.

## Implementation and review

- Read complete modified methods and their immediate dependencies after formatting.
- Keep multi-step operations organized by named responsibilities. A formatter passing
  does not establish structural readability.
- Refactor methods that mix three or more responsibilities, require several progress
  flags, repeat state-transition logic, or hide substantial recovery inside handlers.
- Separate preparation, validation, execution, recovery, and cleanup with clear control
  flow. Extract coherent operations; avoid trivial forwarding helpers.
- Use descriptive domain types in shared APIs, borrowed inputs for inspection, concrete
  public errors with causes, and the narrowest useful visibility.
- Avoid panics on external input and lossy integer conversions at trust boundaries.
- Make resource ownership explicit. Provide fallible close paths; destructors must not
  perform protocol exchanges or wait on unfinished threads.
- Attempt independent cleanup even after a failure. Retain the original error and attach
  recovery failures. Preserve recovery data until commit or recovery succeeds.
- Every unsafe operation needs a local `SAFETY` explanation of its actual invariants.
- Test observable behavior and partial failures. Reproduce a bug before fixing it; avoid
  tests that mirror implementation branches or restate constants.
- Review every changed file against both skills. Group review findings by severity and
  include a rule, source location, consequence, and concrete fix.

## Required verification

Run `python3 scripts/check-rust.py` from the repository root. It checks both maintained
crates with the pinned compiler, including the MCP-only and desktop feature combinations.
The root `Cargo.toml` is the shared lint policy. Keep warnings as errors in tooling,
never as a source-level `deny(warnings)`. Use narrowly scoped `expect` attributes with
reasons when a diagnostic is inappropriate.

The command writes individual logs and a hashed source inventory to
`build/rust-review/coverage.json`. That inventory records automated coverage; do not
present it as proof of manual review or universal correctness. Add new crates and Rust
source roots to the check command before introducing them.

Provider and UI acceptance checks remain separate from deterministic Rust checks. When
changing REA integration, run `scripts/smoke-rea.py` against local applications using an
isolated profile, and record the runtime version and observed results. Do not infer
Windows, Linux, FFI soundness, or interactive behavior from a macOS build alone.
