use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use pulse_core::{Diagnostic as PulseDiagnostic, PulseError};
use pulse_runtime::Runtime;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
// mod repl; moved to submodule

mod docs;
mod package;
mod repl;

static SELFHOST_BOOTSTRAP_DONE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompilerTrack {
    Rust,
    Selfhost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BuildEmit {
    Bc,
    Ir,
    Obj,
    Exe,
}

#[derive(Subcommand)]
enum SelfhostCommand {
    /// Run all self-hosted compiler files as smoke tests
    Test,
    /// Run self-hosted compiler bootstrap smoke test
    Bootstrap {
        /// Entry file to run (default: self-hosted/compiler.pulse)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

#[derive(Parser)]
#[command(name = "pulse")]
#[command(version = "0.1.2")]
#[command(about = "The Pulse programming language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a Pulse source file
    Run {
        /// Path to the .pulse file
        file: PathBuf,
    },
    /// Start the interactive REPL
    Repl,
    /// Run the demo programs
    Demo,
    /// Run tests in a file or directory
    Test {
        /// Path to test file or directory (default: tests/)
        path: Option<PathBuf>,
    },
    /// Parse + typecheck source files without executing
    Check {
        /// Path to .pulse file or directory (default: current directory)
        path: Option<PathBuf>,
        /// Emit machine-readable diagnostics JSON
        #[arg(long, default_value_t = false)]
        diagnostics_json: bool,
    },
    /// Run performance benchmarks
    Benchmark {
        /// Path to benchmark file (default: bench_baseline.pulse)
        file: Option<PathBuf>,
        /// Number of iterations
        #[arg(short, long, default_value = "3")]
        iterations: usize,
        /// Optional performance gate: fail if average runtime exceeds this value (ms)
        #[arg(long)]
        max_avg_ms: Option<f64>,
    },
    /// Initialize a new Pulse project
    Init {
        /// Project directory (default: current dir)
        path: Option<PathBuf>,
        /// Project name (default: directory name)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Add a dependency to the project
    Add {
        /// Package name
        package: String,
        /// Package version (default: latest)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Build the project to a native executable
    Build {
        /// Path to the .pulse file
        file: PathBuf,
        /// Output executable name
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Build in release mode (with optimizations)
        #[arg(short, long)]
        release: bool,
        /// Link object file into executable (default: true)
        #[arg(long, default_value_t = true, action = ArgAction::Set)]
        link: bool,
        /// Select output artifact
        #[arg(long, value_enum, default_value_t = BuildEmit::Exe)]
        emit: BuildEmit,
    },
    /// Generate documentation
    Doc {
        /// Source directory (default: src/)
        #[arg(short, long)]
        source: Option<PathBuf>,
        /// Output directory (default: docs/)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Self-hosted compiler workflows
    Selfhost {
        #[command(subcommand)]
        command: SelfhostCommand,
    },
}

#[tokio::main]
async fn main() {
    // Graceful shutdown handler
    tokio::spawn(async {
        #[allow(clippy::redundant_pattern_matching)]
        if let Ok(_) = tokio::signal::ctrl_c().await {
            println!("\nShutting down Pulse CLI...");
            std::process::exit(0);
        }
    });

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file, .. }) => {
            if let Err(e) = run_file(file).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Repl) => {
            repl::start().await;
        }
        Some(Commands::Demo) => {
            run_demo();
        }
        Some(Commands::Test { path }) => {
            if let Err(e) = run_tests(path) {
                eprintln!("Test Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Check {
            path,
            diagnostics_json,
        }) => {
            let check_path = path.unwrap_or_else(|| PathBuf::from("."));
            if let Err(e) = check_sources(check_path, diagnostics_json) {
                eprintln!("Check Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Benchmark {
            file,
            iterations,
            max_avg_ms,
        }) => {
            if let Err(e) = run_benchmarks(file, iterations, max_avg_ms) {
                eprintln!("Benchmark Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Init { path, name }) => {
            if let Err(e) = init_project(path, name) {
                eprintln!("Init Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Add { package, version }) => {
            if let Err(e) = add_dependency(package, version) {
                eprintln!("Add Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Build {
            file,
            output,
            release,
            link,
            emit,
        }) => {
            if let Err(e) = build_file(file, output, release, link, emit) {
                eprintln!("Build Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Doc { source, output }) => {
            if let Err(e) = generate_docs(source, output) {
                eprintln!("Doc Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Selfhost { command }) => {
            if let Err(e) = run_selfhost(command).await {
                eprintln!("Selfhost Error: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // Default to REPL if no command provided
            repl::start().await;
        }
    }
}

#[derive(Debug, Serialize)]
struct CheckDiagnostic {
    #[serde(flatten)]
    diagnostic: PulseDiagnostic,
}

#[derive(Debug, Serialize)]
struct CheckFileResult {
    file: String,
    success: bool,
    diagnostics: Vec<CheckDiagnostic>,
}

fn pulse_error_to_diagnostic(err: &PulseError) -> CheckDiagnostic {
    CheckDiagnostic {
        diagnostic: err.to_diagnostic(),
    }
}

fn collect_pulse_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "pulse") {
            return Ok(vec![path.to_path_buf()]);
        }
        return Err(format!("Not a .pulse file: {}", path.display()));
    }

    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()));
    }

    let mut files = Vec::new();
    let entries =
        fs::read_dir(path).map_err(|e| format!("Failed to read directory {:?}: {}", path, e))?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            files.extend(collect_pulse_files(&entry_path)?);
        } else if entry_path.extension().is_some_and(|e| e == "pulse") {
            files.push(entry_path);
        }
    }
    files.sort();
    Ok(files)
}

fn selected_compiler_track() -> CompilerTrack {
    match std::env::var("PULSE_COMPILER_TRACK")
        .unwrap_or_else(|_| "rust".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "selfhost" => CompilerTrack::Selfhost,
        _ => CompilerTrack::Rust,
    }
}

fn resolve_selfhost_entry() -> PathBuf {
    if let Ok(entry) = std::env::var("PULSE_SELFHOST_ENTRY") {
        return PathBuf::from(entry);
    }

    resolve_selfhost_dir().join("compiler.pulse")
}

fn resolve_selfhost_dir() -> PathBuf {
    let local = PathBuf::from("self-hosted");
    if local.exists() {
        return local;
    }

    if let Ok(home) = std::env::var("PULSE_HOME") {
        return PathBuf::from(home)
            .join("share")
            .join("pulse")
            .join("self-hosted");
    }

    PathBuf::from("self-hosted")
}

async fn ensure_compiler_track_ready_async() -> Result<(), String> {
    if selected_compiler_track() == CompilerTrack::Selfhost
        && !SELFHOST_BOOTSTRAP_DONE.load(Ordering::SeqCst)
    {
        let entry = resolve_selfhost_entry();
        execute_file(entry).await?;
        SELFHOST_BOOTSTRAP_DONE.store(true, Ordering::SeqCst);
    }
    Ok(())
}

fn ensure_compiler_track_ready_blocking() -> Result<(), String> {
    if selected_compiler_track() == CompilerTrack::Rust
        || SELFHOST_BOOTSTRAP_DONE.load(Ordering::SeqCst)
    {
        return Ok(());
    }

    tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime for selfhost bootstrap: {}", e))?;
        rt.block_on(async { ensure_compiler_track_ready_async().await })
    })
}

fn check_sources(path: PathBuf, diagnostics_json: bool) -> Result<(), String> {
    ensure_compiler_track_ready_blocking()?;

    let files = collect_pulse_files(&path)?;
    if files.is_empty() {
        return Err(format!("No .pulse files found under {}", path.display()));
    }

    let mut results = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;

    for file in files {
        let display = file.display().to_string();
        let source =
            fs::read_to_string(&file).map_err(|e| format!("Failed to read {}: {}", display, e))?;
        let module_path = Some(display.clone());

        match pulse_compiler::compile(&source, module_path) {
            Ok(_) => {
                passed += 1;
                results.push(CheckFileResult {
                    file: display,
                    success: true,
                    diagnostics: Vec::new(),
                });
            }
            Err(err) => {
                failed += 1;
                results.push(CheckFileResult {
                    file: display,
                    success: false,
                    diagnostics: vec![pulse_error_to_diagnostic(&err)],
                });
            }
        }
    }

    if diagnostics_json {
        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| format!("Failed to serialize diagnostics JSON: {}", e))?;
        println!("{}", json);
    } else {
        println!("===========================================");
        println!("          Pulse Source Check               ");
        println!("===========================================");
        for result in &results {
            if result.success {
                println!("[OK] {}", result.file);
            } else {
                println!("[FAILED] {}", result.file);
                for d in &result.diagnostics {
                    match d.diagnostic.span.as_ref() {
                        Some(span) => {
                            if let Some(col) = span.column {
                                println!(
                                    "  - {}:{}:{} [{}] {}",
                                    result.file,
                                    span.line,
                                    col,
                                    d.diagnostic.code,
                                    d.diagnostic.message
                                );
                            } else {
                                println!(
                                    "  - {}:{} [{}] {}",
                                    result.file, span.line, d.diagnostic.code, d.diagnostic.message
                                );
                            }
                        }
                        None => {
                            println!("  - [{}] {}", d.diagnostic.code, d.diagnostic.message);
                        }
                    }
                }
            }
        }
        println!();
        println!("Summary: {} passed, {} failed", passed, failed);
    }

    if failed > 0 {
        return Err(format!("{} file(s) failed checks", failed));
    }

    Ok(())
}

async fn run_selfhost(command: SelfhostCommand) -> Result<(), String> {
    match command {
        SelfhostCommand::Test => {
            let selfhost_dir = resolve_selfhost_dir();
            let files = collect_pulse_files(&selfhost_dir)?;
            if files.is_empty() {
                return Err(format!(
                    "No self-hosted .pulse files found under {}",
                    selfhost_dir.display()
                ));
            }

            let mut failed = 0usize;
            for file in files {
                if let Err(err) = execute_file(file.clone()).await {
                    eprintln!("[FAILED] {}: {}", file.display(), err);
                    failed += 1;
                } else {
                    println!("[OK] {}", file.display());
                }
            }

            if failed > 0 {
                return Err(format!("{} self-hosted file(s) failed", failed));
            }
            println!("Self-host test suite passed.");
            Ok(())
        }
        SelfhostCommand::Bootstrap { file } => {
            let entry = file.unwrap_or_else(resolve_selfhost_entry);
            execute_file(entry.clone()).await?;
            SELFHOST_BOOTSTRAP_DONE.store(true, Ordering::SeqCst);
            println!("Bootstrap smoke succeeded: {}", entry.display());
            Ok(())
        }
    }
}

async fn run_file(path: PathBuf) -> Result<(), String> {
    ensure_compiler_track_ready_async().await?;
    execute_file(path).await
}

async fn execute_file(path: PathBuf) -> Result<(), String> {
    let source =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    let chunk = pulse_compiler::compile(&source, Some(path.to_string_lossy().to_string()))
        .map_err(|e| format!("Compilation error: {}", e))?;

    let runtime = Runtime::new(0); // Node ID 0 for CLI
    runtime
        .handle
        .spawn(chunk, Some(path.to_string_lossy().to_string()))
        .await; // Assuming spawn returns just pid, ignored for now

    runtime
        .run()
        .await
        .map_err(|e| format!("Runtime error:\n{}", e))
}

// run_repl moved to repl::start()

fn run_demo() {
    println!("===========================================");
    println!("       Pulse Language Demo Programs       ");
    println!("===========================================\n");

    let examples_dir = PathBuf::from("examples");

    // Collect all .pulse files
    let mut demo_files: Vec<PathBuf> = Vec::new();

    if examples_dir.exists() {
        if let Ok(entries) = fs::read_dir(&examples_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "pulse") {
                    demo_files.push(path);
                }
            }
        }
    }

    if demo_files.is_empty() {
        println!("No demo files found in examples/ directory.");
        println!("Try running: pulse run <file.pulse>");
        return;
    }

    println!("Found {} demo programs:\n", demo_files.len());

    // Run each demo file synchronously using spawn_blocking
    for (i, file) in demo_files.iter().enumerate() {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("{}. Running: {}", i + 1, name);
        println!("{}", "-".repeat(40));

        // Run the file in a blocking task since we can't spawn a new runtime
        let result = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Expected a value");
            rt.block_on(async { run_file(file.clone()).await })
        });

        match result {
            Ok(_) => println!("[OK] {}\n", name),
            Err(e) => println!("[FAILED] {}: {}\n", name, e),
        }
    }

    println!("===========================================");
    println!("       Demo completed successfully!        ");
    println!("===========================================");
}

