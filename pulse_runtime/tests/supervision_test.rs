use pulse_core::{Chunk, Constant, Op, ActorId, Value};
use pulse_runtime::runtime::Runtime;
use pulse_runtime::mailbox::Message;
use pulse_runtime::actor::ActorStatus;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};

fn create_child_actor_code() -> Chunk {
    // Child code:
    // PUSH "Child Running"
    // PRINT
    // RECEIVE (Wait for message)
    // HALT
    let mut chunk = Chunk::new();
    let msg_idx = chunk.add_constant(Constant::String("Child Running".to_string()));
    
    chunk.write(Op::Const as u8, 1);
    chunk.write(msg_idx as u8, 1);
    chunk.write(Op::Print as u8, 1);
    chunk.write(Op::Receive as u8, 2);
    chunk.write(Op::Halt as u8, 3);
    chunk
}

#[test]
fn test_spawn_link_and_failure_propagation() {
    let mut runtime = Runtime::new(1);
    
    // Parent Code:
    // SPAWN_LINK (Child)
    // PUSH "Parent Waiting"
    // PRINT
    // RECEIVE (Wait for exit signal or other message)
    // HALT
    
    let mut chunk = Chunk::new();
    let child_chunk = create_child_actor_code();
    
    // We need to put child code in parent's constants or similar?
    // VM implementation of Spawn/SpawnLink takes an offset to function code in SAME chunk.
    // For this test, we can manually construct a chunk that has parent code + child code.
    
    // 0: JUMP to Parent Start (e.g. 10)
    chunk.write(Op::Jump as u8, 1);
    chunk.write(7, 1); // Offset 7 (to 10)
    chunk.write(0, 1);
    
    // Padding
    for _ in 3..10 { chunk.write(Op::Halt as u8, 0); }
    
    // 10: Parent Start
    // SpawnLink(20) -> Child code at 20
    chunk.write(Op::SpawnLink as u8, 2);
    chunk.write(20, 2); // Low
    chunk.write(0, 2);  // High
    
    // Store Child PID (on stack) into local 0? Or just pop it.
    // Let's keep it on stack.
    
    // Receive (Block parent)
    chunk.write(Op::Receive as u8, 3);
    chunk.write(Op::Halt as u8, 4);
    
    // Padding
    for _ in (chunk.code.len())..20 { chunk.write(Op::Halt as u8, 0); }
    
    // 20: Child Code
    // Child immediately crashes (Divide by zero)
    chunk.write(Op::Const as u8, 5);
    let idx = chunk.add_constant(Constant::Int(1)); // 1
    chunk.write(idx as u8, 5);
    
    chunk.write(Op::Const as u8, 5);
    let idx2 = chunk.add_constant(Constant::Int(0)); // 0
    chunk.write(idx2 as u8, 5);
    
    chunk.write(Op::Div as u8, 5); // 1 / 0 -> Error
    chunk.write(Op::Halt as u8, 6);

    let pid = runtime.spawn(chunk, None);
    
    // Run for a bit
    for _ in 0..20 {
        if !runtime.step() {
            thread::sleep(Duration::from_millis(10));
        }
    }
    
    // Parent should be terminated because linked child crashed
    let parent_vm = runtime.get_actor_vm(pid).unwrap();
    let parent = parent_vm.lock().unwrap();
    assert_eq!(parent.status, ActorStatus::Terminated, "Parent should have terminated due to child crash");
}

#[test]
fn test_monitor_functionality() {
    let mut runtime = Runtime::new(1);
    
    // Monitor Code:
    // SPAWN (Target)
    // MONITOR (Target on stack)
    // RECEIVE (Wait for Down message)
    // PUSH "Received Down"
    // PRINT
    // HALT
    
    let mut chunk = Chunk::new();
    
    // 0: JUMP 20
    chunk.write(Op::Jump as u8, 1);
    chunk.write(17, 1); // 3+17=20 (Wait, 3+17=20? verify)
    chunk.write(0, 1);
    
    // Padding
    for _ in 3..20 { chunk.write(Op::Halt as u8, 0); }
    
    // 20: Monitor Actor (Parent Code)
    // 1. Reserve Local 1 (Local 0 is script closure)
    chunk.write(Op::Unit as u8, 2); // Stack: [Closure, Unit]
    
    // 2. Spawn Target
    chunk.write(Op::Spawn as u8, 2);
    chunk.write(50, 2); // Target code at 50
    chunk.write(0, 2); // Stack: [Closure, Unit, Pid]
    
    // 3. Store Pid in Local 1
    chunk.write(Op::SetLocal as u8, 2);
    chunk.write(1, 2); // Slot 1
    
    // 4. Pop (remove Pid from top, it's safe in Local 1)
    chunk.write(Op::Pop as u8, 2); // Stack: [Closure, Unit]
    
    // 5. Monitor (Get Pid first)
    chunk.write(Op::GetLocal as u8, 2);
    chunk.write(1, 2); // Stack: [Closure, Unit, Pid]
    chunk.write(Op::Monitor as u8, 2); // Pops Pid. Stack: [Closure, Unit]
    
    // 6. Send "Die" to Child
    // Push Target
    chunk.write(Op::GetLocal as u8, 2);
    chunk.write(1, 2); // Stack: [Closure, Unit, Pid]
    // Push Message "Die"
    let msg_idx = chunk.add_constant(Constant::String("Die".to_string()));
    chunk.write(Op::Const as u8, 2);
    chunk.write(msg_idx as u8, 2); // Stack: [..., Pid, "Die"]
    
    // Send
    chunk.write(Op::Send as u8, 2); // Pops "Die", Pid. Stack: [Closure, Unit]
    
    // 7. Receive (Wait for Down)
    chunk.write(Op::Receive as u8, 3);
    
    // 8. Halt
    chunk.write(Op::Halt as u8, 4);
    
    // Padding to 50
    for _ in (chunk.code.len())..50 { chunk.write(Op::Halt as u8, 0); }
    
    // 50: Target Code
    // 1. Receive (Wait for "Die")
    chunk.write(Op::Receive as u8, 5);
    // 2. Halt
    chunk.write(Op::Halt as u8, 5);
    
    let pid = runtime.spawn(chunk, None);
    
    // Run
    for _ in 0..20 {
        if !runtime.step() {
            thread::sleep(Duration::from_millis(10));
        }
    }
    
    let parent_vm = runtime.get_actor_vm(pid).unwrap();
    let parent = parent_vm.lock().unwrap();
    
    assert_eq!(parent.status, ActorStatus::Terminated, "Parent should have processed monitor message and halted");
}
