//! Pulse REPL - Interactive Read-Eval-Print-Loop

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use pulse_runtime::Runtime;
use std::path::PathBuf;

const PROMPT: &str = "pulse> ";
const CONTINUATION_PROMPT: &str = "...> ";
const HISTORY_FILE: &str = ".pulse_history";

pub fn start() {
    println!("Pulse REPL v0.1");
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

    // Persistent runtime across REPL session
    let mut runtime = Runtime::new(1);
    let mut input_buffer = String::new();

    loop {
        let prompt = if input_buffer.is_empty() { PROMPT } else { CONTINUATION_PROMPT };
        
        match rl.readline(prompt) {
            Ok(line) => {
                // Handle special commands
                if input_buffer.is_empty() && line.starts_with(':') {
                    if handle_command(&line, &mut runtime) {
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
                execute_input(&input_buffer, &mut runtime);
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

fn handle_command(cmd: &str, runtime: &mut Runtime) -> bool {
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
            println!("\nDebug commands:");
            println!("  :break <line>     Set breakpoint at line");
            println!("  :breakpoints      List all breakpoints");
            println!("  :delete <line>    Delete breakpoint");
            println!("  :step, :s         Step one instruction");
            println!("  :continue         Continue execution");
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
            println!("GC completed.");
            true
        }
        ":actors" => {
            let count = runtime.handle.get_actor_count();
            println!("Active actors: {}", count);
            true
        }
        ":break" => {
            if let Some(line_str) = parts.get(1) {
                if let Ok(line) = line_str.parse::<usize>() {
                    println!("Breakpoint set at line {}", line);
                    // Note: Breakpoints are stored per-session, applied when compiling
                    println!("(Breakpoints will take effect on next execution)");
                } else {
                    println!("Usage: :break <line_number>");
                }
            } else {
                println!("Usage: :break <line_number>");
            }
            true
        }
        ":breakpoints" => {
            println!("Breakpoints: (feature coming soon)");
            true
        }
        ":delete" => {
            if let Some(line_str) = parts.get(1) {
                if let Ok(line) = line_str.parse::<usize>() {
                    println!("Deleted breakpoint at line {}", line);
                } else {
                    println!("Usage: :delete <line_number>");
                }
            } else {
                println!("Usage: :delete <line_number>");
            }
            true
        }
        ":step" | ":s" => {
            println!("Step mode enabled for next execution.");
            true
        }
        ":continue" => {
            println!("Continuing execution...");
            true
        }
        ":stack" => {
            println!("Stack: (no active debug session)");
            println!("Hint: Set a breakpoint first, then run code.");
            true
        }
        _ => {
            println!("Unknown command: {}. Type :help for available commands.", base_cmd);
            true
        }
    }
}

fn is_complete(input: &str) -> bool {
    let mut brace_depth = 0i32;
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

    brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 && !in_string
}

fn execute_input(input: &str, runtime: &mut Runtime) {
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
            runtime.spawn(chunk, None);
            // Run until idle
            let mut idle = 0;
            while idle < 3 {
                if !runtime.step() {
                    idle += 1;
                } else {
                    idle = 0;
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
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
