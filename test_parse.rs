use pulse_compiler::ParserV2;

fn main() {
    let source = r#"// Test simple pattern matching
print("Simple pattern matching:");

let value = 42;
match value {
    0 => print("Value is zero"),
    42 => print("Value is 42"),
    _ => print("Value is something else")
};
"#;
    println!("Source: {}", source);
    let mut parser = ParserV2::new(source);
    match parser.parse() {
        Ok(script) => println!("Successfully parsed script: {:?}", script),
        Err(err) => println!("Error parsing: {:?}", err),
    }
}
