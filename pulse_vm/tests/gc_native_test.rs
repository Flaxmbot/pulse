use pulse_vm::VM;
use pulse_core::{Chunk, Value, Object, HeapInterface};
use pulse_core::ActorId;

#[test]
fn test_gc_native_function() {
    let chunk = Chunk::new();
    let mut vm = VM::new(chunk, ActorId::new(0, 10));

    // 1. Manually allocating to simulate state
    let h1 = vm.alloc_object(Object::String("Reachable".to_string()));
    vm.globals.insert("root".to_string(), Value::Obj(h1));

    let h2 = vm.alloc_object(Object::String("Garbage".to_string()));
    // h2 is not rooted anywhere.

    assert!(vm.heap.get_object(h2).is_some());

    // 2. Call gc() native function manually via internal API or emulate call
    // Since we don't have easy `call_native` helper exposed for testing, we can use `collect_garbage` directly
    // OR we can construct a chunk that calls `gc`.
    
    // Let's construct a chunk calling `gc`.
    // But `gc` is in globals?
    // VM::new defines `gc` in native table (Heap), but does it put it in globals?
    // `define_native` implementation puts it in `globals`.
    // So "gc" is a global variable holding `NativeFn` object.
    
    // Bytecode to call `gc()`:
    // GetGlobal("gc")
    // Call(0)
    
    use pulse_core::{Op, Constant};
    
    let gc_name_idx = vm.chunk.add_constant(Constant::String("gc".to_string()));
    
    vm.chunk.write(Op::GetGlobal as u8, 1);
    vm.chunk.write(gc_name_idx as u8, 1);
    vm.chunk.write(Op::Call as u8, 1);
    vm.chunk.write(0, 1); // 0 args
    vm.chunk.write(Op::Halt as u8, 1);
    
    vm.run(100);
    
    // 3. Verify h2 is gone
    assert!(vm.heap.get_object(h1).is_some(), "Root object should persist");
    assert!(vm.heap.get_object(h2).is_none(), "Garbage object should be collected");
}
