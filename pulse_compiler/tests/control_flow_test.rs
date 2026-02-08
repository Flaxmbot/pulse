use pulse_compiler::Compiler;
use pulse_core::Op;

#[test]
fn test_compile_if_while() {
    let source = r#"
        let a = 10;
        if (a > 5) {
            print "a is big";
        } else {
            print "a is small";
        }

        let i = 0;
        while (i < 5) {
            print i;
            i = i + 1;
        }
    "#;

    let chunk = pulse_compiler::compile(source).expect("Compilation failed");

    let code = chunk.code;
    
    // Check for opcodes
    assert!(code.contains(&(Op::JumpIfFalse as u8)));
    assert!(code.contains(&(Op::Jump as u8)));
    assert!(code.contains(&(Op::Loop as u8))); 
}
