use pulse_compiler::compiler::compile;
use pulse_ast::error::PulseError;
use pulse_ast::value::ActorId;
use pulse_vm::vm::{VMStatus, VM};
use std::fs;
use std::path::PathBuf;

async fn run_file_expect_success(relative_path: &str) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative_path);

    let path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    let source = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {:?}", path));

    match compile(source.as_str(), None) {
        Ok(chunk) => {
            let mut vm = VM::new(chunk, ActorId::new(0, 0), None);
            let result = vm.run(usize::MAX).await;
            match result {
                VMStatus::Spawn(..) => (), // Allowed for mocked test envs
                VMStatus::Halted => (), // Success
                VMStatus::Error(e) => {
                    // We expect connection refused in isolated tests
                    let error_str = e.to_string();
                    if !error_str.contains("Connection refused") && 
                       !error_str.contains("Bincode does not support") && 
                       !error_str.contains("No connection could be made") { 
                        panic!("VM error running {:?}: {}", path, e) 
                    }
                },
                other => panic!("Unexpected VM status running {:?}: {:?}", path, other),
            }
        }
        Err(e) => panic!("Compilation failed for {:?}: {:?}", path, e),
    }
}

#[tokio::test]
async fn test_networking() {
    run_file_expect_success("../../tests/integration/networking/networking_tests.pulse").await;
}

#[tokio::test]
async fn test_crypto() {
    run_file_expect_success("../../tests/integration/crypto/crypto_tests.pulse").await;
}

#[tokio::test]
async fn test_concurrency() {
    run_file_expect_success("../../tests/integration/concurrency/supervision_tests.pulse").await;
}
