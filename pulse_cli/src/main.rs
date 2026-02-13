use clap::{Parser, Subcommand};
use std::path::PathBuf;
use pulse_compiler;
use pulse_core::Chunk;
use pulse_runtime::Runtime;
use pulse_core::Value;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

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
        /// Path to test file or directory (default: current dir)
        path: Option<PathBuf>,
    },
    /// Run performance benchmarks
    Benchmark,
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
            run_tests(path);
        }
        Some(Commands::Benchmark) => {
            todo!("Benchmark command not yet implemented");
        }
        Some(Commands::Init { path, name }) => {
            init_project(path, name);
        }
        Some(Commands::Add { package, version }) => {
            add_dependency(package, version);
        }
        Some(Commands::Build { file, output }) => {
            if let Err(e) = build_file(file, output) {
                eprintln!("Build Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Doc { source, output }) => {
            generate_docs(source, output);
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
    println!("Running Demo...");
    // TODO: Implement demo using async runtime
}

fn run_tests(path: Option<PathBuf>) {
    println!("Running Tests in {:?}", path);
}

fn init_project(path: Option<PathBuf>, name: Option<String>) {
    println!("Initializing project...");
}

fn add_dependency(package: String, version: Option<String>) {
    println!("Adding dependency {} {:?}", package, version);
}

fn build_file(path: PathBuf, output: Option<PathBuf>) -> Result<(), String> {
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut parser = pulse_compiler::ParserV2::new(&source);
    let script = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let context = inkwell::context::Context::create();
    let mut codegen = pulse_compiler::LLVMCodegen::new(&context, "main_module");
    codegen.gen_script(&script).map_err(|e| format!("Codegen error: {}", e))?;

    let output_name = output.unwrap_or_else(|| path.with_extension("exe"));
    let ll_file = path.with_extension("ll");
    
    // For now, we write the LLVM IR to a file and call clang
    std::fs::write(&ll_file, codegen.module.print_to_string().to_string())
        .map_err(|e| format!("Failed to write LLVM IR: {}", e))?;

    println!("Compiling {} to native executable {}...", path.display(), output_name.display());

    // Call clang to compile and link
    // We need to link with pulse_aot_runtime
    // Assume pulse_aot_runtime.lib is in the target/debug directory
    // This is a bit brittle, but works for development
    let status = std::process::Command::new("clang")
        .arg(ll_file.to_str().unwrap())
        .arg("-o")
        .arg(output_name.to_str().unwrap())
        .arg("-lpulse_aot_runtime")
        .arg("-L./target/debug")
        .status()
        .map_err(|e| format!("Failed to invoke clang: {}", e))?;

    if !status.success() {
        return Err("Clang failed to build executable".into());
    }

    println!("Build successful: {}", output_name.display());
    Ok(())
}

fn build_project() {
    println!("Building project...");
}

fn generate_docs(source: Option<PathBuf>, output: Option<PathBuf>) {
    println!("Generating docs...");
}
