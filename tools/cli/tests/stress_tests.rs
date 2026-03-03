use pulse_ast::error::PulseError;
use pulse_ast::value::ActorId;
use pulse_compiler::compiler::compile;
use pulse_vm::vm::{VMStatus, VM};
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
        }
        Err(e) => Err(e),
    }
}

#[tokio::test]
async fn test_stack_overflow() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/stress_recursion.pulse");

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
    path.push("../../tests/stress_actors.pulse");

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

// Helper to compile and run inline source
async fn run_source_async(source: &str) -> Result<VMStatus, PulseError> {
    match compile(source, None) {
        Ok(chunk) => {
            let mut vm = VM::new(chunk, ActorId::new(0, 0), None);
            let result = vm.run(usize::MAX).await;
            Ok(result)
        }
        Err(e) => Err(e),
    }
}

#[tokio::test]
async fn test_integer_overflow() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/overflow_test.pulse");

    let path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    let result = run_file_async(path.to_str().unwrap()).await;
    match result {
        Ok(VMStatus::Error(PulseError::RuntimeError(msg))) => {
            assert!(
                msg.contains("overflow"),
                "Expected overflow error, got: {}",
                msg
            );
        }
        Ok(status) => panic!("Expected overflow RuntimeError, got {:?}", status),
        Err(e) => panic!("Compilation error: {:?}", e),
    }
}

#[tokio::test]
async fn test_division_by_zero() {
    let result = run_source_async("let x = 1 / 0;").await;
    match result {
        Ok(VMStatus::Error(PulseError::RuntimeError(msg))) => {
            assert!(
                msg.contains("zero"),
                "Expected division by zero error, got: {}",
                msg
            );
        }
        Ok(status) => panic!("Expected division by zero error, got {:?}", status),
        Err(e) => panic!("Compilation error: {:?}", e),
    }
}

#[tokio::test]
async fn test_float_division_by_zero() {
    let result = run_source_async("let x = 1.0 / 0.0;").await;
    match result {
        Ok(VMStatus::Error(PulseError::RuntimeError(msg))) => {
            assert!(
                msg.contains("zero"),
                "Expected float division by zero error, got: {}",
                msg
            );
        }
        Ok(status) => panic!("Expected float division by zero error, got {:?}", status),
        Err(e) => panic!("Compilation error: {:?}", e),
    }
}

#[tokio::test]
async fn test_negate_min_int() {
    // -(-9223372036854775808) should overflow
    let result = run_source_async("let x = -(-9223372036854775807 - 1);").await;
    match result {
        Ok(VMStatus::Error(PulseError::RuntimeError(msg))) => {
            assert!(
                msg.contains("overflow"),
                "Expected overflow error, got: {}",
                msg
            );
        }
        Ok(status) => panic!("Expected overflow error, got {:?}", status),
        Err(e) => panic!("Compilation error: {:?}", e),
    }
}

#[tokio::test]
async fn test_modulo_by_zero() {
    let result = run_source_async("let x = 10 % 0;").await;
    match result {
        Ok(VMStatus::Error(PulseError::RuntimeError(msg))) => {
            assert!(
                msg.contains("zero"),
                "Expected modulo by zero error, got: {}",
                msg
            );
        }
        Ok(status) => panic!("Expected modulo by zero error, got {:?}", status),
        Err(e) => panic!("Compilation error: {:?}", e),
    }
}
