# Pulse - A Modern Actor-Model Programming Language

<p align="center">
  <img src="https://img.shields.io/badge/Pulse-Language-14b8a6?style=for-the-badge" alt="Pulse Language">
  <img src="https://img.shields.io/badge/Version-0.1.2-blue?style=for-the-badge" alt="Version">
  <img src="https://img.shields.io/badge/Rust-2021-edditable?style=for-the-badge" alt="Rust Edition">
  <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" alt="License">
</p>

---

## Overview

Pulse is a modern, high-performance programming language designed from the ground up to solve the challenges of distributed systems and massive concurrency. Built on a blazing-fast Rust runtime, Pulse combines the fault-tolerance principles of the **Actor Model** (inspired by Erlang/Elixir) with a clean, ergonomic syntax influenced by JavaScript, Python, and Rust.

### What is Pulse?

Pulse is a programming language that natively supports concurrent, non-blocking execution using ultra-lightweight actors. Unlike traditional threading models, Pulse actors communicate purely through asynchronous message passingâ€”eliminating the need for mutexes, locks, and shared memory complications.

### Key Differentiators

| Feature | Traditional Languages | Pulse |
|---------|----------------------|-------|
| Concurrency Model | Threads + Locks | Actor Model |
| Memory Safety | Manual/GC | Ownership + Actor Isolation |
| Distributed Computing | Libraries/Frameworks | Native Clustering Support |
| Compilation | Single Backend | VM + LLVM AOT (WIP) |
| Standard Library | Basic | I/O + JSON + HTTP + Math + Testing |

### Target Use Cases

- **Distributed Systems**: Build scalable microservices with native actor clustering
- **High-Concurrency Applications**: Handle thousands of simultaneous connections
- **Real-time Systems**: Gaming servers, chat applications, IoT platforms
- **Competitive Programming**: DSA-ready stdlib with sort, search, math, and I/O

---

## Features

### Actor Model Concurrency

Pulse implements a true actor model where every actor is an isolated execution unit with its own state. Actors communicate exclusively through asynchronous message passing, preventing race conditions and deadlocks at the architecture level.

```pulse
let worker = spawn {
    let state = [];
    while (true) {
        let msg = receive;
        push(state, msg);
        print("Processed: ", msg);
    }
};

send worker, "task1";
send worker, "task2";
send worker, "task3";
```

### Performance Metrics

Pulse is engineered for high-throughput scenarios:

| Metric | Value |
|--------|-------|
| Message Throughput | ~313,000 messages/sec |
| Actor Spawn Rate | 100,000 actors in ~11.4s |
| AOT Compilation | LLVM backend for native binaries |

### Self-Hosted Compiler

Pulse includes a self-hosted compiler track written in Pulse itself. This is
currently an active bootstrap effort and is validated through `pulse selfhost`
smoke tests while parity with the Rust compiler is being completed.

```
self-hosted/
â”œâ”€â”€ lexer.pulse         # Tokenizer implementation
â”œâ”€â”€ parser.pulse        # AST parser
â”œâ”€â”€ type_checker.pulse  # Type system
â”œâ”€â”€ compiler.pulse      # Code generation
â””â”€â”€ bytecode_gen.pulse # Bytecode emitter
```

### Standard Library

The Pulse standard library (`pulse_stdlib`) provides the following modules:

| Category | Modules |
|----------|---------|
| **I/O & Data** | `io`, `json`, `fs` |
| **Web** | `http` |
| **Text** | `string_utils` |
| **System** | `time` |
| **Math & Utils** | `utils` (math, collections, type conversion, sorting) |
| **Testing** | `testing` (assert, assert_eq, assert_ne) |

> **Note:** Additional modules (networking, database, regex, statistics, etc.) are planned for v0.1.2+.

### LSP and VS Code Support

Basic IDE integration:

- **Language Server Protocol (LSP)**: `pulse_lsp` provides:
  - Lexer-level diagnostics
  - Text document synchronization

- **VS Code Extension**: `vscode-pulse` offers:
  - Syntax highlighting for `.pulse` files
  - Language configuration

