use pulse_ast::PulseError;
use pulse_compiler::compile;

#[test]
fn syntax_error_reports_line_column_and_snippet() {
    let source = "fn bad(a {\n    return a;\n}";
    let err = compile(source, Some("diag.pulse".to_string()))
        .expect_err("source should fail due to missing ')' in parameter list");

    match err {
        PulseError::CompileError(msg, line) => {
            assert_eq!(line, 1, "expected error to be reported on line 1");
            assert!(
                msg.contains("Expect ')' after parameters."),
                "expected parameter error message, got: {}",
                msg
            );
            assert!(
                msg.contains("instead of RightParen"),
                "expected expected-token details, got: {}",
                msg
            );
            assert!(
                msg.contains("fn bad(a {"),
                "expected source excerpt, got: {}",
                msg
            );
            assert!(msg.contains('^'), "expected caret pointer, got: {}", msg);
        }
        other => panic!("expected compile error, got: {:?}", other),
    }
}

#[test]
fn identifier_errors_use_real_line_numbers() {
    let source = "let 123 = 1;";
    let err = compile(source, Some("diag.pulse".to_string()))
        .expect_err("source should fail due to invalid variable name");

    match err {
        PulseError::CompileError(msg, line) => {
            assert_eq!(line, 1, "line number should not be zero");
            assert!(
                msg.contains("Expect variable name."),
                "expected variable-name diagnostic, got: {}",
                msg
            );
            assert!(
                msg.contains("(at 1:5)"),
                "expected offending-token details, got: {}",
                msg
            );
            assert!(msg.contains('^'), "expected caret pointer, got: {}", msg);
        }
        other => panic!("expected compile error, got: {:?}", other),
    }
}

#[test]
fn map_syntax_error_reports_expected_token_details() {
    let source = "let m = { \"a\" 1 };";
    let err = compile(source, Some("diag.pulse".to_string()))
        .expect_err("source should fail due to missing ':' in map literal");

    match err {
        PulseError::CompileError(msg, line) => {
            assert_eq!(line, 1, "expected map error to be on line 1");
            assert!(
                msg.contains("Expect ':' after map key."),
                "expected map-key diagnostic, got: {}",
                msg
            );
            assert!(
                msg.contains("instead of Colon"),
                "expected expected-token details, got: {}",
                msg
            );
            assert!(msg.contains('^'), "expected caret pointer, got: {}", msg);
        }
        other => panic!("expected compile error, got: {:?}", other),
    }
}
