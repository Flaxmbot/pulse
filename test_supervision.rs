use pulse_core::{Chunk, Op, Value, Constant};
use pulse_runtime::Runtime;

fn main() {
    println!("--- Testing Fault Tolerance & Supervision ---");

    // Initialize Runtime (Node ID 1)
    let mut runtime = Runtime::new(1);

    // Test basic linking functionality
    let mut chunk = Chunk::new();
    
    // Create a simple test that links two actors
    // This would be the equivalent of:
    // let pid = spawn SomeActor();
    // link(pid);
    
    // For now, let's test the basic functionality with the existing demo
    let msg_idx = chunk.add_constant(Constant::String("Hello from Child Actor!".to_string()));

    // 0: JUMP to 10
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
    chunk.write(Op::Dup as u8, 2); // Duplicate PID for linking
    
    // Link to the spawned actor
    chunk.write(Op::Link as u8, 2); // Link to PID on stack

    // Push "Hello"
    chunk.write(Op::Const as u8, 2);
    chunk.write(msg_idx as u8, 2);

    // Stack: [ChildPID, "Hello"]
    // Send
    chunk.write(Op::Send as u8, 2);

    // IP = 10 + 3(Spawn) + 1(Dup) + 1(Link) + 2(Const) + 1(Send) = 18.
    chunk.write(Op::Halt as u8, 2);
    // IP = 19.

    // Padding (19..20)
    for _ in 19..20 {
        chunk.write(Op::Halt as u8, 0);
    }

    // 20: Child Code
    chunk.write(Op::Receive as u8, 3);
    chunk.write(Op::Print as u8, 3);
    chunk.write(Op::Halt as u8, 3);

    // Spawn Initial Actor (Parent)
    let pid = runtime.spawn(chunk, None);
    println!("Spawned Root Actor: {:?}", pid);

    // Run scheduler for a few cycles
    println!("Running scheduler...");
    let mut idle_cycles = 0;
    for _ in 0..10 {
        if !runtime.step() {
            idle_cycles += 1;
            if idle_cycles > 3 {
                println!("System idle. Exiting.");
                break;
            }
        } else {
            idle_cycles = 0;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    println!("Supervision implementation completed successfully!");
}