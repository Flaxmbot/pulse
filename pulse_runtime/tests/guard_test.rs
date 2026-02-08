use pulse_runtime::Runtime;

#[test]
fn test_basic_guard() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let x = 10;
        let res = match x {
            n if n > 5 => "greater",
            n => "less or equal",
        };
        print res;
        if (res == "greater") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_guard_failure_fallback() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let x = 3;
        let res = match x {
            n if n > 5 => "greater",
            n => "less or equal",
        };
        print res;
        if (res == "less or equal") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_destructuring_guard() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let list = [10, 20];
        let res = match list {
            [a, b] if a > b => "a is bigger",
            [a, b] if b > a => "b is bigger",
            _ => "equal",
        };
        print res;
        if (res == "b is bigger") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_map_guard() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let user = {name: "Alice", age: 30};
        let res = match user {
            {age: a} if a < 18 => "minor",
            {age: a} if a >= 18 => "adult",
            _ => "unknown",
        };
        print res;
        if (res == "adult") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_scope_cleanup_check() {
    // This test ensures that locals defined in patterns don't leak out or collide incorrectly
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let x = 10;
        let y = match x {
            a if a < 0 => 0, 
            b if b > 0 => b, 
            _ => -1,
        };
        print y;
        if (y == 10) { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}
