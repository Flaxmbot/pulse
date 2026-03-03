
use pulse_compiler::lexer::Lexer;
use std::fs;

fn main() {
    let source = fs::read_to_string("test_with_pattern.pulse").expect("Failed to read file");
    println!("Source: \n{}", source);
    println!("\nTokens:");
    
    let mut lexer = Lexer::new(&source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        tokens.push(token.clone());
        println!("{:?}", token);
        if token == pulse_compiler::lexer::Token::Eof {
            break;
        }
    }
}