> **Note:** Auto-completion, go-to-definition, and full parse-error diagnostics are planned.

---

## Quick Start

### Installation

#### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/pulse-lang/pulse.git
cd pulse/pulse_lang

# Build the CLI
cargo build --release

# Add to your PATH
export PATH="$PATH:$(pwd)/target/release"
```

#### Using Install Scripts

**Linux/macOS:**
```bash
# Rust-first installer
bash install-rust.sh --with-vscode

# Self-hosted track installer
bash install-selfhost.sh --with-vscode
```

**Windows:**
```powershell
# Rust-first installer
powershell -ExecutionPolicy Bypass -File install-rust.ps1 -WithVSCode

# Self-hosted track installer
powershell -ExecutionPolicy Bypass -File install-selfhost.ps1 -WithVSCode
```

**Windows MSI build (two installers):**
```powershell
.\scripts\build-msi.ps1 -Track rust -Version 0.1.2
.\scripts\build-msi.ps1 -Track selfhost -Version 0.1.2
```
This MSI flow uses the WiX SDK via `dotnet build` (no `candle`/`light` required).

**Linux/macOS package build:**
```bash
./scripts/build-packages.sh --track rust --version 0.1.2
./scripts/build-packages.sh --track selfhost --version 0.1.2
```

### Hello World

Create a file named `main.pulse`:

```pulse
// main.pulse - Hello World example
print("Hello from Pulse!");
print("Actor Model programming is awesome!");

// Spawn a simple actor
let greeter = spawn {
    let name = receive;
    print("Hello, " + name + "!");
};

send greeter, "World";
sleep(100);
```

Run it:

```bash
pulse run main.pulse
```

### Basic Commands

| Command | Description |
|---------|-------------|
| `pulse run <file>` | Execute a Pulse source file |
| `pulse check [path]` | Parse + typecheck files without executing |
| `pulse repl` | Start the interactive REPL |
| `pulse test [path]` | Run tests in a file or directory |
| `pulse benchmark [file]` | Run performance benchmarks |
| `pulse build <file> --emit <bc|ir|obj|exe> [--link=true|false]` | Compile artifacts and optionally link native executable |
| `pulse init [name]` | Initialize a new Pulse project |
| `pulse add <package>` | Add a dependency to the project |
| `pulse doc` | Generate documentation |
| `pulse selfhost test|bootstrap` | Self-hosted compiler smoke workflows |

Compiler track selection:
- `PULSE_COMPILER_TRACK=rust pulse run app.pulse`
- `PULSE_COMPILER_TRACK=selfhost pulse run app.pulse`
- Installers `install-rust.*` and `install-selfhost.*` set this by default via wrappers.

---

## Architecture

### Component Overview

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                        Pulse Language                          â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”‚
â”‚  â”‚   CLI        â”‚  â”‚   LSP        â”‚  â”‚   VS Code Extension  â”‚ â”‚
â”‚  â”‚  (Commands)  â”‚  â”‚  ( IDE )     â”‚  â”‚   (Editor Support)   â”‚ â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚
â”‚  â”‚                    Compiler Pipeline                      â”‚  â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”‚  â”‚
â”‚  â”‚  â”‚ Lexer  â”‚ â†’ â”‚ Parser  â”‚ â†’ â”‚Type Checkerâ”‚ â†’ â”‚Compiler â”‚ â”‚  â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â”‚  â”‚
â”‚  â”‚                                              â†“            â”‚  â”‚
â”‚  â”‚                   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”        â”‚  â”‚
â”‚  â”‚                   â”‚  Bytecode / LLVM IR Output  â”‚        â”‚  â”‚
â”‚  â”‚                   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜        â”‚  â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”‚
â”‚  â”‚   Pulse VM   â”‚  â”‚ LLVM Backend â”‚  â”‚   AOT Runtime        â”‚ â”‚
â”‚  â”‚ (Bytecode)   â”‚  â”‚   (Native)   â”‚  â”‚   (Compiled)         â”‚ â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚
â”‚  â”‚                     Runtime System                        â”‚  â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚  â”‚
â”‚  â”‚  â”‚ Actor   â”‚  â”‚ Mailbox   â”‚  â”‚Cluster  â”‚  â”‚  Network  â”‚  â”‚  â”‚
â”‚  â”‚  â”‚ System  â”‚  â”‚ Messaging â”‚  â”‚ Support â”‚  â”‚   I/O     â”‚  â”‚  â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚  â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚
â”‚  â”‚                   Standard Library                        â”‚  â”‚
â”‚  â”‚  io â”‚ json â”‚ http â”‚ fs â”‚ time â”‚ utils â”‚ testing â”‚ ...   â”‚  â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

### Component Descriptions

#### CLI (`pulse_cli`)

The command-line interface providing all user-facing commands:
- `run` - Execute scripts via the VM
- `repl` - Interactive read-eval-print loop
- `test` - Test runner
- `benchmark` - Performance profiling
- `build` - AOT compilation to native binaries
- `init` / `add` - Project and dependency management

#### Compiler (`pulse_compiler`)

The Pulse compiler pipeline:
- **Lexer** (`lexer.rs`): Tokenizes source code
- **Parser** (`parser_v2.rs`): Builds abstract syntax tree
- **Type Checker** (`type_checker.rs`): Validates types
- **Code Generator** (`compiler.rs`): Emits bytecode

#### Virtual Machine (`pulse_vm`)

Bytecode interpreter with:
- Stack-based VM architecture
- Garbage collection
- Closure support
- Module system

#### Runtime (`pulse_runtime`)

Actor model implementation:
- **Actor**: Isolated execution units
- **Mailbox**: Asynchronous message queues
- **Cluster**: Distributed actor support
- **Supervision**: Fault tolerance hierarchies

#### LLVM Backend (`pulse_llvm_backend`)

Ahead-of-time compilation:
- JIT compilation for fast iteration
- AOT compilation for production binaries
- LLVM IR generation

#### Standard Library (`pulse_stdlib`)

Native bindings to Rust libraries:
- Core modules: I/O, JSON, HTTP, filesystem, time, math, string utilities, testing
- Seamless integration with the runtime

#### LSP (`pulse_lsp`)

Language Server Protocol implementation:
- Diagnostic reporting
- Code completion
- Text document synchronization

---

## Examples

### Example 1: Concurrent Message Processing

```pulse
// concurrent_processor.pulse
// Spawn multiple workers to process messages in parallel

