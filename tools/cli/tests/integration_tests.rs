use pulse_ast::error::PulseError;
use pulse_ast::value::ActorId;
use pulse_compiler::compiler::compile;
use pulse_vm::vm::{VMStatus, VM};
use std::fs;
use std::path::PathBuf;

/// Helper: compile a .pulse file, run to completion, expect Halted status
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
                VMStatus::Halted => (), // Success
                VMStatus::Error(e) => panic!("VM error running {:?}: {}", path, e),
                other => panic!("Unexpected VM status running {:?}: {:?}", path, other),
            }
        }
        Err(e) => panic!("Compilation failed for {:?}: {:?}", path, e),
    }
}

/// Helper: compile inline source, run to completion, expect Halted status
async fn run_source_expect_success(source: &str) {
    match compile(source, None) {
        Ok(chunk) => {
            let mut vm = VM::new(chunk, ActorId::new(0, 0), None);
            let result = vm.run(usize::MAX).await;
            match result {
                VMStatus::Halted => (),
                VMStatus::Error(e) => panic!("VM error: {}", e),
                other => panic!("Unexpected VM status: {:?}", other),
            }
        }
        Err(e) => panic!("Compilation failed: {:?}", e),
    }
}

/// Helper: compile inline source, expect a RuntimeError
async fn run_source_expect_error(source: &str) -> PulseError {
    match compile(source, None) {
        Ok(chunk) => {
            let mut vm = VM::new(chunk, ActorId::new(0, 0), None);
            let result = vm.run(usize::MAX).await;
            match result {
                VMStatus::Error(e) => e,
                VMStatus::Halted => panic!("Expected error but got Halted"),
                other => panic!("Expected error but got: {:?}", other),
            }
        }
        Err(e) => e,
    }
}

// ============================================================
// Integration tests: compile .pulse files and run to completion
// ============================================================

#[tokio::test]
async fn test_arithmetic() {
    run_file_expect_success("../../tests/integration/arithmetic.pulse").await;
}

#[tokio::test]
async fn test_variables() {
    run_file_expect_success("../../tests/integration/variables.pulse").await;
}

#[tokio::test]
async fn test_control_flow() {
    run_file_expect_success("../../tests/integration/control_flow.pulse").await;
}

#[tokio::test]
async fn test_functions() {
    run_file_expect_success("../../tests/integration/functions.pulse").await;
}

#[tokio::test]
async fn test_data_structures() {
    run_file_expect_success("../../tests/integration/data_structures.pulse").await;
}

#[tokio::test]
async fn test_strings() {
    run_file_expect_success("../../tests/integration/strings.pulse").await;
}

#[tokio::test]
async fn test_classes() {
    run_file_expect_success("../../tests/integration/classes.pulse").await;
}

#[tokio::test]
async fn test_error_handling() {
    run_file_expect_success("../../tests/integration/error_handling.pulse").await;
}

#[tokio::test]
async fn test_language_full_coverage() {
    run_file_expect_success("../../tests/integration/language_full_coverage.pulse").await;
}

#[tokio::test]
async fn test_examples_comprehensive() {
    run_file_expect_success("../../examples/comprehensive.pulse").await;
}

// ============================================================
// Edge case tests: specific VM behaviors
// ============================================================

#[tokio::test]
async fn test_basic_int_arithmetic() {
    run_source_expect_success("print(2 + 3);").await;
    run_source_expect_success("print(10 - 7);").await;
    run_source_expect_success("print(6 * 7);").await;
    run_source_expect_success("print(10 / 3);").await;
}

#[tokio::test]
async fn test_boolean_operations() {
    run_source_expect_success("print(true and true);").await;
    run_source_expect_success("print(true or false);").await;
    run_source_expect_success("print(!false);").await;
    run_source_expect_success("print(1 == 1);").await;
    run_source_expect_success("print(1 != 2);").await;
}

#[tokio::test]
async fn test_dsa_bitwise_and_power_operations() {
    run_source_expect_success("print(5 & 3);").await;
    run_source_expect_success("print(5 | 2);").await;
    run_source_expect_success("print(5 ^ 1);").await;
    run_source_expect_success("print(~1);").await;
    run_source_expect_success("print(1 << 3);").await;
    run_source_expect_success("print(8 >> 1);").await;
    run_source_expect_success("print(2 ** 10);").await;
}

#[tokio::test]
async fn test_comparison_operators() {
    run_source_expect_success("print(5 > 3);").await;
    run_source_expect_success("print(3 < 5);").await;
    run_source_expect_success("print(1 == 1);").await;
    run_source_expect_success("print(1 != 2);").await;
}

#[tokio::test]
async fn test_undefined_variable_error() {
    let err = run_source_expect_error("print(undefined_var);").await;
    assert!(
        matches!(err, PulseError::CompileError(_, _)),
        "Expected CompileError, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_type_mismatch_arithmetic() {
    let err = run_source_expect_error("let x = true + 1;").await;
    assert!(
        matches!(err, PulseError::CompileError(_, _)),
        "Expected CompileError, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_index_out_of_bounds() {
    let err = run_source_expect_error("let l = [1, 2, 3]; print(l[10]);").await;
    assert!(
        matches!(err, PulseError::RuntimeError(_)),
        "Expected RuntimeError, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_empty_program() {
    // An empty program or just whitespace should compile and run
    run_source_expect_success("").await;
}