fn run_tests(path: Option<PathBuf>) -> Result<(), String> {
    let test_path = path.unwrap_or_else(|| PathBuf::from("tests"));

    println!("===========================================");
    println!("          Pulse Test Runner                 ");
    println!("===========================================\n");

    if !test_path.exists() {
        return Err(format!("Test directory not found: {:?}", test_path));
    }

    // Collect all test files
    let test_files = collect_test_files(&test_path)?;

    if test_files.is_empty() {
        return Err("No .pulse test files found".to_string());
    }

    println!("Found {} test file(s)\n", test_files.len());

    let mut passed = 0;
    let mut failed = 0;

    // Run each test file
    for file in &test_files {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        print!("Running: {} ... ", name);

        // Run the test file using block_in_place
        let result = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Expected a value");
            rt.block_on(async { run_file(file.clone()).await })
        });

        match result {
            Ok(_) => {
                println!("PASSED");
                passed += 1;
            }
            Err(e) => {
                println!("FAILED");
                println!("  Error: {}", e);
                failed += 1;
            }
        }
    }

    println!("\n===========================================");
    println!("Test Results: {} passed, {} failed", passed, failed);
    println!("===========================================");

    if failed > 0 {
        return Err(format!("{} test(s) failed", failed));
    }

    Ok(())
}

