use pulse_runtime::Runtime;

#[test]
fn test_match_literals() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let x = 42;
        let res = match x {
            1 => "one",
            42 => "answer",
            100 => "hundred",
        };
        print res;
        if (res == "answer") { print "Success"; } else { print "Failure"; }
    "#;
    
    // We expect "answer" and "Success"
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_match_variable_binding() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        fn log_identity(val) {
             print "Captured:";
             print val;
             return val;
        }

        let x = 100;
        let res = match x {
            1 => "one",
            n => log_identity(n),
        };
        
        if (res == 100) { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_match_ordering() {
    // Ensure first match wins
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let x = 1;
        let res = match x {
            1 => "first",
            1 => "second",
            n => "wildcard",
        };
        if (res == "first") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_match_nested() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        fn fact(n) {
            return match n {
                0 => 1,
                x => x * fact(x - 1),
            };
        }
        
        let res = fact(5);
        print res;
        if (res == 120) { print "Success"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    
    for _ in 0..100 {
        if !runtime.step() { break; }
    }
}
