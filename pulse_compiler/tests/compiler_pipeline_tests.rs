use pulse_compiler::{compile, Lexer, ParserV2, Token};
use pulse_core::{Op, PulseError};
use std::fs;
use std::path::PathBuf;

#[test]
fn lexer_recognizes_shift_and_power_tokens() {
    let source = "let x = 1 << 2; let y = 2 ** 3; let z = 8 >> 1;";
    let mut lexer = Lexer::new(source);
    let mut seen_shift_left = false;
    let mut seen_shift_right = false;
    let mut seen_pow = false;

    loop {
        let token = lexer
            .next_token()
            .expect("lexer tokenization should succeed");
        match token {
            Token::ShiftLeft => seen_shift_left = true,
            Token::ShiftRight => seen_shift_right = true,
            Token::StarStar => seen_pow = true,
            Token::Eof => break,
            _ => {}
        }
    }

    assert!(seen_shift_left, "expected to see << token");
    assert!(seen_shift_right, "expected to see >> token");
    assert!(seen_pow, "expected to see ** token");
}

#[test]
fn parser_parses_function_and_call() {
    let source = r#"
        fn add(a: Int, b: Int) -> Int {
            return a + b;
        }
        let x = add(1, 2);
    "#;
    let mut parser = ParserV2::new(source);
    let script = parser.parse().expect("parser should succeed");
    assert!(
        script.declarations.len() >= 2,
        "expected at least function + let declarations"
    );
}

#[test]
fn parser_reports_missing_semicolon() {
    let source = "let x = 1";
    let mut parser = ParserV2::new(source);
    let err = parser
        .parse()
        .expect_err("parser should reject missing semicolon");
    assert!(
        matches!(err, PulseError::CompileError(_, _)),
        "expected compile error, got: {:?}",
        err
    );
}

#[test]
fn compiler_rejects_path_traversal_imports() {
    let source = r#"import "../secret.pulse";"#;
    let err = compile(source, Some("main.pulse".to_string()))
        .expect_err("compiler should block path traversal");
    assert!(
        matches!(err, PulseError::PathTraversalAttempted(_)),
        "expected PathTraversalAttempted, got: {:?}",
        err
    );
}

#[test]
fn compiler_emits_halt_for_simple_program() {
    let source = "let x = 1 + 2; print(x);";
    let chunk =
        compile(source, Some("main.pulse".to_string())).expect("compilation should succeed");
    assert!(!chunk.code.is_empty(), "chunk should contain instructions");
    assert!(
        chunk.code.contains(&(Op::Halt as u8)) || chunk.code.contains(&(Op::Return as u8)),
        "compiled chunk should contain HALT or RETURN termination opcode"
    );
}

#[test]
fn compiler_compiles_language_full_coverage_file() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../tests/integration/language_full_coverage.pulse");
    let source = fs::read_to_string(&path).expect("coverage file should be readable");

    compile(&source, Some(path.display().to_string()))
        .expect("coverage file should compile successfully");
}

#[test]
fn compiler_compiles_examples_comprehensive_file() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../examples/comprehensive.pulse");
    let source = fs::read_to_string(&path).expect("comprehensive file should be readable");

    compile(&source, Some(path.display().to_string()))
        .expect("comprehensive file should compile successfully");
}

#[test]
fn compiler_compiles_examples_comprehensive_super_file() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../examples/comprehensive_super.pulse");
    let source = fs::read_to_string(&path).expect("comprehensive_super file should be readable");

    compile(&source, Some(path.display().to_string()))
        .expect("comprehensive_super file should compile successfully");
}
