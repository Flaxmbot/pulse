use pulse_runtime::Runtime;

#[test]
fn test_list_destructuring_exact() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let x = [1, 2];
        let res = match x {
            [1, 2] => "exact",
            [a, b] => "bind",
            _ => "fail",
        };
        print res;
        if (res == "exact") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_list_destructuring_bind() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        fn return_b(val) { return val; }

        let x = [10, 20];
        let res = match x {
            [a, b] => return_b(b),
            _ => 0,
        };
        print res;
        if (res == 20) { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_list_tail() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        fn get_tail(l) {
            return match l {
                [h | t] => t,
                [] => [],
            };
        }
        
        let x = [1, 2, 3];
        let t = get_tail(x);
        // t should be [2, 3]
        
        // Check t length or content
        let res = match t {
            [2, 3] => "ok",
            _ => "fail",
        };
        
        print res;
        if (res == "ok") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_list_tail_bind() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        fn sum_list(l) {
            return match l {
                [] => 0,
                [h | t] => h + sum_list(t),
            };
        }
        
        let x = [1, 2, 3, 4];
        let s = sum_list(x);
        print s;
        if (s == 10) { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    
    for _ in 0..200 {
        if !runtime.step() { break; }
    }
}

#[test]
fn test_nested_destructuring() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        fn extract_inner(x) {
            return match x {
                [[i, j], k] => i + j + k,
                _ => 0,
            };
        }
        
        let val = [[1, 2], 3];
        let res = extract_inner(val);
        print res;
        if (res == 6) { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_map_destructuring() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let m = {name: "Pulse", ver: 21};
        let res = match m {
            {name: "Pulse", ver: v} => v,
            _ => 0,
        };
        print res;
        if (res == 21) { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_map_missing_key() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let m = {name: "Pulse"};
        let res = match m {
            {name: "Pulse", ver: v} => "fail", // ver missing
            {name: n} => n,
        };
        print res;
        if (res == "Pulse") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}

#[test]
fn test_nested_map_list() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let data = {users: [{id: 1, name: "Alice"}, {id: 2, name: "Bob"}]};
        
        let res = match data {
            {users: [u1, {name: n}]} => n,
            _ => "fail",
        };
        
        print res;
        if (res == "Bob") { print "Success"; } else { print "Failure"; }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    runtime.step(); 
}
