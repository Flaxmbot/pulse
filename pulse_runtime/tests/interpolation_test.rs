use pulse_runtime::Runtime;

#[test]
fn test_basic_interpolation() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let name = "World";
        let msg = "Hello, ${name}!";
        print msg;
        if (msg == "Hello, World!") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_expression_interpolation() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let msg = "Sum: ${1 + 2}";
        print msg;
        if (msg == "Sum: 3") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_number_coercion() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let count = 42;
        let msg = "Count: ${count}";
        print msg;
        if (msg == "Count: 42") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_multiple_interpolations() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let a = "Hello";
        let b = "World";
        let msg = "${a}, ${b}!";
        print msg;
        if (msg == "Hello, World!") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_no_interpolation() {
    let mut runtime = Runtime::new(1);
    
    // Regular strings without ${} should still work
    let source = r#"
        let msg = "Hello World";
        print msg;
        if (msg == "Hello World") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}