let worker_count = 5;
let workers = [];

// Create worker pool
let i = 0;
while (i < worker_count) {
    let worker = spawn {
        let id = receive;  // Receive worker ID
        while (true) {
            let task = receive;
            if (task == "shutdown") {
                print("Worker " + to_string(id) + " shutting down");
                return;
            }
            print("Worker " + to_string(id) + " processing: " + task);
        }
    };
    push(workers, worker);
    send worker, i;  // Assign ID
    i = i + 1;
}

// Distribute tasks
let tasks = ["taskA", "taskB", "taskC", "taskD", "taskE", "taskF"];
let j = 0;
while (j < len(tasks)) {
    let worker_idx = j % worker_count;
    send workers[worker_idx], tasks[j];
    j = j + 1;
}

// Shutdown workers
let k = 0;
while (k < worker_count) {
    send workers[k], "shutdown";
    k = k + 1;
}

sleep(500);
print("All workers completed");
```

### Example 2: Object-Oriented Programming with Classes

```pulse
// shapes.pulse
// Demonstrate classes and encapsulation

class Rectangle {
    fn init(width, height) {
        this.width = width;
        this.height = height;
    }

    fn area() {
        return this.width * this.height;
    }

    fn perimeter() {
        return 2 * (this.width + this.height);
    }

    fn scale(factor) {
        this.width = this.width * factor;
        this.height = this.height * factor;
    }
}

class Circle {
    fn init(radius) {
        this.radius = radius;
    }

    fn area() {
        return 3.14159 * this.radius * this.radius;
    }
}

// Usage
let rect = Rectangle(5, 3);
print("Rectangle area: ", rect.area());
print("Rectangle perimeter: ", rect.perimeter());
rect.scale(2);
print("Scaled area: ", rect.area());

