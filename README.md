# Pulse Programming Language

<p align="center">
  <img src="https://img.shields.io/badge/version-1.0.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/Rust-1.70+-orange" alt="Rust">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License">
</p>

## Overview

Pulse is a modern, high-performance programming language designed with actor-based concurrency, functional programming features, and a clean syntax inspired by JavaScript, Python, and Rust. It combines the fault-tolerance of the Actor Model with modern, ergonomic syntax and a high-performance Rust runtime.

## Features

### Core Language Features
- **Dynamic Typing** - Flexible type system with runtime type checking
- **First-class Functions** - Functions as values, closures, and higher-order functions
- **Object-Oriented** - Classes with inheritance and encapsulation
- **Pattern Matching** - Powerful destructuring and matching expressions
- **Async/Await** - Built-in support for asynchronous programming
- **Metaprogramming** - Macros and reflection capabilities

### Concurrency Model
- **Actor-based Concurrency** - Built-in support for concurrent programming using the actor model
- **No Shared Memory** - Actors communicate only via message passing
- **Crash-Oriented** - Failure is normal. Supervisors restart crashed actors automatically
- **Location Transparency** - Sending a message to a local actor or remote node looks identical
- **Deterministic Execution** - Stack-based VM ensures predictable execution

### Implementation

The Pulse language implementation includes multiple execution backends:

| Component | Description | Status |
|-----------|-------------|--------|
| **Bytecode VM** | Stack-based interpreter with GC | ✅ Stable |
| **JIT Compiler** | Just-in-time compilation via LLVM | ⚠️ In Development |
| **LLVM AOT** | Ahead-of-time compilation to native code | ⚠️ In Development |
| **Actor Runtime** | Tokio-based async actors | ✅ Stable |

## Architecture

```
Pulse Language Architecture
├── pulse_cli/           # Command-line interface
├── pulse_compiler/      # Lexer, Parser, Bytecode Emitter  
├── pulse_vm/            # Virtual Machine with GC
├── pulse_runtime/       # Actor system and async runtime
├── pulse_stdlib/        # Standard library (IO, Net, JSON, Regex)
├── pulse_llvm_backend/  # JIT and AOT compilation via LLVM
├── pulse_aot_runtime/   # Runtime support for AOT-compiled code
├── pulse_core/          # Core types and utilities
└── pulse_lsp/          # Language Server Protocol implementation
```

## Quick Start

### Prerequisites

- **Rust 1.70+** - Install from https://rustup.rs
- **LLVM 18** - Required for JIT/AOT (optional for interpreter mode)
- **Clang** - For native compilation

### Installation

#### Windows (PowerShell)
```powershell
# Run the installer
.\install.ps1

# For release build
.\install.ps1 -Release
```

#### Linux/macOS (Bash)
```bash
# Make executable
chmod +x install.sh

# Run installer
./install.sh

# For release build
./install.sh --release
```

### Manual Build

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run a program
cargo run -- run examples/hello.pulse

# Start REPL
cargo run -- repl

# Build to native executable
cargo run -- build examples/hello.pulse -o hello.exe
```

## Usage

### Running Programs

```bash
# Run with interpreter (fastest development cycle)
pulse run program.pulse

# Start interactive REPL
pulse repl

# Compile to native executable
pulse build program.pulse -o output.exe
```

### Command-Line Options

```
pulse [OPTIONS] [COMMAND]

Commands:
  run <FILE>      Run a Pulse source file
  repl            Start the interactive REPL  
  build <FILE>    Build to native executable
```

## Language Examples

### Hello World
```pulse
var greeting = "Hello, Pulse!";
println(greeting);
```

### Functions
```pulse
fn factorial(n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

println("5! = " + factorial(5));
```

### Classes
```pulse
class Calculator {
    value;
    
    init() {
        this.value = 0;
    }
    
    fn add(n) {
        this.value = this.value + n;
        return this;
    }
}

var calc = Calculator();
calc.add(10).add(5);
println(calc.result());
```

### Actor Concurrency
```pulse
actor Counter {
    count;
    
    init() {
        this.count = 0;
    }
    
    receive(msg) {
        if (msg == "increment") {
            this.count = this.count + 1;
            return this.count;
        }
        return nil;
    }
}

var counter = Counter();
counter ! "increment";
counter ! "increment";
var result = counter ?? "get";
println("Count: " + result);
```

### Async/Await
```pulse
async fn fetch_data(url) {
    await sleep(100);
    return { "status": 200, "data": "Response" };
}

var result = await fetch_data("https://api.example.com");
println(result);
```

## Standard Library

The Pulse standard library provides:

- **IO** - File operations, console I/O
- **Networking** - TCP/UDP sockets, HTTP client
- **JSON** - Parse and serialize JSON
- **Regex** - Regular expression support
- **String Utilities** - String manipulation
- **Testing** - Built-in test framework

## Development Status

Pulse is currently in **active development**. While the core language and VM are functional, some advanced features are still being developed:

| Feature | Status |
|---------|--------|
| Bytecode VM | ✅ Complete |
| Garbage Collection | ✅ Complete |
| Actor Runtime | ✅ Complete |
| Basic Standard Library | ✅ Complete |
| JIT Compilation | ⚠️ Partial |
| LLVM AOT | ⚠️ Partial |
| Type Checker | 🔄 Planned |
| Self-hosted Compiler | 🔄 Planned |

## Contributing

Contributions are welcome! Please ensure:

1. Code passes `cargo check` without errors
2. Tests pass with `cargo test`
3. Documentation is updated for any new features

## License

MIT License - see LICENSE file for details.

## Acknowledgments

Pulse draws inspiration from many excellent programming languages including JavaScript, Python, Ruby, Rust, Erlang, and others.
