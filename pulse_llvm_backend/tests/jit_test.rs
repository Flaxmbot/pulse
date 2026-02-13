//! Tests for the JIT Compiler

use pulse_llvm_backend::JITCompiler;
use inkwell::context::Context;

#[test]
fn test_jit_initialization() {
    let context = Context::create();
    let jit = JITCompiler::new(&context);
    assert!(jit.is_ok());
}

#[test]
fn test_module_creation() {
    let context = Context::create();
    let jit = JITCompiler::new(&context).unwrap();
    let module = jit.get_module();
    assert!(!module.get_name().is_empty());
}

#[test]
fn test_empty_chunk() {
    let context = Context::create();
    let mut jit = JITCompiler::new(&context).unwrap();
    let chunk = pulse_core::Chunk::new();
    let result = jit.compile_chunk(&chunk);
    assert!(result.is_ok());
}

#[test]
fn test_stats_initialization() {
    let context = Context::create();
    let jit = JITCompiler::new(&context).unwrap();
    let stats = jit.get_stats();
    assert_eq!(stats.instructions_compiled, 0);
}

#[test]
fn test_multiple_functions() {
    let context = Context::create();
    let mut jit = JITCompiler::new(&context).unwrap();
    
    for _ in 0..5 {
        let chunk = pulse_core::Chunk::new();
        let _ = jit.compile_chunk(&chunk);
    }
    
    let stats = jit.get_stats();
    assert_eq!(stats.functions_compiled, 5);
}
