$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

Write-Host "[STEP] Running LLVM backend tests (AOT + JIT)..."
cargo test -p pulse_llvm_backend --test e2e_aot_test --test jit_test

Write-Host "[STEP] Running CLI AOT build smoke..."
cargo run --bin pulse_cli -- build examples/hello_aot.pulse

Write-Host "[OK] AOT/JIT verification passed."