fn collect_test_files(dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "pulse") {
                files.push(path);
            } else if path.is_dir() {
                files.extend(collect_test_files(&path)?);
            }
        }
    }

    // Sort for consistent ordering
    files.sort();
    Ok(files)
}

fn init_project(path: Option<PathBuf>, name: Option<String>) -> Result<(), String> {
    let project_path = path.unwrap_or_else(|| PathBuf::from("."));

    // Create directory if it doesn't exist
    if !project_path.exists() {
        fs::create_dir_all(&project_path)
            .map_err(|e| format!("Failed to create project directory: {}", e))?;
    }

    // Use the existing package module
    package::init_project(&project_path, name.clone())?;

    let project_name = name.unwrap_or_else(|| {
        project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pulse_project")
            .to_string()
    });

    println!("===========================================");
    println!("     Pulse Project Initialized!           ");
    println!("===========================================");
    println!();
    println!("Project: {}", project_name);
    println!("Created: Pulse.toml");
    println!("Created: src/main.pulse");
    println!("Created: .gitignore");
    println!();
    println!("Next steps:");
    println!("  cd {}", project_path.display());
    println!("  pulse run src/main.pulse");
    println!("  pulse add <package>  # Add dependencies");
    println!("===========================================");

    Ok(())
}

fn add_dependency(package: String, version: Option<String>) -> Result<(), String> {
    // Find the project directory (look for Pulse.toml)
    let project_dir = find_project_root()?;

    println!("Adding dependency: {} {:?}", package, version);

    // Use the existing package module
    package::add_dependency(&project_dir, &package, version.as_deref())?;

    println!("===========================================");
    println!("Dependency added successfully!");
    println!("===========================================");
    println!();
    println!("Note: Package registry not yet implemented.");
    println!("The dependency has been added to Pulse.toml.");
    println!("Run 'pulse build' to compile your project.");

    Ok(())
}

