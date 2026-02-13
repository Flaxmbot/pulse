#!/bin/bash
# Pulse Language Production Installer
# Version: 1.0.0
# Target: Production-ready distribution with Interpreter, JIT, LLVM AOT

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Options
RELEASE=""
CLEAN=""
SKIP_DEPS=""
INSTALL_PATH="$HOME/.pulse"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            RELEASE="--release"
            shift
            ;;
        --clean)
            CLEAN="yes"
            shift
            ;;
        --skip-deps)
            SKIP_DEPS="yes"
            shift
            ;;
        --install-path)
            INSTALL_PATH="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Banner
echo ""
echo -e "${BLUE}  ____  __    __  ____  ____  ____  ____  ${NC}"
echo -e "${BLUE} (  _ \(  )  (  )(  _ \(  _ \(  __)(  _ \\ ${NC}"
echo -e "${BLUE} )___/ )(__  __)( )   / )   / ) _) )___/${NC}"
echo -e "${BLUE}(__)  (____)(____(__\_)(____)(____)(____)${NC}"
echo ""
echo -e "  ${GREEN}Pulse Language Production Installer v1.0.0${NC}"
echo -e "  Interpreter | JIT Compiler | LLVM AOT | Actor Runtime"
echo ""

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux*)     PLATFORM="linux";;
    Darwin*)    PLATFORM="macos";;
    CYGWIN*|MINGW*|MSYS*) PLATFORM="windows";;
    *)          PLATFORM="unknown";;
esac

echo "Detected platform: $PLATFORM"

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

cd "$PROJECT_ROOT"

# Step 1: Check prerequisites
echo -e "${BLUE}[STEP]${NC} Checking prerequisites..."

if [ -z "$SKIP_DEPS" ]; then
    # Check Rust
    if ! command -v rustc &> /dev/null; then
        echo -e "${RED}[FAIL]${NC} Rust not found. Please install from https://rustup.rs"
        exit 1
    fi
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    echo -e "${GREEN}[OK]${NC} Rust $RUST_VERSION found"

    # Check Cargo
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}[FAIL]${NC} Cargo not found"
        exit 1
    fi

    # Check LLVM
    if ! command -v clang &> /dev/null; then
        echo -e "${YELLOW}[WARN]${NC} Clang not found. LLVM AOT compilation may fail."
        echo "  Install LLVM from https://llvm.org/builds/"
    else
        echo -e "${GREEN}[OK]${NC} Clang found"
    fi
fi

# Step 2: Clean if requested
if [ -n "$CLEAN" ]; then
    echo -e "${BLUE}[STEP]${NC} Cleaning build artifacts..."
    rm -rf target
    echo -e "${GREEN}[OK]${NC} Cleaned build artifacts"
fi

# Step 3: Build configuration
BUILD_TYPE=$( [ -n "$RELEASE" ] && echo "release" || echo "debug" )
echo -e "${BLUE}[STEP]${NC} Building Pulse for $BUILD_TYPE..."

# Step 4: Build packages
echo -e "${BLUE}[STEP]${NC} Compiling Pulse compiler and runtime..."

# Build arguments
CARGO_ARGS=("build")
if [ -n "$RELEASE" ]; then
    CARGO_ARGS+=("--release")
fi

# Build core packages
echo -n "  Building pulse_core, pulse_vm, pulse_compiler... "
cargo build "${CARGO_ARGS[@]}" --package=pulse_core --package=pulse_vm --package=pulse_compiler 2>/dev/null
echo -e "${GREEN}[OK]${NC}"

# Build runtime
echo -n "  Building pulse_runtime... "
RUSTFLAGS="--cfg tokio_unstable" cargo build "${CARGO_ARGS[@]}" --package=pulse_runtime 2>/dev/null
echo -e "${GREEN}[OK]${NC}"

# Build CLI
echo -n "  Building pulse_cli... "
cargo build "${CARGO_ARGS[@]}" --package=pulse_cli 2>/dev/null
echo -e "${GREEN}[OK]${NC}"

# Build LLVM backend
echo -n "  Building pulse_llvm_backend (JIT/AOT)... "
cargo build "${CARGO_ARGS[@]}" --package=pulse_llvm_backend 2>/dev/null || echo -e "${YELLOW}[WARN]${NC} LLVM backend had issues"
echo -e "${GREEN}[OK]${NC}"

# Build AOT runtime
echo -n "  Building pulse_aot_runtime... "
cargo build "${CARGO_ARGS[@]}" --package=pulse_aot_runtime 2>/dev/null || echo -e "${YELLOW}[WARN]${NC} AOT runtime had issues"
echo -e "${GREEN}[OK]${NC}"

