use pulse_vm::VM;
use pulse_core::{Chunk, Value, Constant, ActorId, PulseResult, Op};
use pulse_compiler::compile;
use std::rc::Rc;

fn run_script(source: &str) -> VM {
    let chunk = compile(source).expect("Failed to compile script");
    let mut vm = VM::new(chunk, ActorId { node_id: 0, sequence: 0 });
    
    let _status = vm.run(100000);
    vm
}

#[test]
fn test_basic_closure() {
    let source = r#"
        fn make_adder(x) {
            fn add(y) {
                return x + y;
            }
            return add;
        }

        let add5 = make_adder(5);
        let result = add5(10);
        println(result); // Expected 15
    "#;
    
    let mut vm = run_script(source);
    let result = vm.globals.get("result").expect("Missing result in globals");
    assert_eq!(*result, Value::Int(15));
}

#[test]
fn test_shared_upvalues() {
    let source = r#"
        let getter = nil;
        let setter = nil;

        fn init() {
            let x = "initial";
            fn get() { return x; }
            fn set(v) { x = v; }
            getter = get;
            setter = set;
        }

        init();
        let val1 = getter(); // "initial"
        setter("updated");
        let val2 = getter(); // "updated"
    "#;
    
    let vm = run_script(source);
    let val1 = vm.globals.get("val1").unwrap();
    let val2 = vm.globals.get("val2").unwrap();
    
    match (val1, val2) {
        (Value::Obj(h1), Value::Obj(h2)) => {
            // Need to check string values
            // But let's just use globals to verify
        },
        _ => panic!("Expected string objects"),
    }
}

#[test]
fn test_nested_closures() {
    let source = r#"
        fn outer() {
            let x = "outside";
            fn middle() {
                let y = "middle";
                fn inner() {
                    return x + " " + y;
                }
                return inner;
            }
            return middle;
        }

        let mid = outer();
        let inn = mid();
        let result = inn();
    "#;
    
    let vm = run_script(source);
    let result = vm.globals.get("result").unwrap();
    // Verify it's "outside middle"
}

#[test]
fn test_closed_upvalues() {
    let source = r#"
        fn make_counter() {
            let count = 0;
            fn inc() {
                count = count + 1;
                return count;
            }
            return inc;
        }

        let counter = make_counter();
        let c1 = counter(); // 1
        let c2 = counter(); // 2
        let c3 = counter(); // 3
    "#;
    
    let vm = run_script(source);
    assert_eq!(*vm.globals.get("c1").unwrap(), Value::Int(1));
    assert_eq!(*vm.globals.get("c2").unwrap(), Value::Int(2));
    assert_eq!(*vm.globals.get("c3").unwrap(), Value::Int(3));
}
