use pulse_compiler::compile;
use pulse_ast::PulseError;

fn normalize_diagnostic(err: &PulseError) -> String {
    let diag = err.to_diagnostic();
    let span = match diag.span {
        Some(span) => format!("{}:{}", span.line, span.column.unwrap_or(0)),
        None => "-".to_string(),
    };
    let headline = diag.message.lines().next().unwrap_or("").trim();
    format!("code={}\nspan={}\nheadline={}\n", diag.code, span, headline)
}

fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn golden_syntax_diagnostic_text_and_span() {
    let source = "fn bad(a {\n    return a;\n}";
    let err = compile(source, Some("golden_syntax.pulse".to_string()))
        .expect_err("expected syntax error");
    let actual = normalize_diagnostic(&err);
    let expected = normalize_line_endings(include_str!("golden/syntax_missing_paren.golden"));
    assert_eq!(actual, expected);
}

#[test]
fn golden_type_diagnostic_text_and_span() {
    let source = r#"let x: Int = "oops";"#;
    let err =
        compile(source, Some("golden_type.pulse".to_string())).expect_err("expected type error");
    let actual = normalize_diagnostic(&err);
    let expected = normalize_line_endings(include_str!("golden/type_mismatch_assignment.golden"));
    assert_eq!(actual, expected);
}