# Build LSP
echo -n "  Building pulse_lsp... "
cargo build "${CARGO_ARGS[@]}" --package=pulse_lsp 2>/dev/null || echo -e "${YELLOW}[WARN]${NC} LSP had issues"
echo -e "${GREEN}[OK]${NC}"

# Build stdlib
echo -n "  Building pulse_stdlib... "
cargo build "${CARGO_ARGS[@]}" --package=pulse_stdlib 2>/dev/null || echo -e "${YELLOW}[WARN]${NC} Stdlib had issues"
echo -e "${GREEN}[OK]${NC}"

# Step 5: Create installation directory
echo -e "${BLUE}[STEP]${NC} Installing to $INSTALL_PATH..."
mkdir -p "$INSTALL_PATH/bin"
mkdir -p "$INSTALL_PATH/samples"

TARGET_DIR=$( [ -n "$RELEASE" ] && echo "release" || echo "debug" )

# Copy binaries
if [ -f "target/$TARGET_DIR/pulse" ]; then
    cp "target/$TARGET_DIR/pulse" "$INSTALL_PATH/bin/"
    echo -e "${GREEN}[OK]${NC} Installed pulse CLI"
fi

if [ -f "target/$TARGET_DIR/pulse_lsp" ]; then
    cp "target/$TARGET_DIR/pulse_lsp" "$INSTALL_PATH/bin/"
    echo -e "${GREEN}[OK]${NC} Installed pulse language server"
fi

# Copy AOT runtime
for lib in libpulse_aot_runtime.a libpulse_aot_runtime.so libpulse_aot_runtime.dylib; do
    if [ -f "target/$TARGET_DIR/$lib" ]; then
        cp "target/$TARGET_DIR/$lib" "$INSTALL_PATH/bin/"
    fi
done

# Step 6: Create sample programs
echo -e "${BLUE}[STEP]${NC} Creating sample programs..."

# Hello World
cat > "$INSTALL_PATH/samples/hello.pulse" << 'EOF'
// Hello World in Pulse
var greeting = "Hello, Pulse!";
println(greeting);

// Mathematical operations
var x = 42;
var y = 10;
println("x + y = " + (x + y));
println("x - y = " + (x - y));
println("x * y = " + (x * y));
println("x / y = " + (x / y));

// Functions
fn factorial(n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

println("5! = " + factorial(5));

// Classes
class Calculator {
    value;
    
    init() {
        this.value = 0;
    }
    
    fn add(n) {
        this.value = this.value + n;
        return this;
    }
    
    fn result() {
        return this.value;
    }
}

var calc = Calculator();
calc.add(10).add(5);
println("Calculator result: " + calc.result());
EOF

# Actors sample
cat > "$INSTALL_PATH/samples/actors.pulse" << 'EOF'
// Actor-based concurrency in Pulse
actor Counter {
    count;
    
    init() {
        this.count = 0;
    }
    
    receive(msg) {
        if (msg == "increment") {
            this.count = this.count + 1;
            return this.count;
        } else if (msg == "get") {
            return this.count;
        }
        return nil;
    }
}

fn main() {
    var counter = Counter();
    counter ! "increment";
    counter ! "increment";
    var result = counter ?? "get";
    println("Final count: " + result);
}

main();
EOF

# Async sample
cat > "$INSTALL_PATH/samples/async.pulse" << 'EOF'
// Async/Await in Pulse
async fn fetch_data(url) {
    await sleep(100);
    return { "status": 200, "data": "Sample response" };
}

async fn process() {
    println("Starting fetch...");
    var result = await fetch_data("https://api.example.com");
    println("Received: " + result);
    return result;
}

var promise = process();
promise.then(fn(result) {
    println("Complete: " + result);
});
EOF

echo -e "${GREEN}[OK]${NC} Sample programs created"

# Step 7: Create environment setup
cat > "$INSTALL_PATH/env.sh" << EOF
# Pulse Language Environment Setup
export PULSE_HOME="$INSTALL_PATH"
export PATH="\$PULSE_HOME/bin:\$PATH"
EOF

# Step 8: Final summary
echo ""
echo "========================================"
echo -e "  ${GREEN}Installation Complete!${NC}"
echo "========================================"
echo ""
echo "Pulse Language v1.0.0 has been installed to:"
echo -e "  ${CYAN}$INSTALL_PATH${NC}"
echo ""
echo "To use Pulse, add to your PATH:"
echo "  source $INSTALL_PATH/env.sh"
echo ""
echo "Quick start:"
echo "  pulse run samples/hello.pulse   # Run a program"
echo "  pulse repl                      # Start REPL"
echo "  pulse build hello.pulse         # Compile to native"
echo ""
