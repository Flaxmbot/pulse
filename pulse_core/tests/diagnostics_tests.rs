use pulse_core::error::{DiagnosticSeverity, PulseError};

#[test]
fn compile_error_diagnostic_includes_line_and_column() {
    let err = PulseError::CompileError(
        "Unexpected token (at 12:34)\nprint(1\n     ^".to_string(),
        12,
    );
    let diag = err.to_diagnostic();

    assert_eq!(diag.code, "PUL-E0001");
    assert_eq!(diag.severity, DiagnosticSeverity::Error);
    let span = diag
        .span
        .expect("compile diagnostics should include a span");
    assert_eq!(span.line, 12);
    assert_eq!(span.column, Some(34));
}

#[test]
fn runtime_error_diagnostic_has_actionable_fix() {
    let err = PulseError::IndexOutOfBounds {
        index: 9,
        length: 3,
    };
    let diag = err.to_diagnostic();

    assert_eq!(diag.code, "PUL-E0005");
    assert!(
        diag.span.is_none(),
        "runtime diagnostics should not fake spans"
    );
    assert!(
        !diag.fixes.is_empty(),
        "runtime diagnostics should include at least one fix suggestion"
    );
    assert!(
        diag.fixes[0].message.contains("outside"),
        "expected bounds-related guidance, got: {}",
        diag.fixes[0].message
    );
}
