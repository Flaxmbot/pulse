# Pulse Rust Compiler Review Baseline

This document defines the Rust-first baseline that should be reviewed and green-lit before moving deeper into self-hosting.

## Current Baseline (March 2026)

- Rust workspace compiles and tests cleanly:
  - `cargo test --workspace --all-targets`
- Compiler/VM token handling is aligned for comparison operators (`<`, `>`) and generic type delimiters.
- VM opcode coverage includes DSA-critical operations:
  - arithmetic: `+ - * / %`
  - bitwise: `& | ^ ~`
  - shifts: `<< >>`
  - power: `**`
  - list append opcode support
- CLI integration tests include DSA-focused operator coverage.
- JIT test suite is stabilized to avoid unsafe runtime execution crashes in CI by validating compile paths.

## Minimum Feature Set for LeetCode DSA

- Primitive numerics and booleans.
- Arrays/lists with index read/write.
- Maps/hash maps with key lookup and updates.
- Loops: `while`, `for`.
- Conditionals and boolean logic.
- Functions and recursion.
- String primitives.
- Integer-safe errors (overflow/div by zero/mod by zero surfaced as runtime errors).

## Production Review Gate (Rust Compiler)

Require all items to be green before self-hosting promotion:

1. Reliability
   - `cargo test --workspace --all-targets` passing in CI.
   - No panics/segfaults on supported platforms in standard test runs.
2. Language Correctness
   - Integration tests for arithmetic, control flow, functions, data structures, classes, and DSA operators.
   - Parser/compiler token consistency validated by tests.
3. Security and Runtime Safety
   - Capability checks enforced in VM/runtime where applicable.
   - Path sanitization for imports enabled and tested.
4. Code Health
   - `cargo fmt` clean.
   - `cargo clippy --workspace --all-targets` actionable warnings triaged.
5. Release Artifacts
   - CLI build (`pulse_cli`) reproducible.
   - LLVM backend object/IR artifact generation verified.

## Self-Hosting Progression Rule

Treat the Rust compiler as canonical until all gate criteria are stable for multiple review cycles.  
Only then treat self-hosted compiler output as candidate-trustworthy for parity validation and bootstrap phases.
