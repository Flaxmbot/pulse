//! Pulse REPL - Interactive Read-Eval-Print-Loop

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use pulse_runtime::Runtime;
use pulse_vm::{VM, VMStatus};
use pulse_core::Chunk;
use std::path::PathBuf;

const PROMPT: &str = "pulse> ";
const CONTINUATION_PROMPT: &str = "...> ";
const HISTORY_FILE: &str = ".pulse_history";

pub async fn start() {
    println!("Pulse REPL v0.1.0");
    println!("Type :help for available commands, :quit to exit.\n");

    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("Failed to initialize REPL: {}", e);
            return;
        }
    };

    // Load history
    let history_path = get_history_path();
    let _ = rl.load_history(&history_path);

    // Persistent runtime and VM across REPL session
    let runtime = Runtime::new(0);
    
    // Create a persistent VM for the REPL environment
    let mut vm = VM::new(
        Chunk::new(), 
        pulse_core::ActorId::new(0, 0), // PID 0,0 for REPL
        Some(runtime.handle.shared_heap())
    );
    
    let mut input_buffer = String::new();

    loop {
        let prompt = if input_buffer.is_empty() { PROMPT } else { CONTINUATION_PROMPT };
        
        match rl.readline(prompt) {
            Ok(line) => {
                // Handle special commands
                if input_buffer.is_empty() && line.starts_with(':') {
                    if handle_command(&line, &runtime, &mut vm).await {
                        continue;
                    } else {
                        break; // :quit was called
                    }
                }

                input_buffer.push_str(&line);
                input_buffer.push('\n');

                // Check if input is complete (balanced braces/parens)
                if !is_complete(&input_buffer) {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(input_buffer.trim());

                // Compile and execute
                execute_input(&input_buffer, &runtime, &mut vm).await;
                input_buffer.clear();
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C - clear current input
                println!("^C");
                input_buffer.clear();
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D - exit
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    let _ = rl.save_history(&history_path);
}

async fn handle_command(cmd: &str, runtime: &Runtime, vm: &mut VM) -> bool {
    let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
    let base_cmd = parts.get(0).map(|s| *s).unwrap_or("");
    
    match base_cmd {
        ":help" | ":h" => {
            println!("Available commands:");
            println!("  :help, :h         Show this help message");
            println!("  :quit, :q         Exit the REPL");
            println!("  :clear, :c        Clear the screen");
            println!("  :gc               Force garbage collection");
            println!("  :actors           List active actors");
            println!("  :globals          List all global variables");
            println!("\nDebug commands:");
            println!("  :stack            Show current stack");
            println!();
            true
        }
        ":quit" | ":q" | ":exit" => {
            println!("Goodbye!");
            false
        }
        ":clear" | ":c" => {
            print!("\x1B[2J\x1B[1;1H"); // ANSI clear screen
            true
        }
        ":gc" => {
            println!("Forcing garbage collection...");
            vm.collect_garbage();
            println!("GC completed.");
            true
        }
        ":actors" => {
            let count = runtime.handle.get_actor_count();
            println!("Active actors: {}", count);
            true
        }
        ":globals" => {
            println!("Global Variables:");
            for (name, value) in &vm.globals {
                print!("  {} = ", name);
                vm.print_value(value);
                println!();
            }
            true
        }
        ":stack" => {
            println!("Stack:");
            for (i, val) in vm.stack.iter().enumerate() {
                print!("  [{}] ", i);
                vm.print_value(val);
                println!();
            }
            true
        }
        _ => {
            println!("Unknown command: {}. Type :help for available commands.", base_cmd);
            true
        }
    }
}

fn is_complete(input: &str) -> bool {
    let mut brace_depth = 0;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => brace_depth += 1,
            '}' if !in_string => brace_depth -= 1,
            '(' if !in_string => paren_depth += 1,
            ')' if !in_string => paren_depth -= 1,
            '[' if !in_string => bracket_depth += 1,
            ']' if !in_string => bracket_depth -= 1,
            _ => {}
        }
    }

    brace_depth <= 0 && paren_depth <= 0 && bracket_depth <= 0 && !in_string
}

async fn execute_input(input: &str, runtime: &Runtime, vm: &mut VM) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return;
    }

    // Wrap expression in print if it doesn't end with semicolon
    // and isn't a statement (let, fn, if, while, etc.)
    let source = if should_print_result(trimmed) {
        format!("print {};", trimmed.trim_end_matches(';'))
    } else {
        trimmed.to_string()
    };

    match pulse_compiler::compile(&source, None) {
        Ok(chunk) => {
            let status = vm.execute_chunk(chunk).await;
            
            // Handle effects (Spawn, Send, etc.) for the REPL actor
            while let VMStatus::Running = status {
                 // In execute_chunk, we call vm.run(usize::MAX), so it should
                 // only return if it halts, errors, or needs an effect.
                 // Actually, VMStatus::Running shouldn't be returned by vm.run(MAX).
                 break;
            }
            
            match status {
                VMStatus::Error(e) => eprintln!("Runtime Error: {}", e),
                VMStatus::Spawn(ip) => {
                    // This is tricky: VM wants to spawn from its CURRENT chunk.
                    // For REPL, we just spawned a temporary chunk.
                    let current_chunk = match vm.get_current_chunk() {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Error getting current chunk: {:?}", e);
                            return;
                        }
                    };
                    // Use runtime.spawn_from_actor if Runtime is from pulse_runtime.
                    // If repl.rs Runtime has handle, use it.
                    // Assuming runtime.spawn_from_actor is correct based on actor.rs
                    // But if original code use runtime.handle, maybe keep it?
                    // Original: runtime.handle.spawn_from_actor
                    // Let's check imports first. If verified, update.
                    // For now, I will use runtime.handle as it was in original.
                    let _ = runtime.handle.spawn_from_actor(current_chunk, ip).await;
                },
                _ => {} // Halted, etc.
            }
            
            // Allow background actors to run a bit
            tokio::task::yield_now().await;
        }
        Err(e) => {
            eprintln!("Compilation Error: {}", e);
        }
    }
}

fn should_print_result(input: &str) -> bool {
    let trimmed = input.trim();
    
    // Don't print for statements
    let statement_starters = ["let ", "fn ", "if ", "while ", "for ", "print ", "send ", 
                               "link ", "monitor ", "spawn_link ", "import ", "try ", "throw "];
    
    for starter in statement_starters {
        if trimmed.starts_with(starter) {
            return false;
        }
    }
    
    // Don't print if it ends with semicolon (explicit statement)
    if trimmed.ends_with(';') {
        return false;
    }
    
    // Don't print if it's a block (probably function def or control flow)
    if trimmed.ends_with('}') {
        return false;
    }
    
    true
}

fn get_history_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(HISTORY_FILE)
    } else {
        PathBuf::from(HISTORY_FILE)
    }
}
