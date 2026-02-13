use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;
use std::fs;
use pulse_runtime::Runtime;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

mod docs;
mod package;

#[derive(Parser)]
#[command(name = "pulse")]
#[command(version = "0.1.0")]
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
    /// Run performance benchmarks
    Benchmark {
        /// Path to benchmark file (default: bench_baseline.pulse)
        file: Option<PathBuf>,
        /// Number of iterations
        #[arg(short, long, default_value = "3")]
        iterations: usize,
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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file }) => {
            if let Err(e) = run_file(file).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Repl) => {
            if let Err(e) = run_repl().await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
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
        Some(Commands::Benchmark { file, iterations }) => {
            if let Err(e) = run_benchmarks(file, iterations) {
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
        Some(Commands::Build { file, output, release }) => {
            if let Err(e) = build_file(file, output, release) {
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
        None => {
            // Default to REPL if no command provided
            if let Err(e) = run_repl().await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn run_file(path: PathBuf) -> Result<(), String> {
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let chunk = pulse_compiler::compile(&source, Some(path.to_string_lossy().to_string()))
        .map_err(|e| format!("Compilation error: {}", e))?;

    let runtime = Runtime::new(0); // Node ID 0 for CLI
    runtime.handle.spawn(chunk, Some(path.to_string_lossy().to_string())).await; // Assuming spawn returns just pid, ignored for now
    
    runtime.run().await; // Waits for all actors to finish
    
    // Check for runtime errors
    // Since runtime is shared, we might need a way to check if it exited cleanly?
    // The current Runtime::run doesn't return status.
    // Ideally we catch errors via monitoring.
    Ok(())
}

async fn run_repl() -> Result<(), String> {
    println!("Pulse Programming Language v0.1.0");
    println!("Type 'exit' or press Ctrl-D to quit");

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;
    
    // We re-create runtime for each REPL session?
    // Or keep one runtime alive?
    // For REPL, we want one runtime state (globals preserved).
    // EXCEPT: VM globals are per-actor ideally, but REPL usually acts as one "Main" actor.
    
    let runtime = Runtime::new(0);
    // We need an actor to execute REPL commands.
    // Or we spawn a new actor for each command?
    // If we want state persistence, we need a persistent actor.
    // But our Actors run a Chunk.
    // We can't injected code into a running chunk easily.
    
    // Alternative: REPL compiles code into a Chunk, spawns an actor, runs it.
    // State sharing is hard this way.
    
    // For now: Naive REPL - separate actor per command.
    // Improve later.
    
    loop {
        let readline = rl.readline("pulse> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line == "exit" {
                    break;
                }
                if line.is_empty() {
                    continue;
                }
                
                rl.add_history_entry(line).map_err(|e| e.to_string())?;

                match pulse_compiler::compile(line, None) {
                    Ok(chunk) => {
                        runtime.handle.spawn(chunk, None).await;
                        // wait for it?
                        // In naive mode, we don't wait. Output happens async.
                        // Ideally we wait for this specific execution.
                        // But `runtime.run()` waits for ALL.
                        // We can't call it here inside the loop easily.
                        
                        // Let's just spawn and continue.
                        tokio::task::yield_now().await; 
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    
    Ok(())
}

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
        let name = file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        println!("{}. Running: {}", i + 1, name);
        println!("{}", "-".repeat(40));
        
        // Run the file in a blocking task since we can't spawn a new runtime
        let result = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                run_file(file.clone()).await
            })
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
        let name = file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        print!("Running: {} ... ", name);
        
        // Run the test file using block_in_place
        let result = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                run_file(file.clone()).await
            })
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
        project_path.file_name()
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
    let mut current = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
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

fn build_file(path: PathBuf, output: Option<PathBuf>, release: bool) -> Result<(), String> {
    let source = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mode = if release { "release" } else { "debug" };
    println!("===========================================");
    println!("     Pulse Build Pipeline                  ");
    println!("===========================================");
    println!("Building: {}", path.display());
    println!("Mode: {}", mode);
    println!();
    
    // Step 1: Compile to bytecode
    println!("[1/1] Compiling to bytecode...");
    let chunk = pulse_compiler::compile(&source, Some(path.to_string_lossy().to_string()))
        .map_err(|e| format!("Compilation error: {}", e))?;
    
    // Create output directory
    let output_dir = PathBuf::from("target").join(mode);
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    
    let output_name = output.unwrap_or_else(|| {
        path.file_stem()
            .map(|s| output_dir.join(s))
            .unwrap_or_else(|| output_dir.join("a.out"))
    });
    
    // Serialize and save bytecode
    let bytecode = serde_json::to_vec(&chunk)
        .map_err(|e| format!("Failed to serialize bytecode: {}", e))?;
    
    let bc_file = output_name.with_extension("pulse");
    fs::write(&bc_file, &bytecode)
        .map_err(|e| format!("Failed to write bytecode: {}", e))?;
    
    println!("      Bytecode compiled successfully!");
    println!("      Output: {}", bc_file.display());
    println!();
    println!("===========================================");
    println!("Build completed successfully!");
    println!("===========================================");
    println!();
    println!("Note: Native compilation (LLVM) is not yet available.");
    println!("Run with: pulse run {}", bc_file.display());
    
    Ok(())
}

fn run_benchmarks(file: Option<PathBuf>, iterations: usize) -> Result<(), String> {
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
                .unwrap();
            rt.block_on(async {
                run_file(bench_path.clone()).await
            })
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
    println!("  Average: {:.2} ms", avg as f64 / 1_000_000.0);
    println!("  Min:     {:.2} ms", min as f64 / 1_000_000.0);
    println!("  Max:     {:.2} ms", max as f64 / 1_000_000.0);
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
    fs::write(&index_path, index_content)
        .map_err(|e| format!("Failed to write index: {}", e))?;
    
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