let circle = Circle(4);
print("Circle area: ", circle.area());
```

### Example 3: Data Processing

```pulse
// data_processing.pulse
// Manual data processing with lists

// Create arrays
let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// Calculate statistics
let sum = 0;
let i = 0;
while (i < len(data)) {
    sum = sum + data[i];
    i = i + 1;
}

let mean = sum / len(data);

// Calculate variance
let variance_sum = 0;
let j = 0;
while (j < len(data)) {
    let diff = data[j] - mean;
    variance_sum = variance_sum + (diff * diff);
    j = j + 1;
}

let variance = variance_sum / len(data);
let stddev = variance ^ 0.5;

print("Data: ", data);
print("Mean: ", mean);
print("Standard Deviation: ", stddev);

// JSON serialization
let result = {
    "data": data,
    "mean": mean,
    "stddev": stddev
};

let json_str = json_stringify(result);
print("JSON Output: ", json_str);
```

---

## Contributing

### How to Contribute

We welcome contributions from the community! Here are the key areas where help is needed:

1. **LLVM Backend**: Complete the AOT compilation pipeline
2. **Self-Hosted Compiler**: Expand bootstrapping capabilities
3. **Standard Library**: Add new modules and bindings
4. **IDE Integration**: Improve LSP and editor support
5. **Documentation**: Enhance the documentation site

### Development Setup

```bash
# Clone the repository
git clone https://github.com/pulse-lang/pulse.git
cd pulse/pulse_lang

# Build the project
cargo build --release

# Run tests
cargo test --workspace

# Run specific test suites
cargo test -p pulse_vm        # VM tests
cargo test -p pulse_runtime   # Runtime tests
cargo test -p pulse_compiler  # Compiler tests

# Run integration tests
cargo test -p pulse_cli

# Verify AOT + JIT pipeline
./scripts/verify-aot-jit.sh   # Linux/macOS
powershell -File .\scripts\verify-aot-jit.ps1  # Windows

# Package VS Code extension
cd vscode-pulse && npm install && npm run package && cd ..

# Run docs site (Next.js)
cd docs && npm install && npm run dev && cd ..

# Run benchmarks
cargo run --release --bin pulse_cli -- benchmark

# Run the REPL
cargo run --release --bin pulse_cli -- repl
```

### Project Structure

```
pulse_lang/
â”œâ”€â”€ Cargo.toml              # Workspace manifest
â”œâ”€â”€ pulse_cli/              # Command-line interface
â”œâ”€â”€ pulse_compiler/        # Compiler (lexer, parser, type checker)
â”œâ”€â”€ pulse_core/            # Core types and values
â”œâ”€â”€ pulse_vm/              # Bytecode virtual machine
â”œâ”€â”€ pulse_runtime/         # Actor runtime system
â”œâ”€â”€ pulse_stdlib/          # Standard library modules
â”œâ”€â”€ pulse_lsp/             # Language Server Protocol
â”œâ”€â”€ pulse_llvm_backend/    # LLVM AOT compiler
â”œâ”€â”€ pulse_aot_runtime/     # AOT runtime
â”œâ”€â”€ self-hosted/           # Self-hosted compiler in Pulse
â”œâ”€â”€ vscode-pulse/          # VS Code extension
â”œâ”€â”€ examples/              # Example programs
â””â”€â”€ tests/                 # Test suites
```

### Code Style

- Follow Rust formatting conventions (`cargo fmt`)
- Run clippy for linting (`cargo clippy`)
- Write tests for new features
- Update documentation accordingly

---

## License

Pulse is licensed under the **MIT License**.

```
MIT License

Copyright (c) 2024 Pulse Language Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

<p align="center">
  <strong>Built with â¤ï¸ using Rust</strong><br>
  <a href="https://github.com/pulse-lang/pulse">GitHub</a> â€¢
  <a href="https://pulse-lang.dev">Website</a> â€¢
  <a href="https://docs.pulse-lang.dev">Documentation</a>
</p>

