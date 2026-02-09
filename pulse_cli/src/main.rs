mod repl;
mod package;
mod docs;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Build the project
    Build,
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { file }) => run_file(file),
        Some(Commands::Repl) => repl::start(),
        Some(Commands::Demo) => run_demo(),
        Some(Commands::Test { path }) => run_tests(path),
        Some(Commands::Init { path, name }) => cmd_init(path, name),
        Some(Commands::Add { package, version }) => cmd_add(package, version),
        Some(Commands::Build) => cmd_build(),
        Some(Commands::Doc { source, output }) => cmd_doc(source, output),
        None => {
            // Default to REPL if no command given
            println!("--- Pulse Language v0.1 ---");
            println!("Use 'pulse run <file>' to run a file, or 'pulse repl' for interactive mode.");
            println!("Starting REPL...\n");
            repl::start();
        }
    }
}

fn cmd_doc(source: Option<PathBuf>, output: Option<PathBuf>) {
    let src_dir = source.unwrap_or_else(|| PathBuf::from("src"));
    let out_dir = output.unwrap_or_else(|| PathBuf::from("docs"));
    
    println!("Generating documentation...");
    println!("  Source: {}", src_dir.display());
    println!("  Output: {}", out_dir.display());
    
    match docs::generate_docs(&src_dir, &out_dir) {
        Ok(count) => {
            if count == 0 {
                println!("\nNo documented items found.");
            } else {
                println!("\n✓ Generated {} documentation file(s)", count);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_init(path: Option<PathBuf>, name: Option<String>) {
    let dir = path.unwrap_or_else(|| PathBuf::from("."));
    
    println!("Initializing Pulse project...");
    
    match package::init_project(&dir, name) {
        Ok(()) => {
            println!("✓ Created Pulse.toml");
            println!("✓ Created src/main.pulse");
            println!("\nProject initialized! Run 'pulse run src/main.pulse' to start.");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_add(pkg: String, version: Option<String>) {
    let dir = PathBuf::from(".");
    
    match package::add_dependency(&dir, &pkg, version.as_deref()) {
        Ok(()) => {
            println!("✓ Added dependency: {}", pkg);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_build() {
    let dir = PathBuf::from(".");
    
    // Load manifest
    let manifest = match package::Manifest::load(&dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Is this a Pulse project? Run 'pulse init' first.");
            std::process::exit(1);
        }
    };
    
    println!("Building {}...", manifest.package.name);
    
    // Compile entry point
    let entry_path = dir.join(&manifest.package.entry);
    let source = match std::fs::read_to_string(&entry_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", entry_path.display(), e);
            std::process::exit(1);
        }
    };
    
    match pulse_compiler::compile(&source, Some(entry_path.to_string_lossy().to_string())) {
        Ok(_chunk) => {
            println!("✓ Build successful!");
        }
        Err(e) => {
            eprintln!("Compile error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_tests(path: Option<PathBuf>) {
    use std::fs;
    use std::time::Instant;
    
    let test_path = path.unwrap_or_else(|| PathBuf::from("."));
    
    println!("--- Pulse Test Runner ---\n");
    
    // Collect test files
    let test_files: Vec<PathBuf> = if test_path.is_file() {
        vec![test_path]
    } else {
        collect_test_files(&test_path)
    };
    
    if test_files.is_empty() {
        println!("No test files found (looking for *_test.pulse or test_*.pulse)");
        return;
    }
    
    println!("Found {} test file(s)\n", test_files.len());
    
    let mut passed = 0;
    let mut failed = 0;
    let start = Instant::now();
    
    for file in &test_files {
        print!("Testing {}... ", file.display());
        
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                println!("SKIP ({})", e);
                continue;
            }
        };
        
        match pulse_compiler::compile(&source, Some(file.to_string_lossy().to_string())) {
            Ok(chunk) => {
                let mut runtime = pulse_runtime::Runtime::new(1);
                runtime.spawn(chunk, None);
                
                // Run until complete or error
                let mut error_msg = None;
                let mut cycles = 0;
                loop {
                    match runtime.step() {
                        true => { cycles = 0; }
                        false => {
                            cycles += 1;
                            if cycles > 3 { break; }
                        }
                    }
                    // Check for runtime errors in actors
                    if let Some(msg) = runtime.get_last_error() {
                        error_msg = Some(msg);
                        break;
                    }
                }
                
                if let Some(msg) = error_msg {
                    println!("FAIL");
                    println!("    Error: {}", msg);
                    failed += 1;
                } else {
                    println!("OK");
                    passed += 1;
                }
            }
            Err(e) => {
                println!("FAIL");
                println!("    Compile error: {}", e);
                failed += 1;
            }
        }
    }
    
    let duration = start.elapsed();
    println!("\n--------------------------");
    println!("Tests: {} passed, {} failed", passed, failed);
    println!("Time: {:.2}s", duration.as_secs_f64());
    
    if failed > 0 {
        std::process::exit(1);
    }
}

fn collect_test_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if (name.ends_with("_test.pulse") || name.starts_with("test_"))
                        && name.ends_with(".pulse")
                    {
                        files.push(path);
                    }
                }
            } else if path.is_dir() {
                files.extend(collect_test_files(&path));
            }
        }
    }
    files.sort();
    files
}

fn run_file(path: PathBuf) {
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path.display(), e);
            std::process::exit(1);
        }
    };

    println!("--- Pulse Language v0.1 ---");
    println!("Running: {}\n", path.display());

    let mut runtime = pulse_runtime::Runtime::new(1);
    
    match pulse_compiler::compile(&source, Some(path.to_string_lossy().to_string())) {
        Ok(chunk) => {
            runtime.spawn(chunk, None);
            run_scheduler(&mut runtime);
        }
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_demo() {
    use pulse_core::{Chunk, Op, Constant};
    
    println!("--- Pulse Language v0.1 Demo ---");

    let mut runtime = pulse_runtime::Runtime::new(1);

    // Create actor demo bytecode
    let mut chunk = Chunk::new();
    let msg_idx = chunk.add_constant(Constant::String("Hello from Child Actor!".to_string()));
    
    chunk.write(Op::Jump as u8, 1);
    chunk.write(7, 1);
    chunk.write(0, 1);

    for _ in 3..10 {
        chunk.write(Op::Halt as u8, 0);
    }

    chunk.write(Op::Spawn as u8, 2);
    chunk.write(20, 2);
    chunk.write(0, 2);

    chunk.write(Op::Const as u8, 2);
    chunk.write(msg_idx as u8, 2);

    chunk.write(Op::Send as u8, 2);
    chunk.write(Op::Halt as u8, 2);

    for _ in 17..20 {
        chunk.write(Op::Halt as u8, 0);
    }

    chunk.write(Op::Receive as u8, 3);
    chunk.write(Op::Print as u8, 3);
    chunk.write(Op::Halt as u8, 3);

    let pid = runtime.spawn(chunk, None);
    println!("Spawned Root Actor: {:?}", pid);

    // Standard library demo
    let source = r#"
        println("--- Standard Library Demo ---");
        let start = clock();
        println("Start time:", start);

        let msg = "Hello " + "World!";
        println(msg);

        let end = clock();
        println("End time:", end);
        println("Duration:", end - start);
    "#;

    println!("\nCompiling Std Lib Demo...");
    match pulse_compiler::compile(source, None) {
        Ok(chunk) => {
            let pid = runtime.spawn(chunk, None);
            println!("Spawned Std Lib Demo Actor: {:?}", pid);
        }
        Err(e) => println!("Compilation failed: {}", e),
    }

    run_scheduler(&mut runtime);
}

fn run_scheduler(runtime: &mut pulse_runtime::Runtime) {
    println!("Running scheduler...");
    let mut idle_cycles = 0;
    loop {
        if !runtime.step() {
            idle_cycles += 1;
            if idle_cycles > 5 {
                println!("System idle. Exiting.");
                break;
            }
        } else {
            idle_cycles = 0;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
