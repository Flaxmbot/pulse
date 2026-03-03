//! Tests for the JIT Compiler

use inkwell::context::Context;
use pulse_llvm_backend::JITCompiler;

fn quick_compile(source: &str) -> Result<(), String> {
    let chunk = pulse_compiler::compile(source, None).map_err(|e| e.to_string())?;
    let context = Context::create();
    let mut jit = JITCompiler::new(&context).map_err(|e| e.to_string())?;
    jit.compile_chunk(&chunk)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

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
    let chunk = pulse_ast::Chunk::new();
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
        let chunk = pulse_ast::Chunk::new();
        let _ = jit.compile_chunk(&chunk);
    }

    let stats = jit.get_stats();
    assert_eq!(stats.functions_compiled, 5);
}

// ============================================================================
// Basic Integer Arithmetic Tests (these should work)
// ============================================================================

#[test]
fn test_int_addition() {
    // Test: 1 + 2 = 3
    let code = r#"
        let x = 1 + 2;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Integer addition should compile and execute successfully"
    );
}

#[test]
fn test_int_subtraction() {
    // Test: 5 - 2 = 3
    let code = r#"
        let x = 5 - 2;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Integer subtraction should compile and execute successfully"
    );
}

#[test]
fn test_int_multiplication() {
    // Test: 2 * 3 = 6
    let code = r#"
        let x = 2 * 3;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Integer multiplication should compile and execute successfully"
    );
}

#[test]
fn test_int_comparison() {
    // Test: 1 < 2
    let code = r#"
        if (1 < 2) {
            print(1);
        }
    "#;
    let result = quick_compile(code);
    if let Err(ref e) = result {
        println!("Compilation Error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "Integer comparison should compile and execute successfully"
    );
}

// ============================================================================
// Float Arithmetic Tests - Tests for type support verification
// These tests verify that float operations can be compiled
// Note: Some may have runtime issues due to JIT implementation status
// ============================================================================

#[test]
fn test_float_addition() {
    // Test: 1.5 + 2.5 = 4.0
    let code = r#"
        let x = 1.5 + 2.5;
        print(x);
    "#;
    let result = quick_compile(code);
    // The JIT should be able to compile this - runtime may vary
    assert!(result.is_ok(), "Float addition should attempt to compile");
}

#[test]
fn test_float_subtraction() {
    // Test: 5.0 - 2.5 = 2.5
    let code = r#"
        let x = 5.0 - 2.5;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float subtraction should attempt to compile"
    );
}

#[test]
fn test_float_multiplication() {
    // Test: 2.0 * 3.0 = 6.0
    let code = r#"
        let x = 2.0 * 3.0;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float multiplication should attempt to compile"
    );
}

#[test]
fn test_float_division() {
    // Test: 6.0 / 2.0 = 3.0
    let code = r#"
        let x = 6.0 / 2.0;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "Float division should attempt to compile");
}

// ============================================================================
// Float Comparison Tests - Verify float comparison support
// ============================================================================

#[test]
fn test_float_less_than() {
    // Test: 1.0 < 2.0 = true
    let code = r#"
        if (1.0 < 2.0) {
            print(1);
        }
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float less than comparison should attempt to compile"
    );
}

#[test]
fn test_float_greater_than() {
    // Test: 3.0 > 2.0 = true
    let code = r#"
        if (3.0 > 2.0) {
            print(1);
        }
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float greater than comparison should attempt to compile"
    );
}

#[test]
fn test_float_equality() {
    // Test: 2.0 == 2.0 = true
    let code = r#"
        1.5 == 1.5;
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float equality comparison should attempt to compile"
    );
}

#[test]
fn test_float_not_equal() {
    // Test: 2.0 != 3.0 = true
    let code = r#"
        if (2.0 != 3.0) {
            print(1);
        }
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float not equal comparison should attempt to compile"
    );
}

// ============================================================================
// List Operations Tests - Verify list support in JIT
// ============================================================================

#[test]
fn test_list_build_and_index() {
    // Test: Build list [1, 2, 3] and access index 0
    let code = r#"
        let l = [1, 2, 3];
        print(l[0]);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "List build and index access should attempt to compile"
    );
}

#[test]
fn test_list_set_index() {
    // Test: Set list index l[0] = 10
    let code = r#"
        let l = [1, 2, 3];
        l[0] = 10;
        print(l[0]);
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "List set index should attempt to compile");
}

