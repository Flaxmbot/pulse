use pulse_compiler::Compiler;
use pulse_core::Op;

#[test]
fn test_compile_for_loop() {
    let source = r#"
        for (let i = 0; i < 5; i = i + 1) {
            print i;
        }
    "#;
    let chunk = pulse_compiler::compile(source).expect("Compilation failed");
    let code = chunk.code;
    assert!(code.contains(&(Op::Loop as u8)));
}

#[test]
fn test_compile_break() {
    let source = r#"
        while (true) {
            break;
        }
    "#;
    let mut compiler = Compiler::new(source);
    let chunk = compiler.compile().expect("Compilation failed");
    // Should compile without error
}

#[test]
fn test_compile_continue() {
    let source = r#"
        for (let i = 0; i < 5; i = i + 1) {
            if (i == 2) {
                continue;
            }
            print i;
        }
    "#;
    let mut compiler = Compiler::new(source);
    let chunk = compiler.compile().expect("Compilation failed");
    // Should compile without error
}
