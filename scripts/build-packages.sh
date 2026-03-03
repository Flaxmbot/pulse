#!/usr/bin/env bash
set -euo pipefail

TRACK="rust"
VERSION="0.1.2"
OUT_DIR="dist/unix"

usage() {
  cat <<'EOF'
Usage: scripts/build-packages.sh [options]
  --track <rust|selfhost>   Package track
  --version <x.y.z>         Package version
  --out-dir <dir>           Output directory
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --track) TRACK="${2:-}"; shift 2 ;;
    --version) VERSION="${2:-}"; shift 2 ;;
    --out-dir) OUT_DIR="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

if [[ "${TRACK}" != "rust" && "${TRACK}" != "selfhost" ]]; then
  echo "Invalid track: ${TRACK}" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
PKG_NAME="pulse-${TRACK}-${VERSION}-${OS}-${ARCH}"
STAGE="dist/staging/${PKG_NAME}"

mkdir -p "${STAGE}/bin" "${STAGE}/lib" "${STAGE}/share/pulse" "${OUT_DIR}"

echo "[STEP] Building release binaries..."
cargo build --release -p pulse_cli -p pulse_lsp -p pulse_aot_runtime

cp "target/release/pulse_cli" "${STAGE}/bin/pulse_cli"
if [[ -f "target/release/pulse-lsp" ]]; then
  cp "target/release/pulse-lsp" "${STAGE}/bin/pulse-lsp"
fi
for lib in libpulse_aot_runtime.a libpulse_aot_runtime.so libpulse_aot_runtime.dylib; do
  if [[ -f "target/release/${lib}" ]]; then
    cp "target/release/${lib}" "${STAGE}/lib/${lib}"
  fi
done

cp -R "self-hosted" "${STAGE}/share/pulse/self-hosted"

cat > "${STAGE}/bin/pulse-rust" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export PULSE_COMPILER_TRACK=rust
export PULSE_HOME="$(cd "$(dirname "$0")/.." && pwd)"
exec "$(dirname "$0")/pulse_cli" "$@"
EOF

cat > "${STAGE}/bin/pulse-selfhost" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export PULSE_COMPILER_TRACK=selfhost
export PULSE_HOME="$(cd "$(dirname "$0")/.." && pwd)"
export PULSE_SELFHOST_ENTRY="${PULSE_HOME}/share/pulse/self-hosted/compiler.pulse"
exec "$(dirname "$0")/pulse_cli" "$@"
EOF

if [[ "${TRACK}" == "selfhost" ]]; then
  cat > "${STAGE}/bin/pulse" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec "$(dirname "$0")/pulse-selfhost" "$@"
EOF
else
  cat > "${STAGE}/bin/pulse" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec "$(dirname "$0")/pulse-rust" "$@"
EOF
fi

chmod +x "${STAGE}/bin/pulse" "${STAGE}/bin/pulse-rust" "${STAGE}/bin/pulse-selfhost"

cat > "${STAGE}/share/pulse/env.sh" <<EOF
export PULSE_HOME="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="\${PULSE_HOME}/bin:\${PATH}"
export LD_LIBRARY_PATH="\${PULSE_HOME}/lib:\${LD_LIBRARY_PATH:-}"
export DYLD_LIBRARY_PATH="\${PULSE_HOME}/lib:\${DYLD_LIBRARY_PATH:-}"
export PULSE_COMPILER_TRACK="${TRACK}"
EOF

if [[ -d "vscode-pulse" ]]; then
  pushd "vscode-pulse" >/dev/null
  npm install
  npm run compile
  npm run package
  VSIX="$(ls -1 *.vsix | tail -n1)"
  cp "${VSIX}" "../${STAGE}/share/pulse/${VSIX}"
  popd >/dev/null
fi

tar -czf "${OUT_DIR}/${PKG_NAME}.tar.gz" -C "dist/staging" "${PKG_NAME}"
echo "[OK] Package created: ${OUT_DIR}/${PKG_NAME}.tar.gz"

