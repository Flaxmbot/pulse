# Pulse Language Specification (Locked Baseline)

Status: `0.1.x pre-stable locked baseline`  
Effective date: `2026-03-03`

This document defines the baseline syntax/semantics that both compiler tracks must implement consistently before a stable `1.0.0` declaration.

## Canonical Front-End

- Lexer tokenization in `pulse_compiler/src/lexer.rs` is normative.
- Parser behavior in `pulse_compiler/src/parser_v2.rs` is normative for syntax acceptance and diagnostic spans.
- Self-hosted parser must remain behavior-compatible with this baseline; any deltas require an explicit compatibility note.

## Baseline Language Surface

- Declarations: `let`, `const`, `fn`/`def`, `class`, `actor`, `import`, shared/atomic declarations.
- Control flow: `if/else`, `while`, `for` (C-style and `for .. in`), `break`, `continue`, `return`.
- Data: scalars, list/map literals, indexing, assignment, closures, method/property access.
- Concurrency: actor spawn/send/receive/link/monitor primitives.
- Pattern matching: `match` with wildcard/variable/literal/range patterns.
- Errors: `try/catch/throw`.

## Diagnostics Contract

- Every compiler/runtime failure must provide:
  - stable error code (`PUL-EXXXX`)
  - severity
  - primary span when source is available
  - optional fix hints
- Text diagnostics are treated as user-facing API and protected by golden tests.

## Compatibility Policy (Pre-1.0)

- `0.1.x` allows breaking changes only when:
  - parser/diagnostic goldens are updated,
  - release notes call out the break explicitly,
  - both Rust and self-host tracks are validated in CI.
- No silent grammar drift: parser acceptance changes require test coverage and changelog entries.

## Stable Gate for 1.0

- Rust + self-host front-end compatibility report green.
- AOT and JIT smoke tests green on Windows + Linux.
- Packaging and installer checks green.
- Diagnostic golden suite green.
- Language spec + compatibility docs updated and approved.
