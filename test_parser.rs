
use pulse_compiler::lexer::Lexer;
use pulse_compiler::parser_v2::ParserV2;

fn main() {
    let source = r#"let value = 42;
match value {
    0 => print("Value is zero"),
    42 => print("Value is 42"),
    _ => print("Value is something else")
};"#;

    println!("=== Testing Lexer ===");
    let mut lexer = Lexer::new(source);
    loop {
        let token = lexer.next_token().unwrap();
        println!("{:?}", token);
        if token == pulse_compiler::lexer::Token::Eof {
            break;
        }
    }

    println!("\n=== Testing Parser ===");
    let mut parser = ParserV2::new(source);
    match parser.parse() {
        Ok(script) => println!("Success: {:?}", script),
        Err(e) => println!("Error: {:?}", e),
    }
}
