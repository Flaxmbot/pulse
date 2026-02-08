use pulse_runtime::Runtime;

#[test]
fn test_try_catch_basic() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let result = "not caught";
        try {
            throw "error message";
            result = "never reached";
        } catch e {
            result = "caught";
        }
        print result;
        if (result == "caught") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_try_no_throw() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let result = "before";
        try {
            result = "inside try";
        } catch e {
            result = "caught";
        }
        print result;
        if (result == "inside try") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_nested_try_catch() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let result = "";
        try {
            try {
                throw "inner";
            } catch e1 {
                result = "inner caught";
            }
        } catch e2 {
            result = "outer caught";
        }
        print result;
        if (result == "inner caught") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_throw_propagates() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let result = "";
        try {
            try {
                throw "error";
            } catch e1 {
                throw "rethrown";
            }
        } catch e2 {
            result = "outer caught";
        }
        print result;
        if (result == "outer caught") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}

#[test]
fn test_exception_value() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let result = "";
        try {
            throw 42;
        } catch e {
            result = "caught: ${e}";
        }
        print result;
        if (result == "caught: 42") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step();
}
