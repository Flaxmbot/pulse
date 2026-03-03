export type DocPage = {
  slug: string;
  title: string;
  summary: string;
  section: string;
  level: "Beginner" | "Intermediate" | "Advanced";
  keywords: string[];
  readTime: string;
  content: string;
};

export const SECTION_ORDER = [
  "Start Here",
  "Tutorials",
  "Language Reference",
  "Guides",
  "Advanced",
  "Interactive"
] as const;

export const DOCS: DocPage[] = [
  {
    slug: "start/overview",
    title: "Pulse Overview",
    summary: "What Pulse is, where it fits, and how to approach learning it.",
    section: "Start Here",
    level: "Beginner",
    keywords: ["overview", "intro", "actor model", "why pulse"],
    readTime: "6 min",
    content: `# Pulse Overview

Pulse is a modern actor-model language designed for high-concurrency and distributed systems.  
You can run programs on the VM quickly, and compile to native binaries when needed.

## Core ideas

- Actor-first concurrency with message passing
- A practical CLI for run, check, test, benchmark, and build
- Multiple execution paths: VM, JIT, and AOT
- A Rust compiler today, with a self-hosted compiler track under active bootstrap

## How to learn Pulse effectively

1. Install tools and run your first program
2. Learn language basics and CLI workflow
3. Build one small project (CLI or actor service)
4. Move into grammar/reference material
5. Learn AOT/JIT tradeoffs and production packaging
6. Explore compiler/runtime internals when needed

## What "production-ready" means here

A production release is not just "it runs examples". You need:

- Strict quality gates (fmt, clippy, tests)
- Deterministic CI on Linux, Windows, macOS
- AOT/JIT verification
- Packaging/installers validated on clean machines
- Self-host track validated by bootstrap and parity workflows

## Next steps

- Continue to **Installation and Toolchain**
- Then complete **Your First Program**
- Start the **Build a CLI App** tutorial`,
  },
  {
    slug: "start/installation",
    title: "Installation and Toolchain",
    summary: "Set up Rust track or self-host track on Windows, Linux, and macOS.",
    section: "Start Here",
    level: "Beginner",
    keywords: ["install", "toolchain", "windows", "linux", "mac", "track"],
    readTime: "10 min",
    content: `# Installation and Toolchain

This page gives a practical setup path for all platforms.

## Prerequisites

- Rust toolchain (stable)
- LLVM/Clang (for AOT/JIT workflows)
- Node.js (for docs and VS Code extension packaging)
- .NET SDK (for WiX SDK MSI build flow on Windows)

## Install from source

\`\`\`bash
git clone https://github.com/pulse-lang/pulse.git
cd pulse/pulse_lang
cargo build --release
\`\`\`

## Track-based install scripts

### Linux/macOS

\`\`\`bash
bash install-rust.sh --with-vscode
bash install-selfhost.sh --with-vscode
\`\`\`

### Windows

\`\`\`powershell
powershell -ExecutionPolicy Bypass -File install-rust.ps1 -WithVSCode
powershell -ExecutionPolicy Bypass -File install-selfhost.ps1 -WithVSCode
\`\`\`

## Verify installation

\`\`\`bash
pulse --help
pulse check .
pulse selfhost --help
\`\`\`

## Build MSI installers (Windows)

\`\`\`powershell
.\scripts\build-msi.ps1 -Track rust -Version 0.1.0
.\scripts\build-msi.ps1 -Track selfhost -Version 0.1.0
\`\`\`

The MSI flow uses WiX SDK through \`dotnet build\`.

## Common setup issues

- Missing clang/LLVM: AOT build fails during linking
- Wrong track expectations: set \`PULSE_COMPILER_TRACK\` explicitly
- Self-host entry not found: ensure \`PULSE_HOME\` and wrappers are used
- VS Code extension not installed: use packaged \`.vsix\` from release artifacts`,
  },
  {
    slug: "start/first-program",
    title: "Your First Program",
    summary: "Write, run, check, and test your first Pulse project end-to-end.",
    section: "Start Here",
    level: "Beginner",
    keywords: ["first program", "run", "check", "test", "hello world"],
    readTime: "8 min",
    content: `# Your First Program

## 1) Create a file

\`\`\`pulse
print("Hello from Pulse!");
\`\`\`

Save as \`main.pulse\`.

## 2) Run it

\`\`\`bash
pulse run main.pulse
\`\`\`

## 3) Add state and control flow

\`\`\`pulse
let total = 0;
let i = 1;
while (i <= 5) {
  total = total + i;
  i = i + 1;
}
print("sum(1..5) =", total);
\`\`\`

## 4) Validate without executing

\`\`\`bash
pulse check main.pulse
\`\`\`

## 5) Add a test file

\`\`\`pulse
let x = 2 + 2;
if (x != 4) {
  panic("math failed");
}
\`\`\`

Run:

\`\`\`bash
pulse test .
\`\`\`

## 6) Build native executable

\`\`\`bash
pulse build main.pulse --emit exe
\`\`\`

## Done

You now have a full loop:

- edit
- check
- run
- test
- build`,
  },
  {
    slug: "tutorials/build-a-cli",
    title: "Tutorial: Build a Small CLI App",
    summary: "A beginner project that parses args and performs useful work.",
    section: "Tutorials",
    level: "Beginner",
    keywords: ["tutorial", "cli", "project", "example"],
    readTime: "14 min",
    content: `# Tutorial: Build a Small CLI App

Goal: build a tiny task CLI that stores tasks in memory for one run.

## Step 1: command dispatcher

\`\`\`pulse
let cmd = "list"; // replace with arg parsing when available
if (cmd == "add") {
  print("add mode");
} else if (cmd == "list") {
  print("list mode");
} else {
  print("unknown command");
}
\`\`\`

## Step 2: represent tasks

\`\`\`pulse
let tasks = [
  {"id": 1, "title": "write docs", "done": false},
  {"id": 2, "title": "run tests", "done": true}
];
\`\`\`

## Step 3: filtering

\`\`\`pulse
let i = 0;
while (i < len(tasks)) {
  let t = tasks[i];
  if (!t["done"]) {
    print("[ ] ", t["title"]);
  }
  i = i + 1;
}
\`\`\`

## Step 4: validate behavior

- Add invalid command path
- Add empty task list path
- Add task with missing fields

Run:

\`\`\`bash
pulse check .
pulse test .
\`\`\`

## Step 5: package

\`\`\`bash
pulse build app.pulse --emit exe
\`\`\`

## Extension ideas

- Persist JSON to file
- Add \`done <id>\` command
- Add sorting by status or title`,
  },
  {
    slug: "tutorials/actor-service",
    title: "Tutorial: Build an Actor-Based Service",
    summary: "Design a resilient actor workflow with supervision behavior.",
    section: "Tutorials",
    level: "Intermediate",
    keywords: ["actors", "service", "supervision", "messages"],
    readTime: "16 min",
    content: `# Tutorial: Build an Actor-Based Service

Goal: create a worker actor and a coordinator actor.

## Step 1: worker actor

\`\`\`pulse
let worker = spawn {
  while (true) {
    let msg = receive;
    print("worker received:", msg);
  }
};
\`\`\`

## Step 2: coordinator actor

\`\`\`pulse
let coordinator = spawn {
  send worker, {"type": "job", "payload": "task-a"};
  send worker, {"type": "job", "payload": "task-b"};
};
\`\`\`

## Step 3: add protocol checks

Define message formats and reject malformed messages early.

\`\`\`pulse
if (msg["type"] != "job") {
  print("unsupported message type");
}
\`\`\`

## Step 4: failure strategy

- Fail fast on invalid internal state
- Use supervisor policy to restart workers
- Emit structured logs for crashes and restarts

## Step 5: load test shape

- Burst 1k messages quickly
- Measure latency and dropped/failed work
- Verify system remains responsive

Run:

\`\`\`bash
pulse benchmark tests/bench_messaging.pulse
\`\`\`

## Production notes

- Enforce bounded mailbox strategy
- Track dead-letter behavior
- Test disconnect/reconnect when clustering is enabled`,
  },
  {
    slug: "reference/grammar",
    title: "Grammar Reference (EBNF)",
    summary: "Formal grammar overview and parser-facing syntax guidance.",
    section: "Language Reference",
    level: "Intermediate",
    keywords: ["grammar", "ebnf", "parser", "syntax"],
    readTime: "15 min",
    content: `# Grammar Reference (EBNF)

This is a practical grammar reference aligned with current parser behavior.

## High-level structure

\`\`\`text
program        = { declaration } EOF ;
declaration    = varDecl | funDecl | classDecl | statement ;
statement      = exprStmt | ifStmt | whileStmt | returnStmt | block ;
block          = "{" { declaration } "}" ;
\`\`\`

## Declarations

\`\`\`text
varDecl        = "let" IDENTIFIER "=" expression ";" ;
funDecl        = "fun" IDENTIFIER "(" [ params ] ")" block ;
params         = IDENTIFIER { "," IDENTIFIER } ;
classDecl      = "class" IDENTIFIER [ ":" IDENTIFIER ] "{" { funDecl } "}" ;
\`\`\`

## Control flow

\`\`\`text
ifStmt         = "if" "(" expression ")" statement [ "else" statement ] ;
whileStmt      = "while" "(" expression ")" statement ;
returnStmt     = "return" [ expression ] ";" ;
\`\`\`

## Expressions

\`\`\`text
expression     = assignment ;
assignment     = IDENTIFIER "=" assignment | logic_or ;
logic_or       = logic_and { "or" logic_and } ;
logic_and      = equality { "and" equality } ;
equality       = comparison { ( "==" | "!=" ) comparison } ;
comparison     = term { ( ">" | ">=" | "<" | "<=" ) term } ;
term           = factor { ( "+" | "-" ) factor } ;
factor         = unary { ( "*" | "/" | "%" ) unary } ;
unary          = ( "!" | "-" | "~" ) unary | call ;
call           = primary { "(" [ args ] ")" | "[" expression "]" } ;
args           = expression { "," expression } ;
\`\`\`

## Practical parser tips

- Semicolons are required for terminating statements
- Keep module exports explicit and terminated
- Avoid ambiguous nested expressions without parentheses
- Validate grammar changes with golden parser tests`,
  },
  {
    slug: "reference/language",
    title: "Language Basics Reference",
    summary: "Variables, expressions, control flow, functions, classes, and modules.",
    section: "Language Reference",
    level: "Beginner",
    keywords: ["variables", "functions", "classes", "modules", "reference"],
    readTime: "18 min",
    content: `# Language Basics Reference

## Variables

\`\`\`pulse
let count = 42;
let name = "pulse";
\`\`\`

## Conditionals

\`\`\`pulse
if (count > 0) {
  print("positive");
} else {
  print("zero or negative");
}
\`\`\`

## Loops

\`\`\`pulse
let i = 0;
while (i < 3) {
  print(i);
  i = i + 1;
}
\`\`\`

## Functions

\`\`\`pulse
fun add(a, b) {
  return a + b;
}
\`\`\`

## Collections

\`\`\`pulse
let list = [1, 2, 3];
let map = {"k": "v"};
\`\`\`

## Classes

\`\`\`pulse
class User {
  fun greet() {
    print("hi");
  }
}
\`\`\`

## Modules and imports

- Keep import paths normalized
- Avoid path traversal patterns
- Prefer explicit module boundaries`,
  },
  {
    slug: "reference/actors",
    title: "Actor and Concurrency Reference",
    summary: "Actor lifecycle, messaging semantics, supervision, and cluster behavior.",
    section: "Language Reference",
    level: "Intermediate",
    keywords: ["actor", "mailbox", "supervisor", "cluster", "concurrency"],
    readTime: "14 min",
    content: `# Actor and Concurrency Reference

## Actor lifecycle

1. Spawn actor
2. Process mailbox messages
3. Handle failures via supervision strategy
4. Shutdown with cleanup hooks

## Message design

- Use typed/structured message shapes
- Include version field for evolving protocols
- Fail early on malformed payloads

## Supervisor behavior

- Restart on recoverable failure
- Escalate on repeated crash loops
- Emit restart events for observability

## Backpressure

- Define mailbox pressure limits
- Drop, defer, or dead-letter intentionally
- Validate with stress tests

## Distributed runtime concerns

- Node disconnect/rejoin handling
- Monitor/link propagation correctness
- Deterministic behavior under churn`,
  },
  {
    slug: "reference/cli",
    title: "CLI Reference",
    summary: "Complete command reference for development, verification, and release.",
    section: "Language Reference",
    level: "Beginner",
    keywords: ["cli", "check", "build", "selfhost", "diagnostics-json"],
    readTime: "12 min",
    content: `# CLI Reference

## Core commands

\`\`\`text
pulse run <file>
pulse repl
pulse test [path]
pulse check [path] [--diagnostics-json]
pulse benchmark [file]
pulse build <file> [--emit bc|ir|obj|exe] [--link=true|false]
pulse selfhost test
pulse selfhost bootstrap
\`\`\`

## Check mode

Use \`pulse check\` to parse/typecheck without execution.

\`\`\`bash
pulse check self-hosted --diagnostics-json
\`\`\`

## Build modes

- \`--emit bc\`: bytecode
- \`--emit ir\`: LLVM IR
- \`--emit obj\`: native object
- \`--emit exe\`: executable

## Compiler tracks

\`\`\`bash
PULSE_COMPILER_TRACK=rust pulse run app.pulse
PULSE_COMPILER_TRACK=selfhost pulse run app.pulse
\`\`\`

Install wrappers set this for you:

- \`pulse-rust\`
- \`pulse-selfhost\`
- \`pulse\` (default track wrapper)`,
  },
  {
    slug: "guides/testing-and-quality",
    title: "Guide: Testing and Quality Gates",
    summary: "Unit/integration/edge/stress/fuzz/perf strategy for production quality.",
    section: "Guides",
    level: "Intermediate",
    keywords: ["testing", "quality", "ci", "stress", "fuzz", "coverage"],
    readTime: "17 min",
    content: `# Guide: Testing and Quality Gates

## Baseline merge gates

\`\`\`bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
\`\`\`

## Required test layers

- Unit: lexer/parser/typechecker/compiler/vm/runtime/stdlib/lsp/cli
- Integration: end-to-end language programs
- Edge cases: overflow, divide by zero, recursion depth, malformed imports
- Stress: actor storms, mailbox pressure, large module graphs
- Fuzz: lexer/parser/loader/import resolver
- Perf: compile latency, throughput, memory envelope

## Determinism requirements

- Same source + seed => stable behavior across OSes
- VM path and AOT/JIT path should match semantic outputs

## Release policy

- Block release on any regression
- Run deep stress/fuzz/perf nightly
- Preserve historical baseline metrics`,
  },
  {
    slug: "guides/debugging",
    title: "Guide: Debugging and Diagnostics",
    summary: "Practical workflows for tracing compile/runtime failures quickly.",
    section: "Guides",
    level: "Intermediate",
    keywords: ["debugging", "diagnostics", "errors", "triage"],
    readTime: "11 min",
    content: `# Guide: Debugging and Diagnostics

## Compile failures

- Run \`pulse check --diagnostics-json\`
- Inspect exact line and message
- Minimize reproducer before fixing

## Runtime failures

- Isolate to smallest program that fails
- Validate actor message contract
- Check for panic-prone paths and convert to typed errors when user-triggerable

## AOT/JIT issues

- Verify LLVM and linker availability
- Run backend test suites directly
- Compare VM and AOT/JIT output for parity

## Useful commands

\`\`\`bash
pulse check . --diagnostics-json
powershell -File scripts/verify-aot-jit.ps1
cargo test -p pulse_llvm_backend --test jit_test
\`\`\`

## Bug report template

1. Exact command
2. Input file
3. Full error output
4. Expected behavior
5. Platform/toolchain versions`,
  },
  {
    slug: "guides/aot-jit",
    title: "Guide: AOT and JIT Workflows",
    summary: "When to use VM vs JIT vs AOT and how to verify each backend.",
    section: "Guides",
    level: "Intermediate",
    keywords: ["aot", "jit", "llvm", "build", "backend"],
    readTime: "13 min",
    content: `# Guide: AOT and JIT Workflows

## Backend choices

- VM: fastest edit/run cycle
- JIT: runtime optimized execution for dynamic workloads
- AOT: distributable native executables

## AOT build examples

\`\`\`bash
pulse build examples/hello_aot.pulse --emit ir
pulse build examples/hello_aot.pulse --emit obj
pulse build examples/hello_aot.pulse --emit exe
\`\`\`

## Link control

\`\`\`bash
pulse build examples/hello_aot.pulse --emit exe --link=false
\`\`\`

## Validation

\`\`\`bash
powershell -File scripts/verify-aot-jit.ps1
\`\`\`

## Release rules

- Every release must verify AOT link on all tier-1 platforms
- Runtime ABI compatibility tests must stay green
- Object/IR emission should remain reproducible enough for diff-based diagnosis`,
  },
  {
    slug: "guides/packaging",
    title: "Guide: Packaging and Installers",
    summary: "Build distributable artifacts for Windows, Linux, macOS, and VS Code.",
    section: "Guides",
    level: "Intermediate",
    keywords: ["packaging", "installer", "msi", "tar", "vsix"],
    readTime: "14 min",
    content: `# Guide: Packaging and Installers

## Windows MSI (two tracks)

\`\`\`powershell
.\scripts\build-msi.ps1 -Track rust -Version 0.1.0
.\scripts\build-msi.ps1 -Track selfhost -Version 0.1.0
\`\`\`

Output:

- \`dist/windows/pulse-rust-<version>.msi\`
- \`dist/windows/pulse-selfhost-<version>.msi\`

## Linux/macOS tarball packages

\`\`\`bash
./scripts/build-packages.sh --track rust --version 0.1.0
./scripts/build-packages.sh --track selfhost --version 0.1.0
\`\`\`

## VS Code extension

\`\`\`bash
cd vscode-pulse
npm install
npm run compile
npm run package
\`\`\`

## Packaging checklist

- CLI executable included
- LSP binary included
- Runtime libs included
- self-hosted sources included for selfhost workflows
- wrapper scripts set \`PULSE_HOME\` and track env vars correctly`,
  },
  {
    slug: "advanced/compiler-architecture",
    title: "Compiler Architecture Deep Dive",
    summary: "Pipeline, invariants, and testing strategy for compiler contributors.",
    section: "Advanced",
    level: "Advanced",
    keywords: ["compiler", "lexer", "parser", "typechecker", "bytecode"],
    readTime: "20 min",
    content: `# Compiler Architecture Deep Dive

## Pipeline

1. Lex source into token stream
2. Parse tokens into AST
3. Type-check and annotate constraints
4. Emit bytecode
5. Feed VM or LLVM backend

## Critical invariants

- Parser recovery must not corrupt downstream diagnostics
- Type inference must preserve soundness constraints
- Bytecode stack effects must be validated per opcode
- Import path sanitization must block traversal and disallowed paths

## Recommended test strategy

- Golden token snapshots
- AST snapshot fixtures
- Negative parse diagnostics with exact spans
- Bytecode golden fixtures for control-flow and closures
- Differential behavior tests: VM vs AOT/JIT

## Contributor workflow

\`\`\`bash
cargo test -p pulse_compiler
cargo test -p pulse_compiler --test compiler_pipeline_tests
\`\`\`

## Common pitfalls

- Silent parser acceptance of invalid trailing constructs
- Weak assertions in backend tests
- Hidden behavior differences between debug/release execution`,
  },
  {
    slug: "advanced/runtime-architecture",
    title: "Runtime Architecture Deep Dive",
    summary: "Actor runtime internals, supervision model, and reliability concerns.",
    section: "Advanced",
    level: "Advanced",
    keywords: ["runtime", "actor", "mailbox", "supervision", "cluster"],
    readTime: "18 min",
    content: `# Runtime Architecture Deep Dive

## Runtime layers

- Actor scheduling
- Mailbox and message envelope
- Supervision and restart policies
- Network/cluster membership and monitoring

## Reliability goals

- No user-triggerable panics in public execution paths
- Predictable restart semantics
- Clean shutdown and registry cleanup
- Monitor/link propagation under churn

## Required stress scenarios

- Actor storm with sustained mailbox pressure
- Long soak run (24h)
- Frequent node disconnect/rejoin
- Repeated compile-run loops

## Observability expectations

- Structured restart events
- Dead-letter counters
- Per-node health snapshots
- Latency and throughput histograms`,
  },
  {
    slug: "advanced/selfhost-bootstrap",
    title: "Self-Hosted Compiler Bootstrap",
    summary: "Parity harness and staged promotion from Rust compiler to self-host.",
    section: "Advanced",
    level: "Advanced",
    keywords: ["selfhost", "bootstrap", "parity", "promotion"],
    readTime: "19 min",
    content: `# Self-Hosted Compiler Bootstrap

## Objective

Promote self-host compiler only after sustained parity and reliability.

## Stages

1. Rust compiler builds self-host artifact
2. Self-host compiler recompiles itself
3. Compare outputs semantically (bytecode/behavior)
4. Dual-compiler CI for multiple cycles
5. Canary release with rollback path
6. Promote when SLO and parity targets are sustained

## Commands

\`\`\`bash
pulse selfhost test
pulse selfhost bootstrap
pulse check self-hosted --diagnostics-json
\`\`\`

## Parity definition

- Same language corpus passes on both compilers
- No semantic regressions in observable behavior
- Bootstrap repeatability over consecutive release cycles

## Promotion guardrails

- Rust remains canonical compiler until criteria met
- Automatic rollback path remains active
- Production release blocked on parity regressions`,
  },
  {
    slug: "interactive/playground",
    title: "Pulse Playground",
    summary: "Interactive coding environment to learn and experiment with Pulse language.",
    section: "Interactive",
    level: "Beginner",
    keywords: ["playground", "interactive", "live", "demo"],
    readTime: "5 min",
    content: `# Pulse Playground

The Pulse Playground is an interactive environment where you can write, run, and test Pulse code without having to install anything locally.

## Features

- **Live Code Editing**: Write and edit Pulse code in the integrated editor
- **Instant Execution**: Run your code with a single click
- **Real-time Output**: See results and errors immediately
- **Error Handling**: Clear error messages and stack traces
- **Syntax Highlighting**: Code is highlighted for better readability
- **Reset Functionality**: Start fresh with the default example code

## Getting Started

1. **Write Code**: Type your Pulse code in the editor on the left
2. **Run**: Click the "Run" button to execute your code
3. **View Output**: See the results in the output panel on the right
4. **Reset**: Click "Reset" to return to the default example

## Example Code

The playground comes with a default example that demonstrates:
- Basic variable declaration and manipulation
- Loop constructs
- Print statements
- Actor creation and message passing

## What You Can Try

- Modify the existing code to see how it affects the output
- Try different control flow statements
- Experiment with actor messaging patterns
- Test arithmetic operations and string manipulation

## Limitations

The playground is a demo environment and has some limitations:
- Execution time is limited
- No file system access
- No network connectivity
- Limited memory resources
- Not suitable for large or complex programs

For production use, we recommend installing the Pulse compiler and runtime locally.
`,
  },
  {
    slug: "advanced/production-checklist",
    title: "Production Release Checklist",
    summary: "End-to-end release checklist for language, runtime, tooling, and artifacts.",
    section: "Advanced",
    level: "Advanced",
    keywords: ["production", "release", "checklist", "sign-off"],
    readTime: "12 min",
    content: `# Production Release Checklist

## Language and compiler

- Lexer/parser/typechecker/compiler suites green
- Negative diagnostics verified
- Security checks: import/path traversal and capability boundaries

## Runtime and stdlib

- Actor/runtime stress and soak pass
- Supervision and cleanup semantics verified
- Stdlib modules aligned with docs claims

## Backends

- JIT test suite deterministic and meaningful assertions
- AOT build+run validated on Linux, Windows, macOS
- ABI compatibility checks green

## Tooling

- CLI contract tests pass
- LSP diagnostics stability verified
- VS Code extension package and smoke install validated

## Packaging and distribution

- MSI rust/selfhost built and install tested
- Unix tarballs built and smoke tested
- Artifact integrity hashes generated and published

## Governance

- CI gates required for merge
- Release notes include breaking changes and migration notes
- Rollback plan tested before public release`,
  }
];

export function getAllDocs(): DocPage[] {
  return [...DOCS];
}

export function getDocBySlug(slugPath: string): DocPage | undefined {
  return DOCS.find((doc) => doc.slug === slugPath);
}

export function getDocsBySection(): Record<string, DocPage[]> {
  const grouped: Record<string, DocPage[]> = {};
  for (const section of SECTION_ORDER) {
    grouped[section] = [];
  }

  for (const doc of DOCS) {
    if (!grouped[doc.section]) {
      grouped[doc.section] = [];
    }
    grouped[doc.section].push(doc);
  }

  return grouped;
}

