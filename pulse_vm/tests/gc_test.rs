use pulse_core::{Chunk, Value, Constant};
use pulse_core::object::{Object, HeapInterface};
use pulse_vm::VM;
use pulse_core::ActorId;

#[test]
fn test_gc_basic() {
    let chunk = Chunk::new();
    let mut vm = VM::new(chunk, ActorId::new(0, 1));

    // 1. Allocate Object A (Keep on Stack)
    let handle_a = vm.alloc_object(Object::String("A".to_string()));
    vm.push(Value::Obj(handle_a));

    // 2. Allocate Object B (Pop / Drop -> Garbage)
    let handle_b = vm.alloc_object(Object::String("B".to_string()));
    // Don't push to stack, or push and pop
    vm.push(Value::Obj(handle_b));
    vm.pop().unwrap(); // "B" is now unreachable

    // 3. Allocate Object C (Keep in Globals)
    let handle_c = vm.alloc_object(Object::String("C".to_string()));
    vm.globals.insert("c_var".to_string(), Value::Obj(handle_c));

    // verify all exist before GC (strictly speaking, B exists until reused or swept)
    assert!(vm.heap.get(handle_a).is_some());
    assert!(vm.heap.get(handle_b).is_some());
    assert!(vm.heap.get(handle_c).is_some());

    // 4. Run GC
    vm.collect_garbage();

    // 5. Verify Reachability
    assert!(vm.heap.get(handle_a).is_some(), "Object A should be reachable (Stack)");
    assert!(vm.heap.get(handle_c).is_some(), "Object C should be reachable (Global)");
    assert!(vm.heap.get(handle_b).is_none(), "Object B should be collected (Garbage)");
}

#[test]
fn test_gc_string_concatenation() {
    // Test that intermediate strings during operations are collected if not rooted
    let chunk = Chunk::new();
    let mut vm = VM::new(chunk, ActorId::new(0, 2));

    // Allocate "Hello"
    let h1 = vm.alloc_object(Object::String("Hello ".to_string()));
    vm.push(Value::Obj(h1));

    // Allocate "World"
    let h2 = vm.alloc_object(Object::String("World".to_string()));
    vm.push(Value::Obj(h2));

    // Concatenate -> "Hello World" (Allocates new string)
    // Op::Add logic simulation:
    // Pop h2, Pop h1, Alloc h3, Push h3
    let v2 = vm.pop().unwrap();
    let v1 = vm.pop().unwrap();
    
    // Manually simulate Op::Add behavior for test
    let s1 = if let Value::Obj(h) = v1 { vm.heap.get(h).unwrap().as_string().unwrap().clone() } else { panic!() };
    let s2 = if let Value::Obj(h) = v2 { vm.heap.get(h).unwrap().as_string().unwrap().clone() } else { panic!() };
    let h3 = vm.alloc_object(Object::String(s1 + &s2));
    vm.push(Value::Obj(h3));

    // At this point:
    // Stack has [h3 ("Hello World")]
    // h1 ("Hello ") and h2 ("World") are popped and unreachable

    vm.collect_garbage();

    assert!(vm.heap.get(h3).is_some(), "Result string should be reachable");
    assert!(vm.heap.get(h1).is_none(), "Intermediate string 1 should be collected");
    assert!(vm.heap.get(h2).is_none(), "Intermediate string 2 should be collected");
}

trait AsString {
    fn as_string(&self) -> Option<&String>;
}
impl AsString for Object {
    fn as_string(&self) -> Option<&String> {
        match self {
            Object::String(s) => Some(s),
            _ => None,
        }
    }
}
