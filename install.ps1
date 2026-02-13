# Pulse Language Production Installer
# Version: 1.0.0
# Target: Production-ready distribution with Interpreter, JIT, LLVM AOT

param(
    [switch]$Release,
    [switch]$Clean,
    [switch]$SkipDeps,
    [string]$InstallPath = "$env:USERPROFILE\.pulse"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Colors for output
function Write-Step { param([string]$msg) Write-Host "[STEP] $msg" -ForegroundColor Cyan }
function Write-Success { param([string]$msg) Write-Host "[OK]   $msg" -ForegroundColor Green }
function Write-Warn { param([string]$msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Fail { param([string]$msg) Write-Host "[FAIL] $msg" -ForegroundColor Red }

# Banner
Write-Host ""
Write-Host "  ____  __    __  ____  ____  ____  ____  " -ForegroundColor Blue
Write-Host " (  _ \(  )  (  )(  _ \(  _ \(  __)(  _ \" -ForegroundColor Blue
Write-Host " )___/ )(__  __)( )   / )   / ) _) )___/" -ForegroundColor Blue
Write-Host "(__)  (____)(____(__\_)(____)(____)(____)" -ForegroundColor Blue
Write-Host ""
Write-Host "  Pulse Language Production Installer v1.0.0" -ForegroundColor White
Write-Host "  Interpreter | JIT Compiler | LLVM AOT | Actor Runtime" -ForegroundColor Gray
Write-Host ""

# Detect environment
$IsWindows = $PSVersionTable.Platform -eq "Win32NT" -or $null -eq $PSVersionTable.Platform
$IsMacOS = $PSVersionTable.Platform -eq "Darwin"
$IsLinux = $PSVersionTable.Platform -eq "Unix" -and -not $IsMacOS

# Get script directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = $ScriptDir

# Change to project root
Set-Location $ProjectRoot

# Step 1: Check prerequisites
Write-Step "Checking prerequisites..."

if (-not $SkipDeps) {
    # Check Rust
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    if (-not $rustc) {
        Write-Fail "Rust not found. Please install from https://rustup.rs"
        Write-Host "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" -ForegroundColor Gray
        exit 1
    }
    $rustVersion = (rustc --version).Split(" ")[1]
    Write-Success "Rust $rustVersion found"

    # Check LLVM (required for AOT compilation)
    if (-not (Get-Command clang -ErrorAction SilentlyContinue)) {
        Write-Warn "Clang not found. LLVM AOT compilation may fail."
        Write-Host "  Install LLVM from https://llvm.org/builds/" -ForegroundColor Gray
    } else {
        Write-Success "Clang found"
    }

    # Check CMake for LLVM builds
    $cmake = Get-Command cmake -ErrorAction SilentlyContinue
    if (-not $cmake) {
        Write-Warn "CMake not found - some LLVM features may be limited"
    }
}

# Step 2: Clean if requested
if ($Clean) {
    Write-Step "Cleaning build artifacts..."
    if (Test-Path "target") {
        Remove-Item -Recurse -Force "target"
    }
    Write-Success "Cleaned build artifacts"
}

# Step 3: Build configuration
$BuildType = if ($Release) { "release" } else { "debug" }
Write-Step "Building Pulse for $BuildType..."

# Step 4: Build the project
Write-Step "Compiling Pulse compiler and runtime..."

# First, build the core dependencies
Write-Host "  Building pulse_core..." -NoNewline
$env:LLVM_SYS_180_PREFIX = ""  # Use system LLVM if available
$cargoArgs = @("build")
if ($Release) { $cargoArgs += "--release" }
$cargoArgs += "--package=pulse_core", "--package=pulse_vm", "--package=pulse_compiler"

$pulseCoreResult = Start-Process -FilePath "cargo" -ArgumentList $cargoArgs -NoNewWindow -Wait -PassThru
if ($pulseCoreResult.ExitCode -ne 0) {
    Write-Fail "Failed to build core packages"
    exit 1
}
Write-Success "Core packages built"

# Build runtime
Write-Host "  Building pulse_runtime..." -NoNewline
$env:CARGO_BUILD_RUSTFLAGS = "--cfg tokio_unstable"
$runtimeResult = Start-Process -FilePath "cargo" -ArgumentList ($cargoArgs + "--package=pulse_runtime") -NoNewWindow -Wait -PassThru
if ($runtimeResult.ExitCode -ne 0) {
    Write-Fail "Failed to build runtime"
    exit 1
}
Write-Success "Runtime built"

# Build CLI
Write-Host "  Building pulse_cli..." -NoNewline
$cliResult = Start-Process -FilePath "cargo" -ArgumentList ($cargoArgs + "--package=pulse_cli") -NoNewWindow -Wait -PassThru
if ($cliResult.ExitCode -ne 0) {
    Write-Fail "Failed to build CLI"
    exit 1
}
Write-Success "CLI built"

# Build LLVM backend (JIT + AOT)
Write-Host "  Building pulse_llvm_backend..." -NoNewline
$llvmResult = Start-Process -FilePath "cargo" -ArgumentList ($cargoArgs + "--package=pulse_llvm_backend") -NoNewWindow -Wait -PassThru
if ($llvmResult.ExitCode -ne 0) {
    Write-Warn "LLVM backend build had issues - JIT/AOT may be limited"
} else {
    Write-Success "LLVM backend built"
}

# Build AOT runtime
Write-Host "  Building pulse_aot_runtime..." -NoNewline
$aotResult = Start-Process -FilePath "cargo" -ArgumentList ($cargoArgs + "--package=pulse_aot_runtime") -NoNewWindow -Wait -PassThru
if ($aotResult.ExitCode -ne 0) {
    Write-Warn "AOT runtime build had issues"
} else {
    Write-Success "AOT runtime built"
}

# Build LSP
Write-Host "  Building pulse_lsp..." -NoNewline
$lspResult = Start-Process -FilePath "cargo" -ArgumentList ($cargoArgs + "--package=pulse_lsp") -NoNewWindow -Wait -PassThru
if ($lspResult.ExitCode -ne 0) {
    Write-Warn "LSP build had issues - IDE support may be limited"
} else {
    Write-Success "LSP built"
}

# Build stdlib
Write-Host "  Building pulse_stdlib..." -NoNewline
$stdlibResult = Start-Process -FilePath "cargo" -ArgumentList ($cargoArgs + "--package=pulse_stdlib") -NoNewWindow -Wait -PassThru
if ($stdlibResult.ExitCode -ne 0) {
    Write-Warn "Standard library build had issues"
} else {
    Write-Success "Standard library built"
}

# Step 5: Create installation directory
Write-Step "Installing to $InstallPath..."
if (-not (Test-Path $InstallPath)) {
    New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
}

# Copy executables
$BinPath = Join-Path $InstallPath "bin"
if (-not (Test-Path $BinPath)) {
    New-Item -ItemType Directory -Path $BinPath -Force | Out-Null
}

$TargetDir = if ($Release) { "release" } else { "debug" }

# Copy CLI binary
$cliSource = Join-Path $ProjectRoot "target\$TargetDir\pulse.exe"
if (Test-Path $cliSource) {
    Copy-Item $cliSource -Destination $BinPath -Force
    Write-Success "Installed pulse CLI"
}

# Copy LSP binary
$lspSource = Join-Path $ProjectRoot "target\$TargetDir\pulse_lsp.exe"
if (Test-Path $lspSource) {
    Copy-Item $lspSource -Destination $BinPath -Force
    Write-Success "Installed pulse language server"
}

# Copy AOT runtime library
$libSource = Join-Path $ProjectRoot "target\$TargetDir"
$aotLibs = @("libpulse_aot_runtime.a", "libpulse_aot_runtime.dll", "libpulse_aot_runtime.lib")
foreach ($lib in $aotLibs) {
    $libPath = Join-Path $libSource $lib
    if (Test-Path $libPath) {
        Copy-Item $libPath -Destination $BinPath -Force
    }
}

# Step 6: Create environment setup
Write-Step "Creating environment configuration..."

$EnvSetup = @"
# Pulse Language Environment Setup
# Add to your shell profile (`.bashrc`, `.zshrc`, etc.)

export PULSE_HOME="$InstallPath"
export PATH="`$PULSE_HOME/bin:`$PATH"

# For Windows, run this in PowerShell:
# `$env:PATH = "$BinPath;`$env:PATH"
# `$env:PULSE_HOME = "$InstallPath"
"@

$EnvSetupPath = Join-Path $InstallPath "env.sh"
$EnvSetup | Out-File -FilePath $EnvSetupPath -Encoding UTF8

# Step 7: Create sample programs
Write-Step "Creating sample programs..."
$SamplesPath = Join-Path $InstallPath "samples"
if (-not (Test-Path $SamplesPath)) {
    New-Item -ItemType Directory -Path $SamplesPath -Force | Out-Null
}

# Hello World
@"
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
    
    fn subtract(n) {
        this.value = this.value - n;
        return this;
    }
    
    fn result() {
        return this.value;
    }
}

var calc = Calculator();
calc.add(10).subtract(3);
println("Calculator result: " + calc.result());

// Lists
var numbers = [1, 2, 3, 4, 5];
println("Sum of numbers: " + numbers);
"@ | Out-File -FilePath (Join-Path $SamplesPath "hello.pulse") -Encoding UTF8

# Actor example
@"
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
    
    // Spawn actors
    counter ! "increment";
    counter ! "increment";
    counter ! "increment";
    
    var result = counter ?? "get";
    println("Final count: " + result);
}

main();
"@ | Out-File -FilePath (Join-Path $SamplesPath "actors.pulse") -Encoding UTF8

# Async example
@"
// Async/Await in Pulse
async fn fetch_data(url) {
    // Simulate network request
    await sleep(100);
    return { "status": 200, "data": "Sample response" };
}

async fn process() {
    println("Starting fetch...");
    var result = await fetch_data("https://api.example.com");
    println("Received: " + result);
    return result;
}

// Run async
var promise = process();
promise.then(fn(result) {
    println("Complete: " + result);
});
"@ | Out-File -FilePath (Join-Path $SamplesPath "async.pulse") -Encoding UTF8

Write-Success "Sample programs created"

# Step 8: Final summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Installation Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Pulse Language v1.0.0 has been installed to:" -ForegroundColor White
Write-Host "  $InstallPath" -ForegroundColor Cyan
Write-Host ""
Write-Host "To use Pulse, add to your PATH:" -ForegroundColor White

if ($IsWindows) {
    Write-Host "  `$env:PATH = `"$BinPath;`$env:PATH`"" -ForegroundColor Gray
} else {
    Write-Host "  export PATH=`"$BinPath:`$PATH`"" -ForegroundColor Gray
}
Write-Host ""
Write-Host "Quick start:" -ForegroundColor White
Write-Host "  pulse run samples/hello.pulse   # Run a program" -ForegroundColor Gray
Write-Host "  pulse repl                      # Start REPL" -ForegroundColor Gray
Write-Host "  pulse build hello.pulse        # Compile to native" -ForegroundColor Gray
Write-Host ""
Write-Host "Documentation:" -ForegroundColor White
Write-Host "  $InstallPath\README.md" -ForegroundColor Gray
Write-Host ""
