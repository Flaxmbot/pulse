# Pulse Compatibility Policy

Status: active for `0.1.x` development line.

## Guarantees

- Source compatibility is best-effort inside `0.1.x`, not strict semver-stable yet.
- Diagnostic schema (`code`, `severity`, `span`, `fixes`) is guaranteed to be backward-compatible.
- CLI commands remain backward-compatible unless explicitly deprecated.

## Breaking Change Process

1. Add/update parser, compiler, and diagnostics tests.
2. Update `CHANGELOG.md` with a breaking-change note.
3. Ensure Rust + self-host compiler tracks both pass CI matrix.
4. Update migration guidance in docs/README where needed.

## Track Consistency

- Rust compiler remains release authority.
- Self-host compiler must match language acceptance and diagnostics behavior before stable release.
- Divergence bugs are treated as release blockers.
