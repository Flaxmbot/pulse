use std::collections::HashMap;
use pulse_core::{Value, NativeFn};
use pulse_core::object::Object;
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
            add_native_async("read", pulse_stdlib::io::read_file_native, &mut exports, vm);
            add_native_async("write", pulse_stdlib::io::write_file_native, &mut exports, vm);
            add_native_async("exists", pulse_stdlib::io::file_exists_native, &mut exports, vm);
            add_native_async("delete", pulse_stdlib::io::delete_file_native, &mut exports, vm);
        }
        "std/net" => {
             add_native_async("tcp_connect", pulse_stdlib::networking::tcp_connect_native, &mut exports, vm);
             add_native_async("tcp_listen", pulse_stdlib::networking::tcp_listen_native, &mut exports, vm);
             add_native_async("tcp_accept", pulse_stdlib::networking::tcp_accept_native, &mut exports, vm);
             add_native_async("tcp_send", pulse_stdlib::networking::tcp_send_native, &mut exports, vm);
             add_native_async("tcp_receive", pulse_stdlib::networking::tcp_receive_native, &mut exports, vm);
             add_native_async("udp_bind", pulse_stdlib::networking::socket_create_native, &mut exports, vm); // socket_create is bind?
             add_native_async("dns_resolve", pulse_stdlib::networking::dns_resolve_native, &mut exports, vm);
        }
        "std/http" => {
             add_native_async("get", pulse_stdlib::networking::http_get_native, &mut exports, vm);
             add_native_async("post", pulse_stdlib::networking::http_post_native, &mut exports, vm);
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

fn add_native(name: &str, func: pulse_core::value::SyncNativeFn, exports: &mut HashMap<String, Value>, vm: &mut VM) {
    let native = NativeFn { name: name.to_string(), func: pulse_core::value::NativeFunctionKind::Sync(func) };
    let handle = vm.heap.alloc(Object::NativeFn(native));
    exports.insert(name.to_string(), Value::Obj(handle));
}

fn add_native_async(name: &str, func: pulse_core::value::AsyncNativeFn, exports: &mut HashMap<String, Value>, vm: &mut VM) {
    let native = NativeFn { name: name.to_string(), func: pulse_core::value::NativeFunctionKind::Async(func) };
    let handle = vm.heap.alloc(Object::NativeFn(native));
    exports.insert(name.to_string(), Value::Obj(handle));
}
