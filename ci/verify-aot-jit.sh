#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "[STEP] Running LLVM backend tests (AOT + JIT)..."
cargo test -p pulse_llvm_backend --test e2e_aot_test --test jit_test

echo "[STEP] Running CLI AOT build smoke..."
cargo run --bin pulse_cli -- build examples/hello_aot.pulse

echo "[OK] AOT/JIT verification passed."
