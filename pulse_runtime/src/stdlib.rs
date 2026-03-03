use pulse_core::object::Object;
use pulse_core::{NativeFn, Value};
use pulse_vm::VM;
use std::collections::HashMap;

pub fn load_std_module(name: &str, vm: &mut VM) -> Option<pulse_core::object::ObjHandle> {
    let mut exports = HashMap::new();

    match name {
        "std/math" => {
            // Need to implement math natives or reuse existing ones
            // For now, let's just add abs from utils
            add_native("abs", pulse_stdlib::utils::abs_native, &mut exports, vm);
        }
        "std/io" => {
            add_native_async("read", pulse_stdlib::io::read_file_native, &mut exports, vm);
            add_native_async(
                "write",
                pulse_stdlib::io::write_file_native,
                &mut exports,
                vm,
            );
            add_native_async(
                "exists",
                pulse_stdlib::io::file_exists_native,
                &mut exports,
                vm,
            );
            add_native_async(
                "delete",
                pulse_stdlib::io::delete_file_native,
                &mut exports,
                vm,
            );
        }
        "std/json" => {
            add_native(
                "parse",
                pulse_stdlib::json::json_parse_native,
                &mut exports,
                vm,
            );
            add_native(
                "stringify",
                pulse_stdlib::json::json_stringify_native,
                &mut exports,
                vm,
            );
        }
        // TODO: The following modules were removed during Phase 0 cleanup because
        // they referenced stdlib modules that don't exist yet:
        // - std/net (networking, websocket, fastapi)
        // - std/http (networking)
        // - std/pandas
        // - std/linalg
        // - std/stats
        // - std/random
        // - std/plotting
        // - std/database
        // These will be re-added once the corresponding stdlib modules are implemented.
        _ => return None,
    }

    let handle = vm.heap.alloc(Object::Module(exports));
    Some(handle)
}

fn add_native(
    name: &str,
    func: pulse_core::value::SyncNativeFn,
    exports: &mut HashMap<String, Value>,
    vm: &mut VM,
) {
    let native = NativeFn {
        name: name.to_string(),
        func: pulse_core::value::NativeFunctionKind::Sync(func),
    };
    let handle = vm.heap.alloc(Object::NativeFn(native));
    exports.insert(name.to_string(), Value::Obj(handle));
}

fn add_native_async(
    name: &str,
    func: pulse_core::value::AsyncNativeFn,
    exports: &mut HashMap<String, Value>,
    vm: &mut VM,
) {
    let native = NativeFn {
        name: name.to_string(),
        func: pulse_core::value::NativeFunctionKind::Async(func),
    };
    let handle = vm.heap.alloc(Object::NativeFn(native));
    exports.insert(name.to_string(), Value::Obj(handle));
}