fn find_project_root() -> Result<PathBuf, String> {
    let mut current =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    // Walk up the directory tree looking for Pulse.toml
    loop {
        let manifest_path = current.join("Pulse.toml");
        if manifest_path.exists() {
            return Ok(current);
        }

        if !current.pop() {
            return Err("No Pulse.toml found. Run 'pulse init' first.".to_string());
        }
    }
}

fn runtime_search_roots(mode: &str) -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("target").join(mode)];

    if let Ok(home) = std::env::var("PULSE_HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join("lib"));
        roots.push(home);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            roots.push(exe_dir.to_path_buf());
            roots.push(exe_dir.join("lib"));
            if let Some(parent) = exe_dir.parent() {
                roots.push(parent.join("lib"));
                roots.push(parent.to_path_buf());
            }
        }
    }

    roots
}

fn runtime_staticlib_path(mode: &str) -> Result<(PathBuf, bool), String> {
    let lib_names: &[&str] = if cfg!(windows) {
        &["pulse_aot_runtime.dll.lib", "pulse_aot_runtime.lib"]
    } else {
        &["libpulse_aot_runtime.a"]
    };
    let roots = runtime_search_roots(mode);

    for lib_name in lib_names {
        for root in &roots {
            let candidate = root.join(lib_name);
            if candidate.exists() {
                let requires_runtime_dll =
                    cfg!(windows) && *lib_name == "pulse_aot_runtime.dll.lib";
                return Ok((candidate, requires_runtime_dll));
            }
        }
    }

    let searched = roots
        .iter()
        .flat_map(|root| lib_names.iter().map(move |name| root.join(name)))
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(format!(
        "AOT runtime link library not found. Searched: {}",
        searched
    ))
}

