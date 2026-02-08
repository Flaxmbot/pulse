use pulse_core::{Chunk, Op, Value, Constant};
use pulse_runtime::Runtime;

fn main() {
    println!("--- Pulse Language v0.1 ---");

    // 1. Initialize Runtime (Node ID 1)
    let mut runtime = Runtime::new(1);

    // 2. Create "Hello World" Bytecode
    // Code:
    // PUSH "Hello from Pulse!"
    // PRINT
    // HALT
    // 2. Create Actor Test Bytecode
    // 0: Jump to 10 (Parent)
    // ...
    // 10: Parent: Spawn(20), Push "Hello", Send, Halt
    // ...
    // 20: Child: Receive, Print, Halt

    let mut chunk = Chunk::new();
    let msg_idx = chunk.add_constant(Constant::String("Hello from Child Actor!".to_string()));
    
    // 0: JUMP to 10
    // Op::Jump (1 byte) + Offset (2 bytes) = 3 bytes
    // Current IP (after op) = 1.
    // We read u16. IP becomes 3.
    // We want target 10.
    // 3 + offset = 10 => offset = 7.
    chunk.write(Op::Jump as u8, 1);
    chunk.write(7, 1); // Low byte
    chunk.write(0, 1); // High byte

    // Padding (3..10)
    for _ in 3..10 {
        chunk.write(Op::Halt as u8, 0); 
    }

    // 10: Parent Code
    // Spawn(20)
    chunk.write(Op::Spawn as u8, 2);
    chunk.write(20, 2); // Jump to 20 for child
    chunk.write(0, 2);

    // Stack: [ChildPID]
    // Push "Hello"
    chunk.write(Op::Const as u8, 2);
    chunk.write(msg_idx as u8, 2);

    // Stack: [ChildPID, "Hello"]
    // Send
    chunk.write(Op::Send as u8, 2);

    // IP = 10 + 3(Spawn) + 2(Const) + 1(Send) = 16.
    chunk.write(Op::Halt as u8, 2);
    // IP = 17.

    // Padding (17..20)
    for _ in 17..20 {
        chunk.write(Op::Halt as u8, 0); 
    }

    // 20: Child Code
    chunk.write(Op::Receive as u8, 3);
    chunk.write(Op::Print as u8, 3);
    chunk.write(Op::Halt as u8, 3);

    // 3. Spawn Initial Actor (Parent)
    let pid = runtime.spawn(chunk, None);
    println!("Spawned Root Actor: {:?}", pid);

    // 5. Standard Library Demo (Manual Bytecode for now as we don't have full parser integration in CLI yet)
    // Actually, we DO have full parser integration in `pulse_compiler`.
    // Let's use it!

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
        },
        Err(e) => println!("Compilation failed: {}", e),
    }

    // 6. Test Supervision Features
    let supervision_source = r#"
        println("--- Testing Supervision Features ---");
        // This would test link, monitor, and spawn_link functionality
        // when we have actors implemented in the language
    "#;

    println!("\nCompiling Supervision Test...");
    match pulse_compiler::compile(supervision_source, None) {
        Ok(chunk) => {
            let pid = runtime.spawn(chunk, None);
            println!("Spawned Supervision Test Actor: {:?}", pid);
        },
        Err(e) => println!("Supervision compilation failed: {}", e),
    }


    // 4. Run Scheduler Loop
    println!("Running scheduler...");
    let mut idle_cycles = 0;
    loop {
        if !runtime.step() {
            idle_cycles += 1;
            if idle_cycles > 5 {
                println!("System idle (no actors). Exiting.");
                break;
            }
        } else {
            idle_cycles = 0;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
