use pulse_compiler::ParserV2;

#[test]
fn test_parse_simple_pattern() {
    let source = r#"
// Test simple pattern matching
print("Simple pattern matching:");

let value = 42;
match value {
    0 => print("Value is zero"),
    42 => print("Value is 42"),
    _ => print("Value is something else")
};
"#;
    let mut parser = ParserV2::new(source);
    let result = parser.parse();
    println!("Parsing test_pattern_simple.pulse result: {:?}", result);
    assert!(result.is_ok());
}

#[test]
fn test_parse_range_pattern() {
    let source = r#"
print("Range patterns:");
let x = 5;
match x {
    1..10 => print("Between 1 and 9"),
    10..20 => print("Between 10 and 19"),
    _ => print("Other")
};
"#;
    let mut parser = ParserV2::new(source);
    let result = parser.parse();
    println!("Parsing test_pattern_matching.pulse result: {:?}", result);
    assert!(result.is_ok());
}
