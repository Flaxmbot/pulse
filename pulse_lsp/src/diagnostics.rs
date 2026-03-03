use pulse_compiler::lexer::Lexer;
use pulse_core::PulseError;
use tower_lsp::lsp_types::*;

/// Run diagnostics on source code and return LSP diagnostics
pub fn diagnose_source(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Try to lex the source
    let mut lexer = Lexer::new(source);

    loop {
        match lexer.next_token() {
            Ok(token) => {
                if matches!(token, pulse_compiler::lexer::Token::Eof) {
                    break;
                }
            }
            Err(e) => {
                // Extract line number from error if available
                let line = extract_line_from_error(&e);

                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line, character: 0 },
                        end: Position {
                            line,
                            character: 100,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("pulse".into()),
                    message: format!("{}", e),
                    related_information: None,
                    tags: None,
                    data: None,
                });
                break;
            }
        }
    }

    diagnostics
}

fn extract_line_from_error(e: &PulseError) -> u32 {
    match e {
        PulseError::CompileError(_, line) => *line as u32,
        _ => 0,
    }
}