fn runtime_dll_path(mode: &str) -> Result<PathBuf, String> {
    let roots = runtime_search_roots(mode);

    for root in &roots {
        let candidate = root.join("pulse_aot_runtime.dll");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let searched = roots
        .iter()
        .map(|root| root.join("pulse_aot_runtime.dll").display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "AOT runtime DLL not found (required by import library). Searched: {}",
        searched
    ))
}

fn executable_path_from_output(output_name: &Path) -> PathBuf {
    if cfg!(windows) {
        output_name.with_extension("exe")
    } else {
        output_name.to_path_buf()
    }
}

fn link_native_executable(
    mode: &str,
    object_file: &Path,
    output_name: &Path,
) -> Result<PathBuf, String> {
    let (runtime_lib, requires_runtime_dll) = runtime_staticlib_path(mode)?;

    let executable = executable_path_from_output(output_name);
    let mut cmd = Command::new("clang");
    cmd.arg(object_file)
        .arg(&runtime_lib)
        .arg("-o")
        .arg(&executable);

    if !cfg!(windows) {
        cmd.arg("-ldl").arg("-lpthread");
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to invoke clang for linking: {}", e))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Linker failed:\n{}\n{}", stdout, stderr));
    }

    if cfg!(windows) && requires_runtime_dll {
        let runtime_dll = runtime_dll_path(mode)?;
        let output_dir = executable.parent().ok_or_else(|| {
            format!(
                "Unable to determine output directory for executable {}",
                executable.display()
            )
        })?;
        let copied_dll = output_dir.join("pulse_aot_runtime.dll");
        if runtime_dll != copied_dll {
            fs::copy(&runtime_dll, &copied_dll).map_err(|e| {
                format!(
                    "Linked executable but failed to copy runtime DLL from {} to {}: {}",
                    runtime_dll.display(),
                    copied_dll.display(),
                    e
                )
            })?;
        }
    }

    Ok(executable)
}

