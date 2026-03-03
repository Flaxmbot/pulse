# Changelog

All notable changes to Pulse are documented in this file.

## [0.1.2] - 2026-03-03

### Added
- `examples/comprehensive.pulse`: broad end-to-end language coverage example (variables, functions, recursion, loops, classes, linked list/tree patterns, actors, matching, error handling, collections, input/sync keyword paths).
- `examples/comprehensive_super.pulse`: expanded single-file coverage corpus with additional bitwise/control-flow/collection/OOP/actor/shared-memory grammar paths.
- `tests/bench_baseline.pulse`: deterministic baseline benchmark workload for perf gating.
- Golden diagnostics suite in `pulse_compiler/tests/diagnostics_golden_tests.rs` with text/span snapshots.
- Structured diagnostics schema (`code`, `severity`, `span`, `fixes`) in `pulse_core`.
- CI packaging gates for VS Code extension metadata warnings and packaging smoke jobs.
- Release workflow checksum generation and optional cosign verification hook.
- Selfhost parity harness:
  - `pulse_cli/tests/selfhost_parity_tests.rs` (Rust/selfhost-track differential diagnostics corpus)
- Language baseline docs:
  - `docs/LANGUAGE_SPEC.md`
  - `docs/COMPATIBILITY_POLICY.md`

### Changed
- Enforced static type checking in default `compile()` pipeline before bytecode emission.
- Extended `ParserV2` coverage for parity-critical constructs:
  - `const`, `for` (C-style + `for .. in`), `break`, `continue`
  - `actor`, `shared memory`, `atomic`, `lock`/`unlock`, `fence`/`acquire`/`release`
  - multi-argument `print(...)`, bitwise/mod/power/shift operators, spawn block expressions
  - local function handling inside blocks (closure lowering for type pass)
- Parser/bytecode front drift reductions:
  - compiler parser now accepts `import "path" as alias;` declaration form
  - compiler `let/const` declarations now accept optional type annotations (`let x: Int = ...`)
  - `ParserV2` pre-pass now accepts import expressions used by legacy sources
- Improved type checker behavior for production paths:
  - function/class symbol resolution in variable inference
  - constructor-call handling for class names
  - catch variable scope binding
  - string concatenation support in `+`
  - widened heterogeneous map inference to `Any` where needed
- AOT output reliability fixes:
  - globals now resolved by symbol name (not raw constant index)
  - tag-dispatched print path for int/float/bool/object
  - newline emission for `print`
  - runtime `pulse_print_cstr` support
- AOT backend container/shared-memory semantics are no longer pure stack-effect stubs:
  - metadata-backed list/map/shared-memory handling for `BuildList`, `BuildMap`, `GetIndex`, `SetIndex`, `Len`, `IsList`, `IsMap`, `Slice`, `MapContainsKey`, `ToString`, and shared memory ops.
- Benchmark command now supports a regression gate:
  - `pulse benchmark --max-avg-ms <N>`
  - `PULSE_BENCH_MAX_AVG_MS` environment variable
- CI adds a dedicated performance gate job.
- Release workflow supports required signing mode (`require_signing`) with cosign signing + verification and signature artifact upload.
- Runtime diagnostics now include actionable fixes for bounds, arity, actor lookup, mailbox saturation, and oversized messages.
- VS Code extension packaging quality improvements:
  - added `repository`, `license`, `homepage`, `bugs` metadata
  - added `.vscodeignore`
  - bundled extension `LICENSE`
- Version bumps to `0.1.2` across workspace crates, CLI, packaging scripts, and VS Code extension.

### Fixed
- Native executable issue where `pulse build` output binaries could run without visible output.
- Diagnostics JSON now emits structured fields instead of flat ad-hoc entries.

### Validation
- `cargo test -p pulse_compiler --tests`
- `cargo test -p pulse_cli --test integration_tests`
- `cargo run --bin pulse_cli -- build examples/hello_aot.pulse --output target\\debug\\hello_aot_test`
- `target\\debug\\hello_aot_test.exe` prints expected output.

### MSI
- Updated MSI build defaults to `0.1.2` for both tracks.
- Artifacts expected:
  - `dist/windows/pulse-rust-0.1.2.msi`
  - `dist/windows/pulse-selfhost-0.1.2.msi`
