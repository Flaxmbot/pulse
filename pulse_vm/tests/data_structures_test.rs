use pulse_core::{Value, Object, ActorId};
use pulse_vm::VM;
use pulse_compiler::Compiler;

fn run_script(source: &str) -> VM {
    let chunk = pulse_compiler::compile(source).expect("Compilation failed");
    let mut vm = VM::new(chunk, ActorId::new(0, 1));
    vm.run(10000); 
    vm
}

#[test]
fn test_list_basics() {
    let source = r#"
        let l = [10, 20, 30];
        let size = len(l);
        let first = l[0];
        let mid = l[1];
        
        push(l, 40);
        let size2 = len(l);
        let last = l[3];
        
        let popped = pop(l);
        let size3 = len(l);
    "#;
    let vm = run_script(source);
    
    assert_eq!(vm.globals.get("size"), Some(&Value::Int(3)));
    assert_eq!(vm.globals.get("first"), Some(&Value::Int(10)));
    assert_eq!(vm.globals.get("mid"), Some(&Value::Int(20)));
    assert_eq!(vm.globals.get("size2"), Some(&Value::Int(4)));
    assert_eq!(vm.globals.get("last"), Some(&Value::Int(40)));
    assert_eq!(vm.globals.get("popped"), Some(&Value::Int(40)));
    assert_eq!(vm.globals.get("size3"), Some(&Value::Int(3)));
}

#[test]
fn test_list_indexing_expr() {
    let source = r#"
        let l = [1, 2, 3];
        let idx = 1;
        let val = l[idx];
        l[0] = 99;
        let set_val = l[0];
    "#;
    let vm = run_script(source);
    
    assert_eq!(vm.globals.get("val"), Some(&Value::Int(2)));
    assert_eq!(vm.globals.get("set_val"), Some(&Value::Int(99)));
}

#[test]
fn test_map_basics() {
    let source = r#"
        let m = { "name": "Pulse", "version": 1 };
        let n = m["name"];
        let v = m["version"];
        let size = len(m);
        
        m["new"] = 100;
        let new_val = m["new"];
        let size2 = len(m);
        
        m["version"] = 2;
        let v2 = m["version"];
    "#;
    let mut vm = run_script(source); // Mut to access internals if needed, but run_script returns VM.
    
    // Check strings needs helper or match
    if let Some(Value::Obj(h)) = vm.globals.get("n") {
        if let Some(Object::String(s)) = vm.heap.get(*h) {
             assert_eq!(s, "Pulse");
        } else { panic!("n is not a string object"); }
    } else { panic!("n is not an object"); }

    assert_eq!(vm.globals.get("v"), Some(&Value::Int(1)));
    assert_eq!(vm.globals.get("size"), Some(&Value::Int(2)));
    assert_eq!(vm.globals.get("new_val"), Some(&Value::Int(100)));
    assert_eq!(vm.globals.get("size2"), Some(&Value::Int(3)));
    assert_eq!(vm.globals.get("v2"), Some(&Value::Int(2)));
}

#[test]
fn test_nested_structures() {
    let source = r#"
        let l = [1, [2, 3]];
        let inner = l[1];
        let val = inner[0];
        
        let m = { "list": [10, 20] };
        let ml = m["list"];
        let mv = ml[1];
    "#;
    let vm = run_script(source);
    
    assert_eq!(vm.globals.get("val"), Some(&Value::Int(2)));
    assert_eq!(vm.globals.get("mv"), Some(&Value::Int(20)));
}