fn build_file(
    path: PathBuf,
    output: Option<PathBuf>,
    release: bool,
    link: bool,
    emit: BuildEmit,
) -> Result<(), String> {
    ensure_compiler_track_ready_blocking()?;

    let source = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mode = if release { "release" } else { "debug" };
    let want_bc = matches!(emit, BuildEmit::Bc | BuildEmit::Exe);
    let want_ir = matches!(emit, BuildEmit::Ir | BuildEmit::Exe);
    let want_obj = matches!(emit, BuildEmit::Obj | BuildEmit::Exe);
    let want_exe = matches!(emit, BuildEmit::Exe);

    println!("===========================================");
    println!("     Pulse Native Build Pipeline           ");
    println!("===========================================");
    println!("Building: {}", path.display());
    println!("Mode: {}", mode);
    println!("Emit: {:?}", emit);
    println!("Link: {}", link);
    println!();

    let start = Instant::now();

    // Step 1: Compile to bytecode
    println!("[1/4] Compiling to bytecode...");
    let chunk = pulse_compiler::compile(&source, Some(path.to_string_lossy().to_string()))
        .map_err(|e| format!("Compilation error: {}", e))?;
    println!(
        "      {} instructions, {} constants",
        chunk.code.len(),
        chunk.constants.len()
    );

    // Create output directory
    let output_dir = PathBuf::from("target").join(mode);
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let output_name = output.unwrap_or_else(|| {
        path.file_stem()
            .map(|s| output_dir.join(s))
            .unwrap_or_else(|| output_dir.join("a.out"))
    });

    let bc_file = output_name.with_extension("pulsebc");
    if want_bc {
        let bytecode = serde_json::to_vec(&chunk)
            .map_err(|e| format!("Failed to serialize bytecode: {}", e))?;
        fs::write(&bc_file, &bytecode).map_err(|e| format!("Failed to write bytecode: {}", e))?;
        println!("      Bytecode saved: {}", bc_file.display());
    }

    let ir_file = output_name.with_extension("ll");
    let obj_file = output_name.with_extension("o");
    let mut generated_executable = None;

    if want_ir || want_obj || want_exe {
        // Step 2: AOT compile to LLVM IR + object file
        println!("[2/4] AOT compiling to native code...");
        let context = inkwell::context::Context::create();
        let mut backend = pulse_llvm_backend::LLVMBackend::new(&context)
            .map_err(|e| format!("LLVM backend init failed: {}", e))?;

        let _func = backend
            .compile_chunk(&chunk)
            .map_err(|e| format!("AOT compilation failed: {}", e))?;

        // Generate main() entry point
        let _main = backend
            .generate_main_entry()
            .map_err(|e| format!("Main entry generation failed: {}", e))?;

        if want_ir {
            backend
                .emit_ir(&ir_file)
                .map_err(|e| format!("Failed to save LLVM IR: {}", e))?;
            println!("      LLVM IR saved: {}", ir_file.display());
        }

        if want_obj || want_exe {
            println!("[3/4] Emitting object file...");
            backend
                .emit_object_file(&obj_file)
                .map_err(|e| format!("Object file emission failed: {}", e))?;
            println!("      Object file: {}", obj_file.display());
        }
    }

    if want_exe {
        println!("[4/4] Linking native executable...");
        if link {
            let exe = link_native_executable(mode, &obj_file, &output_name)?;
            println!("      Executable: {}", exe.display());
            generated_executable = Some(exe);
        } else {
            println!("      Skipped linking (--link=false)");
        }
    }

    let elapsed = start.elapsed();
    println!("[done] Build artifacts generated.");
    println!();
    println!("===========================================");
    println!("Build completed in {:.2?}", elapsed);
    println!("===========================================");
    println!();
    println!("Artifacts:");
    if want_bc {
        println!("  Bytecode: {}", bc_file.display());
    }
    if want_ir {
        println!("  LLVM IR:  {}", ir_file.display());
    }
    if want_obj || want_exe {
        println!("  Object:   {}", obj_file.display());
    }
    if let Some(exe) = generated_executable {
        println!("  EXE:      {}", exe.display());
    } else if want_exe && !link {
        println!("  EXE:      not linked (requested --link=false)");
    }

    Ok(())
}