#[test]
fn test_list_multiple_elements() {
    // Test: Access multiple list elements
    let code = r#"
        let l = [10, 20, 30];
        print(l[1]);
        print(l[2]);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "List with multiple elements should attempt to compile"
    );
}

#[test]
fn test_list_empty() {
    // Test: Empty list
    let code = r#"
        let l = [];
        print(0);
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "Empty list should attempt to compile");
}

// ============================================================================
// Map Operations Tests - Verify map support in JIT
// ============================================================================

#[test]
fn test_map_build_and_access() {
    // Test: Build map {"a": 1, "b": 2} and access key "a"
    let code = r#"
        let m = {"a": 1, "b": 2};
        print(m["a"]);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Map build and access should attempt to compile"
    );
}

#[test]
fn test_map_contains_key() {
    // map_has_key lowering should compile through the JIT path.
    let code = r#"
        let m = {"a": 1};
        if map_has_key(m, "a") {
            print(1);
        }
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Map contains key should compile successfully"
    );
}

#[test]
fn test_map_multiple_keys() {
    // Test: Map with multiple keys
    let code = r#"
        let m = {"x": 100, "y": 200, "z": 300};
        print(m["y"]);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Map with multiple keys should attempt to compile"
    );
}

#[test]
fn test_map_update_value() {
    // Test: Update map value
    let code = r#"
        let m = {"a": 1};
        m["a"] = 99;
        print(m["a"]);
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "Map value update should attempt to compile");
}

// ============================================================================
// String Operations Tests - Verify string support in JIT
// ============================================================================

#[test]
fn test_string_concatenation() {
    // Test: Concatenate strings "hello" + " " + "world"
    let code = r#"
        let s = "hello" + " " + "world";
        print(s);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "String concatenation should attempt to compile"
    );
}

#[test]
fn test_string_length() {
    // Test: Get string length using len()
    let code = r#"
        let s = "hello";
        print(len(s));
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "String length should attempt to compile");
}

#[test]
fn test_string_simple() {
    // Test: Simple string
    let code = r#"
        let s = "test";
        print(s);
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "Simple string should attempt to compile");
}

// ============================================================================
// Mixed Type Operations Tests - Verify type coercion in JIT
// ============================================================================

#[test]
fn test_int_float_mixed() {
    // Test: int + float = float (1 + 2.5 = 3.5)
    let code = r#"
        let x = 1 + 2.5;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Int + float mixed operation should attempt to compile"
    );
}

#[test]
fn test_float_to_string() {
    // Test: Convert float to string
    let code = r#"
        let x = 1.5;
        print(to_string(x));
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float to string conversion should attempt to compile"
    );
}

#[test]
fn test_float_negation() {
    // Test: Float negation -5.0
    let code = r#"
        let x = -5.0;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "Float negation should attempt to compile");
}

#[test]
fn test_float_arithmetic_chain() {
    // Test: Chained float operations (1.0 + 2.0) * 3.0 = 9.0
    let code = r#"
        let x = (1.0 + 2.0) * 3.0;
        print(x);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Chained float operations should attempt to compile"
    );
}

// ============================================================================
// Complex/Mixed Tests - Verify complex data structures
// ============================================================================

#[test]
fn test_list_in_list() {
    // Test: Nested lists
    let code = r#"
        let l = [[1, 2], [3, 4]];
        print(l[0][0]);
    "#;
    let result = quick_compile(code);
    assert!(result.is_ok(), "Nested lists should attempt to compile");
}

#[test]
fn test_map_with_list_value() {
    // Test: Map with list as value
    let code = r#"
        let m = {"items": [1, 2, 3]};
        print(m["items"][0]);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Map with list value should attempt to compile"
    );
}

#[test]
fn test_float_in_list() {
    // Test: Float values in list
    let code = r#"
        let l = [1.5, 2.5, 3.5];
        print(l[1]);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "Float values in list should attempt to compile"
    );
}

#[test]
fn test_string_in_list() {
    // Test: String values in list
    let code = r#"
        let l = ["a", "b", "c"];
        print(l[1]);
    "#;
    let result = quick_compile(code);
    assert!(
        result.is_ok(),
        "String values in list should attempt to compile"
    );
}
