use pulse_compiler::Compiler;
use pulse_core::Op;

#[test]
fn test_compile_spawn_send_receive() {
    let source = r#"
        let child = spawn {
            let msg = receive;
            print msg;
        };
        send child, "Hello";
    "#;

    let chunk = pulse_compiler::compile(source).expect("Compilation failed");

    // Verify opcodes exist
    let code = chunk.code;
    assert!(code.contains(&(Op::Spawn as u8)));
    assert!(code.contains(&(Op::Receive as u8)));
    assert!(code.contains(&(Op::Send as u8)));
    assert!(code.contains(&(Op::Print as u8)));
}
