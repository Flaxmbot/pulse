//! End-to-end AOT compilation test
//!
//! Verifies the full pipeline: .pulse source → compile → AOT → object file

use inkwell::context::Context;
use pulse_llvm_backend::LLVMBackend;

fn compile_source(source: &str) -> pulse_ast::Chunk {
    pulse_compiler::compile(source, Some("e2e_test.pulse".to_string()))
        .expect("Failed to compile source")
}

#[test]
fn test_e2e_simple_assignment() {
    let source = "let x = 42;\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let func = backend.compile_chunk(&chunk);
    assert!(func.is_ok(), "AOT compile failed: {:?}", func.err());

    let main = backend.generate_main_entry();
    assert!(main.is_ok(), "Main entry failed: {:?}", main.err());
}

#[test]
fn test_e2e_arithmetic_expression() {
    let source = "let a = 10;\nlet b = 20;\nlet c = a + b;\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let func = backend.compile_chunk(&chunk);
    assert!(func.is_ok(), "AOT compile failed: {:?}", func.err());
}

#[test]
fn test_e2e_print_statement() {
    let source = "println(42);\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let func = backend.compile_chunk(&chunk);
    assert!(func.is_ok(), "AOT compile failed: {:?}", func.err());
}

#[test]
fn test_e2e_object_file_output() {
    let source = "let x = 1 + 2;\nprintln(x);\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let _ = backend.compile_chunk(&chunk).unwrap();
    let _ = backend.generate_main_entry().unwrap();

    let tmp = std::env::temp_dir().join("pulse_e2e_test.o");
    let result = backend.emit_object_file(&tmp, inkwell::OptimizationLevel::Default);
    assert!(
        result.is_ok(),
        "Object file emission failed: {:?}",
        result.err()
    );
    assert!(tmp.exists(), "Object file not created");

    // Verify file is non-empty
    let metadata = std::fs::metadata(&tmp).unwrap();
    assert!(metadata.len() > 0, "Object file is empty");

    // Cleanup
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_e2e_ir_output() {
    let source = "let x = 42;\nprintln(x);\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let _ = backend.compile_chunk(&chunk).unwrap();
    let _ = backend.generate_main_entry().unwrap();

    let tmp = std::env::temp_dir().join("pulse_e2e_test.ll");
    let result = backend.emit_ir(&tmp);
    assert!(result.is_ok(), "IR emission failed: {:?}", result.err());
    assert!(tmp.exists(), "IR file not created");

    // Read and verify IR contains expected elements
    let ir_content = std::fs::read_to_string(&tmp).unwrap();
    assert!(
        ir_content.contains("pulse_main"),
        "IR missing pulse_main function"
    );
    assert!(ir_content.contains("main"), "IR missing main entry point");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_e2e_string_constant() {
    let source = "let name = \"Pulse\";\nprintln(name);\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let func = backend.compile_chunk(&chunk);
    assert!(
        func.is_ok(),
        "String constant AOT compile failed: {:?}",
        func.err()
    );
}

#[test]
fn test_e2e_boolean_logic() {
    let source = "let a = true;\nlet b = false;\nlet c = a and b;\nlet d = a or b;\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let func = backend.compile_chunk(&chunk);
    assert!(
        func.is_ok(),
        "Boolean logic AOT compile failed: {:?}",
        func.err()
    );
}

#[test]
fn test_e2e_comparison_ops() {
    let source = "let x = 10;\nlet y = 20;\nlet lt = x < y;\nlet gt = x > y;\nlet eq = x == y;\n";
    let chunk = compile_source(source);

    let context = Context::create();
    let mut backend = LLVMBackend::new(&context).unwrap();

    let func = backend.compile_chunk(&chunk);
    assert!(
        func.is_ok(),
        "Comparison AOT compile failed: {:?}",
        func.err()
    );
}
