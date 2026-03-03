use pulse_compiler::parser_v2::ParserV2;

#[test]
fn test_parse_match_simple() {
    let source = r#"let value = 42;
match value {
    0 => print("Value is zero"),
    42 => print("Value is 42"),
    _ => print("Value is something else")
};"#;

    let mut parser = ParserV2::new(source);
    match parser.parse() {
        Ok(script) => println!("Success: {:?}", script),
        Err(e) => panic!("Error parsing: {:?}", e),
    }
}