fn run_benchmarks(
    file: Option<PathBuf>,
    iterations: usize,
    max_avg_ms: Option<f64>,
) -> Result<(), String> {
    let bench_path = file.unwrap_or_else(|| PathBuf::from("bench_baseline.pulse"));

    println!("===========================================");
    println!("     Pulse Benchmark Runner               ");
    println!("===========================================");
    println!();
    println!("Benchmark: {}", bench_path.display());
    println!("Iterations: {}", iterations);
    println!();

    if !bench_path.exists() {
        return Err(format!("Benchmark file not found: {:?}", bench_path));
    }

    let mut times: Vec<u128> = Vec::new();

    println!("Running benchmark...");
    println!("{}", "-".repeat(40));

    for i in 0..iterations {
        print!("  Iteration {}/{} ... ", i + 1, iterations);

        let start = Instant::now();

        // Compile and run the benchmark using block_in_place
        let result = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Expected a value");
            rt.block_on(async { run_file(bench_path.clone()).await })
        });

        let elapsed = start.elapsed().as_nanos();
        times.push(elapsed);

        match result {
            Ok(_) => println!("{:.2} ms", elapsed as f64 / 1_000_000.0),
            Err(e) => {
                println!("FAILED");
                return Err(format!("Benchmark execution failed: {}", e));
            }
        }
    }

    // Calculate statistics
    let sum: u128 = times.iter().sum();
    let avg = sum / times.len() as u128;
    let min = times.iter().min().copied().unwrap_or(0);
    let max = times.iter().max().copied().unwrap_or(0);

    println!();
    println!("{}", "-".repeat(40));
    println!("Benchmark Results:");
    let avg_ms = avg as f64 / 1_000_000.0;
    println!("  Average: {:.2} ms", avg_ms);
    println!("  Min:     {:.2} ms", min as f64 / 1_000_000.0);
    println!("  Max:     {:.2} ms", max as f64 / 1_000_000.0);

    let env_gate = std::env::var("PULSE_BENCH_MAX_AVG_MS")
        .ok()
        .and_then(|s| s.parse::<f64>().ok());
    let gate_ms = max_avg_ms.or(env_gate);
    if let Some(limit_ms) = gate_ms {
        println!("  Gate:    <= {:.2} ms", limit_ms);
        if avg_ms > limit_ms {
            return Err(format!(
                "Benchmark regression: average {:.2} ms exceeded gate {:.2} ms",
                avg_ms, limit_ms
            ));
        }
    }
    println!();
    println!("===========================================");
    println!("Benchmark completed!");
    println!("===========================================");

    Ok(())
}

fn generate_docs(source: Option<PathBuf>, output: Option<PathBuf>) -> Result<(), String> {
    let src_dir = source.unwrap_or_else(|| PathBuf::from("src"));
    let out_dir = output.unwrap_or_else(|| PathBuf::from("docs"));

    println!("===========================================");
    println!("     Pulse Documentation Generator        ");
    println!("===========================================");
    println!();
    println!("Source: {}", src_dir.display());
    println!("Output: {}", out_dir.display());
    println!();

    if !src_dir.exists() {
        // Try examples directory as fallback
        let examples_dir = PathBuf::from("examples");
        if examples_dir.exists() {
            println!("Note: src/ not found, using examples/");
            return generate_docs(Some(examples_dir), Some(out_dir));
        }
        return Err(format!("Source directory not found: {:?}", src_dir));
    }

    // Use the existing docs module
    let count = docs::generate_docs(&src_dir, &out_dir)?;

    if count == 0 {
        println!("No documentation comments found.");
        println!("Add documentation comments (//) to functions and actors.");
    } else {
        println!("Generated {} documentation file(s)", count);
    }

    // Generate index
    let index_path = out_dir.join("README.md");
    let index_content = format!("# {}\n\nGenerated documentation for Pulse project.\n\n## Modules\n\nSee individual module files for details.\n", 
        find_project_name().unwrap_or_else(|_| "Pulse Project".to_string()));
    fs::write(&index_path, index_content).map_err(|e| format!("Failed to write index: {}", e))?;

    println!();
    println!("===========================================");
    println!("Documentation generated successfully!");
    println!("===========================================");
    println!();
    println!("Output directory: {}", out_dir.display());

    Ok(())
}

fn find_project_name() -> Result<String, String> {
    let project_dir = find_project_root()?;
    let manifest = package::Manifest::load(&project_dir)?;
    Ok(manifest.package.name)
}
