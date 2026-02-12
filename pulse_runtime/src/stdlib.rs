use std::collections::HashMap;
use pulse_core::{Value, NativeFn};
use pulse_core::object::{Object, HeapInterface};
use pulse_vm::VM;

pub fn load_std_module(name: &str, vm: &mut VM) -> Option<pulse_core::object::ObjHandle> {
    let mut exports = HashMap::new();
    
    match name {
        "std/math" => {
            // Need to implement math natives or reuse existing ones
            // For now, let's just add abs from utils
            add_native("abs", pulse_stdlib::utils::abs_native, &mut exports, vm);
        }
        "std/io" => {
            add_native("read", pulse_stdlib::io::read_file_native, &mut exports, vm);
            add_native("write", pulse_stdlib::io::write_file_native, &mut exports, vm);
            add_native("exists", pulse_stdlib::io::file_exists_native, &mut exports, vm);
        }
        "std/json" => {
            add_native("parse", pulse_stdlib::json::json_parse_native, &mut exports, vm);
            add_native("stringify", pulse_stdlib::json::json_stringify_native, &mut exports, vm);
        }
        _ => return None,
    }
    
    let handle = vm.heap.alloc(Object::Module(exports));
    Some(handle)
}

fn add_native(name: &str, func: fn(&mut dyn HeapInterface, &[Value]) -> pulse_core::PulseResult<Value>, exports: &mut HashMap<String, Value>, vm: &mut VM) {
    let native = NativeFn { name: name.to_string(), func };
    let handle = vm.heap.alloc(Object::NativeFn(native));
    exports.insert(name.to_string(), Value::Obj(handle));
}
