use pulse_vm::vm::{VM, VMStatus};
use pulse_compiler::compiler::compile;
use pulse_core::error::PulseError;
use pulse_core::value::ActorId;
use std::fs;
use std::path::PathBuf;

// Helper for async execution
async fn run_file_async(path: &str) -> Result<VMStatus, PulseError> {
    let source = fs::read_to_string(path).expect("Failed to read file");
    match compile(source.as_str(), None) {
        Ok(chunk) => {
            let mut vm = VM::new(chunk, ActorId::new(0, 0), None);
            let result = vm.run(usize::MAX).await;
            Ok(result)
        },
        Err(e) => Err(e),
    }
}

#[tokio::test]
async fn test_stack_overflow() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../tests/stress_recursion.pulse");
    
    let path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    println!("Testing path: {:?}", path);

    let result = run_file_async(path.to_str().unwrap()).await;
    match result {
        Ok(VMStatus::Error(PulseError::StackOverflow)) => (), // Success
        Ok(status) => panic!("Expected StackOverflow, got {:?}", status),
        Err(e) => panic!("Compilation error or other: {:?}", e),
    }
}

#[tokio::test]
async fn test_actor_limit() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../tests/stress_actors.pulse");
    
    let path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    println!("Testing path: {:?}", path);

    // This test expects to run without crashing.
    let result = run_file_async(path.to_str().unwrap()).await;
    
    match result {
        Ok(VMStatus::Halted) => (),
        Ok(VMStatus::Error(e)) => panic!("Actor stress test failed: {:?}", e),
        Err(e) => panic!("Compilation error: {:?}", e),
        _ => (),
    }
}
