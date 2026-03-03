use pulse_core::object::{
    BoundMethod, Closure, Function, HeapInterface, Instance, ObjHandle, Object,
};
use pulse_core::{ActorId, Chunk, Constant, NativeFn, Op, PulseError, PulseResult, Value};

use crate::shared_heap::{SharedHandle, SharedHeap};
use crate::Heap;
use pulse_stdlib::utils::{
    clock_native, gc_native, len_native, pop_native, println_native, push_native,
};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

const MAX_FRAMES: usize = 1024;
const MAX_STACK_DEPTH: usize = 10000; // Stack overflow protection
const GC_HEAP_THRESHOLD: usize = 10 * 1024 * 1024; // 10MB trigger for GC

// ====================================================================
// PHASE 2.3b: CAPABILITY-BASED SECURITY
// ====================================================================

/// Capability enum representing permissions for privileged operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Permission to read files
    FileRead,
    /// Permission to write files
    FileWrite,
    /// Permission for network access
    Network,
    /// Permission to spawn new actors
    SpawnActor,
    /// Permission to execute shell commands
    Execute,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::FileRead => write!(f, "FileRead"),
            Capability::FileWrite => write!(f, "FileWrite"),
            Capability::Network => write!(f, "Network"),
            Capability::SpawnActor => write!(f, "SpawnActor"),
            Capability::Execute => write!(f, "Execute"),
        }
    }
}

/// Security context containing granted capabilities and security settings
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Set of granted capabilities
    pub permitted: HashSet<Capability>,
    /// Default to deny (true = secure by default)
    pub deny_by_default: bool,
}

impl SecurityContext {
    /// Create a new security context with deny-by-default policy
    pub fn new() -> Self {
        Self {
            permitted: HashSet::new(),
            deny_by_default: true,
        }
    }

    /// Create a new security context with all capabilities granted (for trusted code)
    pub fn trusted() -> Self {
        let mut permitted = HashSet::new();
        permitted.insert(Capability::FileRead);
        permitted.insert(Capability::FileWrite);
        permitted.insert(Capability::Network);
        permitted.insert(Capability::SpawnActor);
        permitted.insert(Capability::Execute);
        Self {
            permitted,
            deny_by_default: false,
        }
    }

    /// Check if a capability is granted
    pub fn has_capability(&self, cap: Capability) -> bool {
        if self.deny_by_default {
            self.permitted.contains(&cap)
        } else {
            true
        }
    }

    /// Grant a capability
    pub fn grant(&mut self, cap: Capability) {
        self.permitted.insert(cap);
    }

    /// Revoke a capability
    pub fn revoke(&mut self, cap: Capability) {
        self.permitted.remove(&cap);
    }

    /// Check capability and return error if denied
    pub fn check_capability(&self, cap: Capability) -> PulseResult<()> {
        if !self.has_capability(cap) {
            Err(PulseError::CapabilityDenied(cap.to_string()))
        } else {
            Ok(())
        }
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self::new()
    }
}

// ====================================================================
// PHASE 2.3c: RESOURCE LIMITS
// ====================================================================

/// Resource limits for memory, CPU time, and file descriptors
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// Maximum memory in MB (default: 100)
    pub max_memory_mb: usize,
    /// Maximum CPU time in milliseconds (default: 30000 = 30s)
    pub max_cpu_time_ms: u64,
    /// Maximum file descriptors (default: 50)
    pub max_file_descriptors: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 100,
            max_cpu_time_ms: 30000,
            max_file_descriptors: 50,
        }
    }
}

/// Resource tracker for monitoring resource usage
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    /// Resource limits
    pub limits: ResourceLimits,
    /// Start time for CPU time tracking
    pub start_time: Instant,
    /// Current file descriptor count
    pub file_descriptor_count: usize,
}

impl ResourceTracker {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            start_time: Instant::now(),
            file_descriptor_count: 0,
        }
    }

    /// Check if memory usage is within limits
    pub fn check_memory(&self, current_bytes: usize) -> PulseResult<()> {
        let max_bytes = self.limits.max_memory_mb * 1024 * 1024;
        if current_bytes > max_bytes {
            Err(PulseError::ResourceLimitExceeded(format!(
                "Memory limit exceeded: {} MB > {} MB",
                current_bytes / (1024 * 1024),
                self.limits.max_memory_mb
            )))
        } else {
            Ok(())
        }
    }

    /// Check if CPU time is within limits
    pub fn check_cpu_time(&self) -> PulseResult<()> {
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        if elapsed_ms > self.limits.max_cpu_time_ms {
            Err(PulseError::ResourceLimitExceeded(format!(
                "CPU time limit exceeded: {} ms > {} ms",
                elapsed_ms, self.limits.max_cpu_time_ms
            )))
        } else {
            Ok(())
        }
    }

    /// Check if file descriptor count is within limits
    pub fn check_file_descriptor(&self) -> PulseResult<()> {
        if self.file_descriptor_count >= self.limits.max_file_descriptors {
            Err(PulseError::ResourceLimitExceeded(format!(
                "File descriptor limit exceeded: {} >= {}",
                self.file_descriptor_count, self.limits.max_file_descriptors
            )))
        } else {
            Ok(())
        }
    }

    /// Increment file descriptor count
    pub fn increment_fd(&mut self) {
        self.file_descriptor_count += 1;
    }

    /// Decrement file descriptor count
    pub fn decrement_fd(&mut self) {
        if self.file_descriptor_count > 0 {
            self.file_descriptor_count -= 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub closure: ObjHandle,
    pub ip: usize,
    pub stack_start: usize,
    pub is_module: bool,
    pub module_path: Option<String>,
    pub prev_globals: Option<HashMap<String, Value>>,
}

#[derive(Debug, PartialEq)]
pub enum VMStatus {
    Running,
    Yielded,
    Blocked, // Waiting for message
    Halted,
    Paused, // Debugger pause
    Error(PulseError),
    // Effects
    Spawn(
        Constant,
        Vec<Constant>,
        std::collections::HashMap<String, Constant>,
    ), // closure, upvalues, globals
    SpawnCall(
        Vec<Constant>,
        Vec<Constant>,
        std::collections::HashMap<String, Constant>,
    ), // args (first is callable), upvalues, globals
    Send {
        target: ActorId,
        msg: Constant,
    },
    Import(String),   // Path to import
    Link(ActorId),    // Link to target actor
    Monitor(ActorId), // Monitor target actor
    SpawnLink(
        Constant,
        Vec<Constant>,
        std::collections::HashMap<String, Constant>,
    ), // Spawn and link to new actor
    Register(String, ActorId),
    Unregister(String),
    WhereIs(String),
}

#[derive(Debug, Clone)]
pub struct ExceptionFrame {
    pub handler_ip: usize,  // Where to jump on exception
    pub stack_depth: usize, // Stack depth when try was entered
    pub frame_depth: usize, // Call frame depth when try was entered
}

pub struct VM {
    // chunk: Chunk, // Removed: Chunk is now in CallFrame (via Closure)
    // ip: usize,    // Removed: IP is now in CallFrame
    pub pid: ActorId,
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
    pub globals: HashMap<String, Value>,
    pub builtins: HashMap<String, Value>,
    global_cache: HashMap<String, Value>, // Cache for frequently accessed globals
    pub heap: Heap,
    /// Shared heap for zero-copy cross-actor communication
    /// This heap stores shared memory objects that all actors can access
    pub shared_heap: Option<Arc<SharedHeap>>,
    pub open_upvalues: Vec<ObjHandle>, // Tracks upvalues still on stack
    pub loaded_modules: HashMap<String, ObjHandle>,
    pub exception_frames: Vec<ExceptionFrame>,
    pub debug_ctx: Option<crate::debug::DebugContext>,
    /// Security context for capability-based access control
    pub security_context: SecurityContext,
    /// Resource limits and tracking
    pub resource_tracker: ResourceTracker,
}

// unsafe impl Send for VM {}

impl VM {
    pub fn new(chunk: Chunk, pid: ActorId, shared_heap: Option<Arc<SharedHeap>>) -> Self {
        let mut heap = Heap::new();

        // Wrap script in Function/Closure
        let script_func = Function {
            arity: 0,
            chunk: Arc::new(chunk),
            name: "script".to_string(),
            upvalue_count: 0,
            module_path: None, // Default to None, Runtime sets it for main
        };
        let _func_handle = heap.alloc(Object::Function(script_func.clone()));
        // Note: Object::Function stores struct.
        // Wait, Object::Function(Function).
        // Closure needs Function struct or handle?
        // Closure struct: pub function: Function.
        // So we copy Function struct into Closure.

        let closure = Closure {
            function: script_func,
            upvalues: Vec::new(),
        };
        let closure_handle = heap.alloc(Object::Closure(closure));

        // Initial Frame
        let frame = CallFrame {
            closure: closure_handle,
            ip: 0,
            stack_start: 0,
            is_module: false,
            module_path: None,
            prev_globals: None,
        };

        let stack = Vec::with_capacity(2048);

        let mut vm = Self {
            pid,
            stack,
            frames: vec![frame], // Start with script frame
            globals: HashMap::new(),
            builtins: HashMap::new(),
            global_cache: HashMap::new(),
            heap,
            shared_heap,
            open_upvalues: Vec::new(),
            loaded_modules: HashMap::new(),
            exception_frames: Vec::new(),
            debug_ctx: None,
            security_context: SecurityContext::trusted(), // Start with all capabilities for main script
            resource_tracker: ResourceTracker::new(ResourceLimits::default()),
        };
        vm.push(Value::Obj(closure_handle)); // Push script closure to slot 0
        vm.register_all_natives();
        vm
    }

    pub fn register_all_natives(&mut self) {
        self.define_native("clock", clock_native);
        self.define_native("println", println_native);
        self.define_native("gc", gc_native);
        self.define_native("len", len_native);
        self.define_native("push", push_native);
        self.define_native("pop", pop_native);

        // Standard Library v2 natives
        self.define_native_async("read_file", pulse_stdlib::io::read_file_native);
        self.define_native_async("write_file", pulse_stdlib::io::write_file_native);
        self.define_native_async("file_exists", pulse_stdlib::io::file_exists_native);
        self.define_native_async("delete_file", pulse_stdlib::io::delete_file_native);
        self.define_native("json_parse", pulse_stdlib::json::json_parse_native);
        self.define_native("json_stringify", pulse_stdlib::json::json_stringify_native);

        self.define_native("random", pulse_stdlib::utils::random_native);
        self.define_native("random_int", pulse_stdlib::utils::random_int_native);
        self.define_native_async("sleep", pulse_stdlib::utils::sleep_native);
        self.define_native("type_of", pulse_stdlib::utils::type_of_native);
        self.define_native("to_string", pulse_stdlib::utils::to_string_native);
        self.define_native("to_int", pulse_stdlib::utils::to_int_native);
        self.define_native("abs", pulse_stdlib::utils::abs_native);
        self.define_native("string_to_list", pulse_stdlib::utils::string_to_list_native);

        // HTTP natives (only modules that exist in pulse_stdlib)
        self.define_native("http_parse", pulse_stdlib::http::http_parse_native);
        self.define_native(
            "http_format_response",
            pulse_stdlib::http::http_format_response_native,
        );

        // String utility natives
        self.define_native(
            "split_string",
            pulse_stdlib::string_utils::split_string_native,
        );
        self.define_native(
            "join_strings",
            pulse_stdlib::string_utils::join_strings_native,
        );
        self.define_native(
            "starts_with",
            pulse_stdlib::string_utils::starts_with_native,
        );
        self.define_native("ends_with", pulse_stdlib::string_utils::ends_with_native);
        self.define_native(
            "trim_string",
            pulse_stdlib::string_utils::trim_string_native,
        );
        self.define_native(
            "string_length",
            pulse_stdlib::string_utils::string_length_native,
        );
        self.define_native("substring", pulse_stdlib::string_utils::substring_native);
        self.define_native(
            "string_contains",
            pulse_stdlib::string_utils::string_contains_native,
        );
        self.define_native(
            "string_replace",
            pulse_stdlib::string_utils::string_replace_native,
        );
        self.define_native(
            "string_uppercase",
            pulse_stdlib::string_utils::string_uppercase_native,
        );
        self.define_native(
            "string_lowercase",
            pulse_stdlib::string_utils::string_lowercase_native,
        );

        // Test framework natives
        self.define_native("assert", pulse_stdlib::testing::assert_native);
        self.define_native("assert_eq", pulse_stdlib::testing::assert_eq_native);
        self.define_native("assert_ne", pulse_stdlib::testing::assert_ne_native);
        self.define_native("fail", pulse_stdlib::testing::fail_native);

        // Collection natives
        self.define_native("create_set", pulse_stdlib::utils::create_set_native);
        self.define_native("add_to_set", pulse_stdlib::utils::add_to_set_native);
        self.define_native(
            "remove_from_set",
            pulse_stdlib::utils::remove_from_set_native,
        );
        self.define_native(
            "contains_in_set",
            pulse_stdlib::utils::contains_in_set_native,
        );
        self.define_native("create_queue", pulse_stdlib::utils::create_queue_native);
        self.define_native("enqueue", pulse_stdlib::utils::enqueue_native);
        self.define_native("dequeue", pulse_stdlib::utils::dequeue_native);
        self.define_native("peek_queue", pulse_stdlib::utils::peek_queue_native);

        // Functional programming natives
        self.define_native("map_list", pulse_stdlib::utils::map_list_native);
        self.define_native("filter_list", pulse_stdlib::utils::filter_list_native);
        self.define_native("reduce_list", pulse_stdlib::utils::reduce_list_native);

        // Math natives
        self.define_native("sin", pulse_stdlib::utils::sin_native);
        self.define_native("cos", pulse_stdlib::utils::cos_native);
        self.define_native("tan", pulse_stdlib::utils::tan_native);
        self.define_native("pow", pulse_stdlib::utils::pow_native);
        self.define_native("sqrt", pulse_stdlib::utils::sqrt_native);
        self.define_native("log", pulse_stdlib::utils::log_native);
        self.define_native("log10", pulse_stdlib::utils::log10_native);
        self.define_native("floor", pulse_stdlib::utils::floor_native);
        self.define_native("ceil", pulse_stdlib::utils::ceil_native);
        self.define_native("round", pulse_stdlib::utils::round_native);

        // Memory isolation natives
        self.define_native("deep_copy", pulse_stdlib::utils::deep_copy_native);

        // ====================================================================
        // FILESYSTEM LIBRARY (async)
        // ====================================================================
        self.define_native_async("read_dir", pulse_stdlib::fs::read_dir_native);
        self.define_native_async("create_dir", pulse_stdlib::fs::create_dir_native);
        self.define_native_async("remove_dir", pulse_stdlib::fs::remove_dir_native);
        self.define_native_async("remove_file", pulse_stdlib::fs::remove_file_native);
        self.define_native_async("get_metadata", pulse_stdlib::fs::get_metadata_native);
        self.define_native_async("copy_file", pulse_stdlib::fs::copy_file_native);
        self.define_native_async("rename_file", pulse_stdlib::fs::rename_file_native);
        self.define_native_async("list_dir", pulse_stdlib::fs::list_dir_native);
        self.define_native_async("is_file", pulse_stdlib::fs::is_file_native);
        self.define_native_async("is_dir", pulse_stdlib::fs::is_dir_native);
        self.define_native_async("read_bytes", pulse_stdlib::fs::read_bytes_native);
        self.define_native_async("write_bytes", pulse_stdlib::fs::write_bytes_native);
        self.define_native_async("get_current_dir", pulse_stdlib::fs::get_current_dir_native);
        self.define_native_async("set_current_dir", pulse_stdlib::fs::set_current_dir_native);

        // ====================================================================
        // TIME LIBRARY
        // ====================================================================
        self.define_native(
            "current_timestamp",
            pulse_stdlib::time::current_timestamp_native,
        );
        self.define_native(
            "current_timestamp_millis",
            pulse_stdlib::time::current_timestamp_millis_native,
        );
        self.define_native(
            "current_timestamp_micros",
            pulse_stdlib::time::current_timestamp_micros_native,
        );
        self.define_native_async("sleep_seconds", pulse_stdlib::time::sleep_seconds_native);
        self.define_native("now", pulse_stdlib::time::now_native);
        self.define_native("format_time", pulse_stdlib::time::format_time_native);
        self.define_native("parse_time", pulse_stdlib::time::parse_time_native);
        self.define_native(
            "duration_create",
            pulse_stdlib::time::duration_create_native,
        );
        self.define_native("duration_add", pulse_stdlib::time::duration_add_native);
        self.define_native_async("measure_time", pulse_stdlib::time::measure_time_native);
        self.define_native(
            "unix_to_datetime",
            pulse_stdlib::time::unix_to_datetime_native,
        );
        self.define_native(
            "datetime_to_unix",
            pulse_stdlib::time::datetime_to_unix_native,
        );

        // ====================================================================
        // HTTP CLIENT LIBRARY (additional methods)
        // ====================================================================
        self.define_native("http_put", pulse_stdlib::http::http_put_native);
        self.define_native("http_delete", pulse_stdlib::http::http_delete_native);
        self.define_native("http_request", pulse_stdlib::http::http_request_native);
        self.define_native("http_get_body", pulse_stdlib::http::http_get_body_native);

        // ====================================================================
        // PHASE 2: CRITICAL MISSING FUNCTIONS
        // ====================================================================

        // Input / Readline
        self.define_native("input", pulse_stdlib::utils::input_native);
        self.define_native("input_prompt", pulse_stdlib::utils::input_prompt_native);

        // Type conversion
        self.define_native("to_float", pulse_stdlib::utils::to_float_native);

        // Scalar min / max
        self.define_native("min_val", pulse_stdlib::utils::min_val_native);
        self.define_native("max_val", pulse_stdlib::utils::max_val_native);

        // List operations
        self.define_native("sort_list", pulse_stdlib::utils::sort_list_native);
        self.define_native("reverse_list", pulse_stdlib::utils::reverse_list_native);
        self.define_native("index_of", pulse_stdlib::utils::index_of_native);
        self.define_native("list_insert", pulse_stdlib::utils::list_insert_native);
        self.define_native("list_remove", pulse_stdlib::utils::list_remove_native);
        self.define_native("list_slice", pulse_stdlib::utils::list_slice_native);
        self.define_native("list_contains", pulse_stdlib::utils::list_contains_native);
        self.define_native("list_concat", pulse_stdlib::utils::list_concat_native);
        self.define_native("range", pulse_stdlib::utils::range_native);
        self.define_native("list_flatten", pulse_stdlib::utils::list_flatten_native);

        // String operations
        self.define_native(
            "index_of_string",
            pulse_stdlib::string_utils::index_of_string_native,
        );
        self.define_native("char_at", pulse_stdlib::string_utils::char_at_native);
        self.define_native(
            "repeat_string",
            pulse_stdlib::string_utils::repeat_string_native,
        );
        self.define_native("pad_start", pulse_stdlib::string_utils::pad_start_native);
        self.define_native("pad_end", pulse_stdlib::string_utils::pad_end_native);

        // ====================================================================
        // PHASE 1.3: TYPE PREDICATE FUNCTIONS
        // ====================================================================
        self.define_native("is_int", pulse_stdlib::utils::is_int_native);
        self.define_native("is_float", pulse_stdlib::utils::is_float_native);
        self.define_native("is_string", pulse_stdlib::utils::is_string_native);
        self.define_native("is_bool", pulse_stdlib::utils::is_bool_native);
        self.define_native("is_list", pulse_stdlib::utils::is_list_native);
        self.define_native("is_map", pulse_stdlib::utils::is_map_native);
        self.define_native("is_nil", pulse_stdlib::utils::is_nil_native);

        // ====================================================================
        // PHASE 1.3: MAP FUNCTIONS
        // ====================================================================
        self.define_native("map_keys", pulse_stdlib::utils::map_keys_native);
        self.define_native("map_values", pulse_stdlib::utils::map_values_native);
        self.define_native("map_entries", pulse_stdlib::utils::map_entries_native);
        self.define_native("map_has_key", pulse_stdlib::utils::map_has_key_native);
        self.define_native("map_delete", pulse_stdlib::utils::map_delete_native);
        self.define_native("map_merge", pulse_stdlib::utils::map_merge_native);

        // ====================================================================
        // PHASE 1.3: COLLECTION UTILITY FUNCTIONS
        // ====================================================================
        self.define_native("enumerate_list", pulse_stdlib::utils::enumerate_list_native);
        self.define_native("zip_lists", pulse_stdlib::utils::zip_lists_native);
        self.define_native("unique_list", pulse_stdlib::utils::unique_list_native);
        self.define_native("count_list", pulse_stdlib::utils::count_list_native);

        // ====================================================================
        // PHASE 1.3: CONVERSION FUNCTIONS
        // ====================================================================
        self.define_native("parse_int", pulse_stdlib::utils::parse_int_native);
        self.define_native("parse_float", pulse_stdlib::utils::parse_float_native);

        // ====================================================================
        // PHASE 1.3: MATH FUNCTIONS
        // ====================================================================
        self.define_native("log2", pulse_stdlib::utils::log2_native);
        self.define_native("min", pulse_stdlib::utils::min_native);
        self.define_native("max", pulse_stdlib::utils::max_native);

        // ====================================================================
        // PHASE 1.3: STRING FUNCTIONS
        // ====================================================================
        self.define_native("char_code", pulse_stdlib::utils::char_code_native);
        self.define_native("from_char_code", pulse_stdlib::utils::from_char_code_native);

        // ====================================================================
        // PHASE 1.3: HEAP / PRIORITY QUEUE
        // ====================================================================
        self.define_native("create_heap", pulse_stdlib::utils::create_heap_native);
        self.define_native("heap_push", pulse_stdlib::utils::heap_push_native);
        self.define_native("heap_pop", pulse_stdlib::utils::heap_pop_native);
        self.define_native("heap_peek", pulse_stdlib::utils::heap_peek_native);
        self.define_native("heap_size", pulse_stdlib::utils::heap_size_native);
    }

    pub fn new_spawn(
        closure_const: Constant,
        captured_upvalues: Vec<Constant>,
        pid: ActorId,
        shared_heap: Option<Arc<SharedHeap>>,
        inherited_globals: std::collections::HashMap<String, Constant>,
        arguments: Vec<Constant>,
    ) -> Self {
        let mut heap = Heap::new();

        let (function, _) = match closure_const {
            Constant::Closure(f, uv) => (*f, uv),
            _ => return Self { pid: pid, stack: Vec::new(), frames: Vec::new(), globals: HashMap::new(), builtins: HashMap::new(), global_cache: HashMap::new(), heap, shared_heap, open_upvalues: Vec::new(), loaded_modules: HashMap::new(), exception_frames: Vec::new(), debug_ctx: None, security_context: SecurityContext::new(), resource_tracker: ResourceTracker::new(ResourceLimits::default()) },
        };

        // Create a new closure WITHOUT the old upvalue handles - they point to the wrong heap
        // The captured_upvalues will be properly added as new upvalues below
        let closure_without_upvalues = Closure {
            function,
            upvalues: Vec::new(), // Start with empty upvalues
        };

        let mut new_upvalues = Vec::new();
        // Upvalues will be processed after VM struct is built, for constant_to_value access

        let closure_handle = heap.alloc(Object::Closure(closure_without_upvalues));

        let frames = vec![CallFrame {
            closure: closure_handle,
            ip: 0,
            stack_start: 0,
            is_module: false,
            module_path: None,
            prev_globals: None,
        }];

        let mut vm = Self {
            pid,
            stack: Vec::with_capacity(2048),
            frames,
            globals: HashMap::new(),
            builtins: HashMap::new(),
            global_cache: HashMap::new(),
            heap,
            shared_heap,
            open_upvalues: Vec::new(),
            loaded_modules: HashMap::new(),
            exception_frames: Vec::new(),
            debug_ctx: None,
            // Spawned actors start with NO capabilities - must be granted explicitly
            security_context: SecurityContext::new(),
            resource_tracker: ResourceTracker::new(ResourceLimits::default()),
        };

        for val_const in captured_upvalues {
            let val = vm.constant_to_value(&val_const);
            let uv = Object::Upvalue(pulse_core::object::Upvalue {
                location: None,
                closed: Some(val),
            });
            new_upvalues.push(vm.heap.alloc_object(uv));
        }

        if let Some(Object::Closure(c)) = vm.heap.get_mut(closure_handle) {
            c.upvalues = new_upvalues;
        }

        for (k, c) in inherited_globals {
            let v = vm.constant_to_value(&c);
            vm.globals.insert(k, v);
        }

        vm.push(Value::Obj(closure_handle));

        if !arguments.is_empty() {
            // First argument is the closure, which we've ALREADY pushed as closure_handle.
            // We just need to push the REST of the arguments.
            for arg in arguments.iter().skip(1) {
                let v = vm.constant_to_value(arg);
                vm.push(v);
            }

            // The function expects to find the closure at stack_start + 0, args at + 1, + 2, etc.
            // We pushed closure_handle at stack[0], and the remaining arguments after it.
            // So `stack_start` should be 0. We don't need a second CallFrame!
            // Wait, let's just make the existing frame correct instead of pushing a second one.
            // We already initialized `frames[0]` with `stack_start = 0`.
            // So we don't need to push a new frame.

            // Just update `ip` or leave it. Frame 0 is correctly aimed at `closure_handle` with stack_start 0!
        }

        vm.register_all_natives();
        vm
    }

    /// Execute a new chunk in the context of this VM (useful for REPL)
    pub async fn execute_chunk(&mut self, chunk: Chunk) -> VMStatus {
        let function = Function {
            arity: 0,
            chunk: Arc::new(chunk),
            name: "repl_snippet".to_string(),
            upvalue_count: 0,
            module_path: None,
        };
        // let func_handle = self.heap.alloc(Object::Function(function.clone()));
        let closure = Closure {
            function,
            upvalues: Vec::new(),
        };
        let closure_handle = self.heap.alloc(Object::Closure(closure));

        let frame = CallFrame {
            closure: closure_handle,
            ip: 0,
            stack_start: self.stack.len(),
            is_module: false,
            module_path: None,
            prev_globals: None,
        };

        self.push(Value::Obj(closure_handle));
        self.frames.push(frame);

        self.run(usize::MAX).await
    }

    pub fn get_current_chunk(&self) -> PulseResult<Arc<Chunk>> {
        let frame = self
            .frames
            .last()
            .ok_or(PulseError::InternalError("No frame".into()))?;
        let closure = self
            .heap
            .get(frame.closure)
            .ok_or(PulseError::InternalError("Closure not found".into()))?;
        match closure {
            Object::Closure(c) => Ok(c.function.chunk.clone()),
            _ => Err(PulseError::InternalError("Frame closure invalid".into())),
        }
    }
    pub fn get_current_chunk_const(&self, idx: usize) -> PulseResult<Constant> {
        let chunk = self.get_current_chunk()?;
        if idx >= chunk.constants.len() {
            return Err(PulseError::InternalError(format!(
                "Constant index out of bounds: {} >= {}. Last op might have had bad operand.",
                idx,
                chunk.constants.len()
            )));
        }
        Ok(chunk.constants[idx].clone())
    }

    pub fn define_native(&mut self, name: &str, func: pulse_core::value::SyncNativeFn) {
        let native = NativeFn {
            name: name.to_string(),
            func: pulse_core::value::NativeFunctionKind::Sync(func),
        };
        let handle = self.heap.alloc(Object::NativeFn(native));
        self.builtins.insert(name.to_string(), Value::Obj(handle));
    }

    pub fn define_native_async(&mut self, name: &str, func: pulse_core::value::AsyncNativeFn) {
        let native = NativeFn {
            name: name.to_string(),
            func: pulse_core::value::NativeFunctionKind::Async(func),
        };
        let handle = self.heap.alloc(Object::NativeFn(native));
        self.builtins.insert(name.to_string(), Value::Obj(handle));
    }

    // ====================================================================
    // CAPABILITY-BASED SECURITY API
    // ====================================================================

    /// Grant a single capability to this VM
    pub fn with_capability(&mut self, cap: Capability) -> &mut Self {
        self.security_context.grant(cap);
        self
    }

    /// Grant multiple capabilities to this VM
    pub fn with_capabilities(&mut self, caps: Vec<Capability>) -> &mut Self {
        for cap in caps {
            self.security_context.grant(cap);
        }
        self
    }

    /// Revoke a capability from this VM
    pub fn revoke_capability(&mut self, cap: Capability) -> &mut Self {
        self.security_context.revoke(cap);
        self
    }

    /// Check if this VM has a specific capability
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.security_context.has_capability(cap)
    }

    /// Check capability and return error if denied
    pub fn check_capability(&self, cap: Capability) -> PulseResult<()> {
        self.security_context.check_capability(cap)
    }

    // ====================================================================
    // RESOURCE LIMITS API
    // ====================================================================

    /// Set resource limits for this VM
    pub fn with_resource_limits(&mut self, limits: ResourceLimits) -> &mut Self {
        self.resource_tracker = ResourceTracker::new(limits);
        self
    }

    /// Check memory limit against current heap size
    pub fn check_memory_limit(&self) -> PulseResult<()> {
        let heap_size = self.heap.size();
        self.resource_tracker.check_memory(heap_size)
    }

    /// Check CPU time limit
    pub fn check_cpu_time_limit(&self) -> PulseResult<()> {
        self.resource_tracker.check_cpu_time()
    }

    /// Check file descriptor limit
    pub fn check_file_descriptor_limit(&self) -> PulseResult<()> {
        self.resource_tracker.check_file_descriptor()
    }

    fn extract_closure_upvalues(&self, closure_val: &Value) -> Result<Vec<Constant>, PulseError> {
        if let Value::Obj(h) = closure_val {
            if let Some(Object::Closure(c)) = self.heap.get(*h) {
                return self.extract_closure_upvalues_from_obj(c);
            }
        }
        Ok(Vec::new())
    }

    fn extract_closure_upvalues_from_obj(&self, c: &Closure) -> Result<Vec<Constant>, PulseError> {
        let mut captured = Vec::new();
        for &uv_h in &c.upvalues {
            if let Some(Object::Upvalue(uv)) = self.heap.get(uv_h) {
                let val = if let Some(loc) = uv.location {
                    self.stack[loc]
                } else {
                    uv.closed.expect("Closed upvalue must have a value")
                };
                captured.push(self.value_to_constant(&val, 0)?);
            } else {
                captured.push(Constant::Unit);
            }
        }
        Ok(captured)
    }

    pub async fn run(&mut self, mut steps: usize) -> VMStatus {
        while steps > 0 {
            // Check bounds? read_byte will panic if out of bounds, or result in error?
            // Better to check.
            let (current_ip, current_line, frame_depth) = {
                let frame = if let Some(f) = self.frames.last() {
                    f
                } else {
                    return VMStatus::Error(PulseError::InternalError("No frame".into()));
                };
                let closure = if let Some(c) = self.heap.get(frame.closure) {
                    c
                } else {
                    return VMStatus::Error(PulseError::InternalError("Closure not found".into()));
                };
                let chunk = match closure {
                    Object::Closure(c) => &c.function.chunk,
                    _ => {
                        return VMStatus::Error(PulseError::InternalError(
                            "Frame closure invalid".into(),
                        ))
                    }
                };
                if frame.ip >= chunk.code.len() {
                    return VMStatus::Halted; // Or Return from script?
                }
                let line = chunk.lines.get(frame.ip).copied().unwrap_or(0);
                (frame.ip, line, self.frames.len())
            };

            // Debug: check breakpoints and step mode
            if let Some(ref mut ctx) = self.debug_ctx {
                if ctx.should_pause(current_ip, current_line, frame_depth) {
                    ctx.mark_paused(current_ip);
                    return VMStatus::Paused;
                }
            }

            steps -= 1;

            let op_code = self.read_byte();
            let op = Op::from(op_code);
            // println!("Op: {:?}", op); // Tracing disabled for performance

            // println!("Op: {:?}", op); // Tracing disabled for performance

            match self.execute_op(op).await {
                Ok(status) => {
                    if status != VMStatus::Running {
                        return status;
                    }
                }
                Err(e) => return VMStatus::Error(e),
            }
        }
        VMStatus::Running
    }

    /// Recursively deep-copies a Value from the actor's heap into a detached Constant
    pub fn value_to_constant(&self, val: &Value, depth: usize) -> PulseResult<Constant> {
        if depth > 100 {
            return Err(PulseError::RuntimeError(
                "Message serialization too deep or cyclical".into(),
            ));
        }
        match val {
            Value::Bool(b) => Ok(Constant::Bool(*b)),
            Value::Int(i) => Ok(Constant::Int(*i)),
            Value::Float(f) => Ok(Constant::Float(*f)),
            Value::Unit => Ok(Constant::Unit),
            Value::Pid(pid) => Ok(Constant::Pid(*pid)),
            Value::Obj(handle) => {
                if let Some(obj) = self.heap.get(*handle) {
                    match obj {
                        Object::String(s) => Ok(Constant::String(s.clone())),
                        Object::List(l) => {
                            let mut const_list = Vec::with_capacity(l.len());
                            for item in l {
                                const_list.push(self.value_to_constant(item, depth + 1)?);
                            }
                            Ok(Constant::List(const_list))
                        }
                        Object::Map(m) => {
                            let mut const_map = std::collections::HashMap::with_capacity(m.len());
                            for (k, v) in m {
                                const_map.insert(k.clone(), self.value_to_constant(v, depth + 1)?);
                            }
                            Ok(Constant::Map(const_map))
                        }
                        Object::Socket(s) => Ok(Constant::Socket(s.clone())),
                        Object::Listener(l) => Ok(Constant::Listener(l.clone())),
                        Object::WebSocket(w) => Ok(Constant::WebSocket(w.clone())),
                        Object::SharedBuffer(sm) => Ok(Constant::SharedMemory(sm.clone())),
                        Object::Function(f) => Ok(Constant::Function(Box::new(f.clone()))),
                        Object::Closure(c) => {
                            let upvals = self.extract_closure_upvalues_from_obj(c)?;
                            Ok(Constant::Closure(Box::new(c.function.clone()), upvals))
                        }
                        Object::NativeFn(n) => Ok(Constant::NativeFn(n.clone())),
                        Object::Module(m) => {
                            let mut const_map = std::collections::HashMap::with_capacity(m.len());
                            for (k, v) in m {
                                const_map.insert(k.clone(), self.value_to_constant(v, depth + 1)?);
                            }
                            Ok(Constant::Module(const_map))
                        }
                        _ => Err(PulseError::RuntimeError(
                            "Cannot send this complex object type yet".into(),
                        )),
                    }
                } else {
                    Err(PulseError::RuntimeError("Cannot send freed object".into()))
                }
            }
        }
    }

    /// Recursively deep-copies a detached Constant into a Value allocated on this VM's heap
    pub fn constant_to_value(&mut self, c: &Constant) -> Value {
        match c {
            Constant::Bool(b) => Value::Bool(*b),
            Constant::Int(i) => Value::Int(*i),
            Constant::Float(f) => Value::Float(*f),
            Constant::Unit => Value::Unit,
            Constant::String(s) => {
                let h = self.heap.alloc(Object::String(s.clone()));
                Value::Obj(h)
            }
            Constant::Function(f) => {
                let h = self.heap.alloc(Object::Function(*f.clone()));
                Value::Obj(h)
            }
            Constant::NativeFn(n) => {
                let h = self.heap.alloc(Object::NativeFn(n.clone()));
                Value::Obj(h)
            }
            Constant::Closure(f, upvals) => {
                let mut new_upvalues = Vec::with_capacity(upvals.len());
                for uv_const in upvals {
                    let val = self.constant_to_value(uv_const);
                    let uv_obj = Object::Upvalue(pulse_core::object::Upvalue {
                        location: None,
                        closed: Some(val),
                    });
                    new_upvalues.push(self.heap.alloc_object(uv_obj));
                }
                let closure = Closure {
                    function: *f.clone(),
                    upvalues: new_upvalues,
                };
                let h = self.heap.alloc(Object::Closure(closure));
                Value::Obj(h)
            }
            Constant::Module(m) => {
                let mut new_map = std::collections::HashMap::with_capacity(m.len());
                for (k, v) in m {
                    new_map.insert(k.clone(), self.constant_to_value(v));
                }
                let h = self.heap.alloc(Object::Module(new_map));
                Value::Obj(h)
            }
            Constant::SharedMemory(sm) => {
                let h = self.heap.alloc(Object::SharedBuffer(sm.clone()));
                Value::Obj(h)
            }
            Constant::Socket(s) => {
                let h = self.heap.alloc(Object::Socket(s.clone()));
                Value::Obj(h)
            }
            Constant::Listener(l) => {
                let h = self.heap.alloc(Object::Listener(l.clone()));
                Value::Obj(h)
            }
            Constant::WebSocket(w) => {
                let h = self.heap.alloc(Object::WebSocket(w.clone()));
                Value::Obj(h)
            }
            Constant::List(l) => {
                let mut new_list = Vec::with_capacity(l.len());
                for item in l {
                    new_list.push(self.constant_to_value(item));
                }
                let h = self.heap.alloc(Object::List(new_list));
                Value::Obj(h)
            }
            Constant::Map(m) => {
                let mut new_map = std::collections::HashMap::with_capacity(m.len());
                for (k, v) in m {
                    new_map.insert(k.clone(), self.constant_to_value(v));
                }
                let h = self.heap.alloc(Object::Map(new_map));
                Value::Obj(h)
            }
            Constant::Pid(pid) => Value::Pid(*pid),
        }
    }

    async fn execute_op(&mut self, op: Op) -> PulseResult<VMStatus> {
        macro_rules! op_match {
            ($op:expr) => {
                #[allow(clippy::collapsible_match)]
                match $op {
                    Op::Halt => return Ok(VMStatus::Halted),

                    Op::Pop => {
                        self.pop()?;
                        return Ok(VMStatus::Running);
                    }

                    Op::Dup => {
                        let val = self.peek(0).clone();
                        self.push(val);
                        return Ok(VMStatus::Running);
                    }

                    Op::IsList => {
                        let is_list = match self.peek(0) {
                            Value::Obj(h) => {
                                 let obj = self.heap.get(*h).ok_or(PulseError::RuntimeError("Invalid object handle".into()))?;
                                 matches!(obj, Object::List(_))
                            },
                            _ => false,
                        };
                        self.push(Value::Bool(is_list));
                        return Ok(VMStatus::Running);
                    }

                    Op::IsMap => {
                        let is_map = match self.peek(0) {
                            Value::Obj(h) => {
                                 let obj = self.heap.get(*h).ok_or(PulseError::RuntimeError("Invalid object handle".into()))?;
                                 matches!(obj, Object::Map(_))
                            },
                            _ => false,
                        };
                        self.push(Value::Bool(is_map));
                        return Ok(VMStatus::Running);
                    }

                    Op::Len => {
                        let val = self.peek(0);
                        let len = match val {
                             Value::Obj(h) => {
                                 let obj = self.heap.get(*h).ok_or(PulseError::RuntimeError("Invalid object handle".into()))?;
                                 match obj {
                                     Object::String(s) => s.len(),
                                     Object::List(l) => l.len(),
                                     Object::Map(m) => m.len(),
                                     Object::Set(s) => s.len(),
                                     Object::Queue(q) => q.len(),
                                     _ => return Err(PulseError::TypeMismatch{expected: "collection".into(), got: val.type_name()}),
                                 }
                             },
                             _ => return Err(PulseError::TypeMismatch{expected: "collection".into(), got: val.type_name()}),
                        };
                        self.push(Value::Int(len as i64));
                        return Ok(VMStatus::Running);
                    }

                    Op::MapContainsKey => {
                        let key_val = self.pop()?;
                        let map_val = self.peek(0);

                        let key = match key_val {
                            Value::Obj(h) => self.get_string(h)?,
                            _ => return Err(PulseError::TypeMismatch{expected: "string key".into(), got: key_val.type_name()}),
                        };

                        let found = match map_val {
                             Value::Obj(h) => {
                                 let obj = self.heap.get(*h).ok_or(PulseError::RuntimeError("Invalid object handle".into()))?;
                                 matches!(obj, Object::Map(m) if m.contains_key(&key))
                             },
                             _ => false,
                        };
                        self.push(Value::Bool(found));
                        return Ok(VMStatus::Running);
                    }

                    Op::Slide => {
                         let count = self.read_byte() as usize; // Number of items to drop UNDER the top value
                         // Stack: [..., Drop1, Drop2, Result]
                         // We want: [..., Result]

                         let result = self.pop()?;
                         for _ in 0..count {
                             self.pop()?;
                         }
                         self.push(result);
                         return Ok(VMStatus::Running);
                    }

                    Op::ToString => {
                        let val = self.pop()?;
                        let str_val = match val {
                            Value::Int(i) => i.to_string(),
                            Value::Float(f) => f.to_string(),
                            Value::Bool(b) => b.to_string(),
                            Value::Unit => "unit".to_string(),
                            Value::Pid(p) => format!("<pid {}:{}>", p.node_id, p.sequence),
                            Value::Obj(h) => {
                                let obj = self.heap.get(h).ok_or(PulseError::RuntimeError("Invalid object handle".into()))?;
                                match obj {
                                    Object::String(_s) => {
                                        // Already a string, just return it
                                        self.push(val);
                                        return Ok(VMStatus::Running);
                                    }
                                    Object::List(_) => "<list>".to_string(),
                                    Object::Map(_) => "<map>".to_string(),
                                    Object::Closure(_) => "<closure>".to_string(),
                                    Object::NativeFn(n) => format!("<native fn {}>", n.name),
                                    Object::Upvalue(_) => "<upvalue>".to_string(),
                                    Object::Function(_) => "<function>".to_string(),
                                    Object::Module(_) => "<module>".to_string(),
                                    Object::Class(c) => format!("<class {}>", c.name),
                                    Object::Instance(i) => format!("<instance {}>", i.class.name),
                                    Object::BoundMethod(_) => "<bound method>".to_string(),

                                    Object::Set(s) => format!("<set len={}>", s.len()),
                                    Object::Queue(q) => format!("<queue len={}>", q.len()),
                                    Object::SharedMemory(sm) => format!("<shared memory locked={}>", sm.locked),
                                    Object::AtomicInt(ai) => format!("<atomic {}>", ai.load(std::sync::atomic::Ordering::SeqCst)),
                                    Object::Socket(_) => "<socket>".to_string(),
                                    Object::SharedBuffer(_) => "<shared buffer>".to_string(),
                                    Object::Listener(_) => "<listener>".to_string(),
                                    Object::Regex(_) => "<regex>".to_string(),
                                    Object::WebSocket(_) => "<websocket>".to_string(),
                                }
                            }
                        };
                        let handle = self.heap.alloc(Object::String(str_val));
                        self.push(Value::Obj(handle));
                        return Ok(VMStatus::Running);
                    }

                    Op::Slice => {
                        // Slice: Pop list, push tail starting at index.
                        // Assuming arg is 1 byte index? Or popped off stack?
                        // For [head | tail], we want tail from index 1.
                        // Let's assume stack: [List, Index]. Pop Index, Pop List, Push Tail.

                        let index_val = self.pop()?;
                        let list_val = self.pop()?;

                        let start_index = match index_val {
                            Value::Int(i) => {
                                if i < 0 { return Err(PulseError::RuntimeError("Negative slice index".into())); }
                                i as usize
                            },
                            _ => return Err(PulseError::TypeMismatch{expected: "int index".into(), got: index_val.type_name()}),
                        };

                        let tail_list = match list_val {
                            Value::Obj(h) => {
                                let obj = self.heap.get(h).ok_or(PulseError::RuntimeError("Invalid object handle".into()))?;
                                if let Object::List(list) = obj {
                                    if start_index > list.len() {
                                        Vec::new() // Empty result
                                    } else {
                                        list[start_index..].to_vec()
                                    }
                                } else {
                                    return Err(PulseError::TypeMismatch{expected: "list".into(), got: "object".into()});
                                }
                            },
                            _ => return Err(PulseError::TypeMismatch{expected: "list".into(), got: list_val.type_name()}),
                        };

                        let tail_obj = self.heap.alloc(Object::List(tail_list));
                        self.push(Value::Obj(tail_obj));

                        return Ok(VMStatus::Running);
                    }

                    Op::Eq => {
                        let b = self.pop()?;
                        let a = self.pop()?;

                        let is_equal = match (&a, &b) {
                            (Value::Obj(h1), Value::Obj(h2)) => {
                                if h1 == h2 {
                                    true
                                } else {
                                    // Deep compare strings
                                    let s1 = self.get_string(*h1);
                                    let s2 = self.get_string(*h2);
                                    match (s1, s2) {
                                        (Ok(str1), Ok(str2)) => str1 == str2,
                                        _ => false // References different, contents not both strings or not equal
                                    }
                                }
                            },
                            _ => a == b,
                        };

                        self.push(Value::Bool(is_equal));
                        return Ok(VMStatus::Running);
                    }

                    Op::Neq => {
                        let b = self.pop()?;
                        let a = self.pop()?;

                        let is_equal = match (&a, &b) {
                            (Value::Obj(h1), Value::Obj(h2)) => {
                                if h1 == h2 {
                                    true
                                } else {
                                    // Deep compare strings
                                    let s1 = self.get_string(*h1);
                                    let s2 = self.get_string(*h2);
                                    match (s1, s2) {
                                        (Ok(str1), Ok(str2)) => str1 == str2,
                                        _ => false
                                    }
                                }
                            },
                            _ => a == b,
                        };

                        self.push(Value::Bool(!is_equal));
                        return Ok(VMStatus::Running);
                    }

                    Op::Gt => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => self.push(Value::Bool(v1 > v2)),
                            (Value::Float(v1), Value::Float(v2)) => self.push(Value::Bool(v1 > v2)),
                            (Value::Obj(h1), Value::Obj(h2)) => {
                                let s1 = self.get_string(h1)?;
                                let s2 = self.get_string(h2)?;
                                self.push(Value::Bool(s1 > s2));
                            },
                            (v1, v2) => return Err(PulseError::TypeMismatch{expected: "number or string".into(), got: format!("{:?} vs {:?}", v1.type_name(), v2.type_name())}),
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Lt => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => self.push(Value::Bool(v1 < v2)),
                            (Value::Float(v1), Value::Float(v2)) => self.push(Value::Bool(v1 < v2)),
                            (Value::Obj(h1), Value::Obj(h2)) => {
                                let s1 = self.get_string(h1)?;
                                let s2 = self.get_string(h2)?;
                                self.push(Value::Bool(s1 < s2));
                            },
                            (v1, v2) => return Err(PulseError::TypeMismatch{expected: "number or string".into(), got: format!("{:?} vs {:?}", v1.type_name(), v2.type_name())}),
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::And => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        let result = self.is_truthy(&a) && self.is_truthy(&b);
                        self.push(Value::Bool(result));
                        return Ok(VMStatus::Running);
                    }

                    Op::Or => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        let result = self.is_truthy(&a) || self.is_truthy(&b);
                        self.push(Value::Bool(result));
                        return Ok(VMStatus::Running);
                    }

                    Op::Const => {
                        let const_idx = self.read_u16() as usize; // Changed from read_byte() to read_u16() to support larger indices
                        let constant = self.get_current_chunk_const(const_idx)?;
                        let val = match constant {
                            Constant::Bool(b) => Value::Bool(b),
                            Constant::Int(i) => Value::Int(i),
                            Constant::Float(f) => Value::Float(f),
                            Constant::Unit => Value::Unit,
                            Constant::String(_) | Constant::Function(_) | Constant::Socket(_) | Constant::WebSocket(_) | Constant::SharedMemory(_) | Constant::Listener(_) | Constant::List(_) | Constant::Map(_) | Constant::NativeFn(_) | Constant::Closure(..) | Constant::Module(_) | Constant::Pid(_) => {
                                self.constant_to_value(&constant)
                            }
                        };
                        self.push(val);
                        return Ok(VMStatus::Running);
                    }

                    Op::Add => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => match v1.checked_add(v2) {
                                Some(sum) => self.push(Value::Int(sum)),
                                None => return Err(PulseError::RuntimeError("Integer overflow".into())),
                            },
                            (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 + v2)),
                            (Value::Int(v1), Value::Float(v2)) => self.push(Value::Float(v1 as f64 + v2)),
                            (Value::Float(v1), Value::Int(v2)) => self.push(Value::Float(v1 + v2 as f64)),
                            (Value::Obj(h1), Value::Obj(h2)) => {
                                // Check if both are strings
                                let s1 = self.get_string(h1)?;
                                let s2 = self.get_string(h2)?;
                                let new_s = s1 + &s2;
                                let handle = self.heap.alloc(Object::String(new_s));
                                self.push(Value::Obj(handle));
                            },
                            // Check for String + Any or Any + String for concatenation?
                            // For now, let's keep strict string+string or number+number
                            (v1, v2) => return Err(PulseError::TypeMismatch{expected: "numbers or strings".into(), got: format!("{:?} + {:?}", v1.type_name(), v2.type_name())}),
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Sub => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => match v1.checked_sub(v2) {
                                Some(diff) => self.push(Value::Int(diff)),
                                None => return Err(PulseError::RuntimeError("Integer overflow".into())),
                            },
                            (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 - v2)),
                            (Value::Int(v1), Value::Float(v2)) => self.push(Value::Float(v1 as f64 - v2)),
                            (Value::Float(v1), Value::Int(v2)) => self.push(Value::Float(v1 - v2 as f64)),
                            (v1, v2) => return Err(PulseError::TypeMismatch{expected: "numbers".into(), got: format!("{:?} - {:?}", v1.type_name(), v2.type_name())}),
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Mul => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => match v1.checked_mul(v2) {
                                Some(prod) => self.push(Value::Int(prod)),
                                None => return Err(PulseError::RuntimeError("Integer overflow".into())),
                            },
                            (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 * v2)),
                            (Value::Int(v1), Value::Float(v2)) => self.push(Value::Float(v1 as f64 * v2)),
                            (Value::Float(v1), Value::Int(v2)) => self.push(Value::Float(v1 * v2 as f64)),
                            (v1, v2) => return Err(PulseError::TypeMismatch{expected: "numbers".into(), got: format!("{:?} * {:?}", v1.type_name(), v2.type_name())}),
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Div => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => {
                                if v2 == 0 { return Err(PulseError::RuntimeError("Division by zero".into())); }
                                match v1.checked_div(v2) {
                                  Some(quot) => self.push(Value::Int(quot)),
                                  None => return Err(PulseError::RuntimeError("Integer overflow".into())),
                                }
                            },
                            (Value::Float(v1), Value::Float(v2)) => {
                                if v2 == 0.0 { return Err(PulseError::RuntimeError("Float division by zero".into())); }
                                self.push(Value::Float(v1 / v2))
                            },
                            (Value::Int(v1), Value::Float(v2)) => {
                                if v2 == 0.0 { return Err(PulseError::RuntimeError("Float division by zero".into())); }
                                self.push(Value::Float(v1 as f64 / v2))
                            },
                            (Value::Float(v1), Value::Int(v2)) => {
                                if v2 == 0 { return Err(PulseError::RuntimeError("Division by zero".into())); }
                                self.push(Value::Float(v1 / v2 as f64))
                            },
                            (v1, v2) => return Err(PulseError::TypeMismatch{expected: "numbers".into(), got: format!("{:?} / {:?}", v1.type_name(), v2.type_name())}),
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Mod => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => {
                                if v2 == 0 { return Err(PulseError::RuntimeError("Modulo by zero".into())); }
                                self.push(Value::Int(v1 % v2))
                            },
                            (Value::Float(v1), Value::Float(v2)) => {
                                if v2 == 0.0 { return Err(PulseError::RuntimeError("Modulo by zero".into())); }
                                self.push(Value::Float(v1 % v2))
                            },
                            (Value::Int(v1), Value::Float(v2)) => {
                                if v2 == 0.0 { return Err(PulseError::RuntimeError("Modulo by zero".into())); }
                                self.push(Value::Float(v1 as f64 % v2))
                            },
                            (Value::Float(v1), Value::Int(v2)) => {
                                if v2 == 0 { return Err(PulseError::RuntimeError("Modulo by zero".into())); }
                                self.push(Value::Float(v1 % v2 as f64))
                            },
                            (v1, v2) => return Err(PulseError::TypeMismatch{expected: "numbers".into(), got: format!("{:?} % {:?}", v1.type_name(), v2.type_name())}),
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::BitAnd => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => self.push(Value::Int(v1 & v2)),
                            (v1, v2) => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "int".into(),
                                    got: format!("{} & {}", v1.type_name(), v2.type_name()),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::BitOr => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => self.push(Value::Int(v1 | v2)),
                            (v1, v2) => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "int".into(),
                                    got: format!("{} | {}", v1.type_name(), v2.type_name()),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::BitXor => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        match (a, b) {
                            (Value::Int(v1), Value::Int(v2)) => self.push(Value::Int(v1 ^ v2)),
                            (v1, v2) => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "int".into(),
                                    got: format!("{} ^ {}", v1.type_name(), v2.type_name()),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::BitNot => {
                        let a = self.pop()?;
                        match a {
                            Value::Int(v) => self.push(Value::Int(!v)),
                            v => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "int".into(),
                                    got: v.type_name(),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Shl => {
                        let shift = self.pop()?;
                        let value = self.pop()?;
                        match (value, shift) {
                            (Value::Int(v), Value::Int(s)) => {
                                if !(0..64).contains(&s) {
                                    return Err(PulseError::RuntimeError(
                                        "Shift amount must be between 0 and 63".into(),
                                    ));
                                }
                                match v.checked_shl(s as u32) {
                                    Some(res) => self.push(Value::Int(res)),
                                    None => {
                                        return Err(PulseError::RuntimeError(
                                            "Shift left overflow".into(),
                                        ))
                                    }
                                }
                            }
                            (v1, v2) => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "int << int".into(),
                                    got: format!("{} << {}", v1.type_name(), v2.type_name()),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Shr => {
                        let shift = self.pop()?;
                        let value = self.pop()?;
                        match (value, shift) {
                            (Value::Int(v), Value::Int(s)) => {
                                if !(0..64).contains(&s) {
                                    return Err(PulseError::RuntimeError(
                                        "Shift amount must be between 0 and 63".into(),
                                    ));
                                }
                                match v.checked_shr(s as u32) {
                                    Some(res) => self.push(Value::Int(res)),
                                    None => {
                                        return Err(PulseError::RuntimeError(
                                            "Shift right overflow".into(),
                                        ))
                                    }
                                }
                            }
                            (v1, v2) => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "int >> int".into(),
                                    got: format!("{} >> {}", v1.type_name(), v2.type_name()),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Pow => {
                        let exponent = self.pop()?;
                        let base = self.pop()?;
                        match (base, exponent) {
                            (Value::Int(b), Value::Int(e)) => {
                                if e >= 0 {
                                    if let Some(result) = b.checked_pow(e as u32) {
                                        self.push(Value::Int(result));
                                    } else {
                                        return Err(PulseError::RuntimeError(
                                            "Integer overflow in pow".into(),
                                        ));
                                    }
                                } else {
                                    self.push(Value::Float((b as f64).powf(e as f64)));
                                }
                            }
                            (Value::Float(b), Value::Float(e)) => {
                                self.push(Value::Float(b.powf(e)));
                            }
                            (Value::Int(b), Value::Float(e)) => {
                                self.push(Value::Float((b as f64).powf(e)));
                            }
                            (Value::Float(b), Value::Int(e)) => {
                                self.push(Value::Float(b.powf(e as f64)));
                            }
                            (v1, v2) => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "number ** number".into(),
                                    got: format!("{} ** {}", v1.type_name(), v2.type_name()),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Append => {
                        let value = self.pop()?;
                        let target = self.pop()?;
                        match target {
                            Value::Obj(handle) => {
                                let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError(
                                    "Invalid list handle".into(),
                                ))?;

                                match obj {
                                    Object::List(list) => {
                                        list.push(value);
                                        self.push(Value::Unit);
                                    }
                                    _ => {
                                        return Err(PulseError::TypeMismatch {
                                            expected: "list".into(),
                                            got: "other object".into(),
                                        })
                                    }
                                }
                            }
                            other => {
                                return Err(PulseError::TypeMismatch {
                                    expected: "list".into(),
                                    got: other.type_name(),
                                })
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Jump => {
                        let offset = self.read_u16();
                        let frame = self.frames.last_mut().ok_or(PulseError::InternalError("No frame".into()))?;
                        let new_ip = frame.ip + offset as usize;
                        let chunk_len = {
                          let closure_handle = frame.closure;
                          let closure = if let Some(Object::Closure(c)) = self.heap.get(closure_handle) { c } else { return Err(PulseError::InternalError("Frame closure invalid".into())); };
                          closure.function.chunk.code.len()
                        };

                        if new_ip > chunk_len {
                            return Err(PulseError::RuntimeError("Jump out of bounds".into()));
                        }
                        frame.ip = new_ip;
                        return Ok(VMStatus::Running);
                    }

                    Op::JumpIfFalse => {
                        let offset = self.read_u16();
                        if !self.is_truthy(self.peek(0)) {
                             let frame = self.frames.last_mut().ok_or(PulseError::InternalError("No frame".into()))?;
                             let new_ip = frame.ip + offset as usize;
                             let chunk_len = {
                               let closure_handle = frame.closure;
                               let closure = if let Some(Object::Closure(c)) = self.heap.get(closure_handle) { c } else { return Err(PulseError::InternalError("Frame closure invalid".into())); };
                               closure.function.chunk.code.len()
                             };
                            if new_ip > chunk_len {
                                return Err(PulseError::RuntimeError("Jump out of bounds".into()));
                            }
                            frame.ip = new_ip;
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Call => {
                        let arg_count = self.read_byte() as usize;
                        let callee_val = self.peek(arg_count).clone();

                        match callee_val {
                            Value::Obj(handle) => {
                                 // Check type without holding borrow too long
                                 let obj_type = self.heap.get(handle).map(|o| match o {
                                     Object::NativeFn(_) => 1,
                                     Object::Closure(_) => 2,
                                     Object::Class(_) => 3,
                                     Object::BoundMethod(_) => 4,
                                     _ => 0,
                                 }).unwrap_or(0);

                                 if obj_type == 1 { // Native
                                     let native = if let Some(Object::NativeFn(n)) = self.heap.get(handle) { n.clone() } else { return Err(PulseError::InternalError("Expected NativeFn".into())); };
                                     let args_start = self.stack.len() - arg_count;
                                     let args = self.stack[args_start..].to_vec();


                                     self.stack.truncate(args_start - 1);

                                     let result = match native.func {
                                         pulse_core::value::NativeFunctionKind::Sync(f) => f(self, &args)?,
                                         pulse_core::value::NativeFunctionKind::Async(f) => f(self, &args).await?,
                                     };

                                     self.push(result);
                                     return Ok(VMStatus::Running);
                                 } else if obj_type == 2 { // Closure
                                     let arity = if let Some(Object::Closure(c)) = self.heap.get(handle) { c.function.arity } else { return Err(PulseError::InternalError("Expected Closure".into())); };
                                     if arg_count != arity {
                                         return Err(PulseError::RuntimeError(format!("Expected {} args, got {}", arity, arg_count)));
                                     }
                                                                  let frame = CallFrame {
                                          closure: handle,
                                          ip: 0,
                                          stack_start: self.stack.len() - arg_count - 1,
                                          is_module: false,
                                          module_path: None,
                                          prev_globals: None,
                                      };
                                     if self.frames.len() >= MAX_FRAMES {
                                         return Err(PulseError::StackOverflow);
                                     }
                                     self.frames.push(frame);
                                     return Ok(VMStatus::Running);
                                 } else if obj_type == 3 { // Class
                                     let class = if let Some(Object::Class(c)) = self.heap.get(handle) { c.clone() } else { return Err(PulseError::InternalError("Expected Class".into())); };
                                     let instance = Instance {
                                         class: Arc::new(class.clone()),
                                         fields: HashMap::new(),
                                     };
                                     let instance_handle = self.heap.alloc(Object::Instance(instance));
                                     let instance_val = Value::Obj(instance_handle);

                                     // Replace class on stack with instance
                                     let stack_idx = self.stack.len() - arg_count - 1;
                                     self.stack[stack_idx] = instance_val;

                                     // Check for init method
                                     if let Some(init_val) = class.methods.get("init") {
                                         match init_val {
                                             Value::Obj(h) => {
                                                 if let Some(Object::Closure(c)) = self.heap.get(*h) {
                                                     let arity = c.function.arity;
                                                     if arg_count != arity {
                                                         return Err(PulseError::RuntimeError(format!("Expected {} args for init, got {}", arity, arg_count)));
                                                     }
                                                     let frame = CallFrame {
                                                          closure: *h,
                                                          ip: 0,
                                                          stack_start: stack_idx, // 'this' is at stack_idx
                                                          is_module: false,
                                                          module_path: None,
                                                          prev_globals: None,
                                                      };
                                                     if self.frames.len() >= MAX_FRAMES {
                                                         return Err(PulseError::StackOverflow);
                                                     }
                                                     self.frames.push(frame);
                                                     return Ok(VMStatus::Running);
                                                 }
                                             },
                                             _ => {},
                                         }
                                     } else if arg_count != 0 {
                                         return Err(PulseError::RuntimeError(format!("Expected 0 args for class without init, got {}", arg_count)));
                                     }
                                     return Ok(VMStatus::Running);
                                 } else if obj_type == 4 { // BoundMethod

                                     let bound = if let Some(Object::BoundMethod(b)) = self.heap.get(handle) { b.clone() } else { return Err(PulseError::InternalError("Expected BoundMethod".into())); };
                                     let stack_idx = self.stack.len() - arg_count - 1;
                                     self.stack[stack_idx] = bound.receiver; // Set 'this'


                                     // Call the method (function/closure)
                                     // Note: We need to wrap function in closure if it's just a function?
                                     // But object.rs definition has method: Function.
                                     // Wait, BoundMethod logic needs to be consistent.
                                     // If I defined BoundMethod with `method: Function`, then I construct a temporary Closure?
                                     // Or assume it's just Function logic.
                                     // Actually, Classes usually store Closures as methods (captured upvalues?).
                                     // If Class methods are Closures. Then BoundMethod should store Closure!

                                     // Rethinking BoundMethod in object.rs:
                                     // I defined `pub method: Function`.
                                     // But `Class` stores `Value` in `methods` map.
                                     // If `Value` is `Obj(Closure)`, then `BoundMethod` should probably hold `Closure` or `ObjHandle`.

                                     // Let's assume for now BoundMethod holds `method: Function`.
                                     // But `Class` methods are `Value`.
                                     // I should update object.rs later if needed. For now assuming I can get Function from it.

                                     let arity = bound.method.arity;
                                     if arg_count != arity {
                                         return Err(PulseError::RuntimeError(format!("Expected {} args, got {}", arity, arg_count)));
                                     }

                                     // We need a handle for the closure to put in CallFrame.
                                     // If BoundMethod doesn't have a handle to a Closure, we must create one?
                                     // This is inefficient.
                                     // Ideally BoundMethod stores Handle to Closure.

                                     // TEMPORARY FIX: Create a closure on the fly.
                                     let closure = Closure {
                                         function: bound.method.clone(),
                                         upvalues: Vec::new(), // Methods usually don't capture upvalues from outside class?
                                     };
                                     let closure_handle = self.heap.alloc(Object::Closure(closure));

                                     let frame = CallFrame {
                                          closure: closure_handle,
                                          ip: 0,
                                          stack_start: stack_idx,
                                          is_module: false,
                                          module_path: None,
                                          prev_globals: None,
                                      };
                                     if self.frames.len() >= MAX_FRAMES {
                                         return Err(PulseError::StackOverflow);
                                     }
                                     self.frames.push(frame);
                                     return Ok(VMStatus::Running);

                                 } else {
                                     Err(PulseError::TypeMismatch{expected: "function".into(), got: "other object".into()})
                                 }
                            },
                            _ => Err(PulseError::TypeMismatch{expected: "function".into(), got: callee_val.type_name()}),
                        }
                    }

                    Op::Return => {
                        let mut result = self.pop()?;
                        let frame = self.frames.pop().ok_or(PulseError::RuntimeError("Return from top level".into()))?;

                        if frame.is_module {
                            // Capture exports
                            let exports = self.globals.clone();
                            let handle = self.heap.alloc(Object::Module(exports));
                            result = Value::Obj(handle);

                            if let Some(path) = frame.module_path {
                                self.loaded_modules.insert(path, handle);
                            }

                            // Restore previous globals
                            // Restore previous globals if they exist.
                            // If prev_globals is None (e.g. for "include" style imports),
                            // we keep the modified globals (namespace pollution), which is desired.
                            if let Some(prev) = frame.prev_globals {
                                self.globals = prev;
                            }
                        }

                        // Close upvalues for this frame's locals
                        self.close_upvalues(frame.stack_start);
                        self.stack.truncate(frame.stack_start);
                        self.push(result);

                        if self.frames.is_empty() {
                            return Ok(VMStatus::Halted);
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::Closure => {
                        let const_idx = self.read_u16() as usize;
                        // println!("Closure const_idx: {}", const_idx);
                        let constant = self.get_current_chunk_const(const_idx)?;
                        // println!("Closure constant type: {:?}", constant); // Just print, assume Debug
                        let function = match constant {
                            Constant::Function(f) => f,
                             _ => return Err(PulseError::TypeMismatch{expected: "function".into(), got: "other constant".into()}),
                        };

                        let mut upvalues = Vec::new();
                        for _ in 0..function.upvalue_count {
                            let is_local = self.read_byte() == 1;
                            let index = self.read_byte();
                            if is_local {
                                let frame_start = self.frames.last().ok_or(PulseError::InternalError("No frame".into()))?.stack_start;
                                upvalues.push(self.capture_upvalue(frame_start + index as usize));
                            } else {
                                // Capture from current closure's upvalues
                                let current_closure_handle = self.frames.last().ok_or(PulseError::InternalError("No frame".into()))?.closure;
                                if let Some(Object::Closure(c)) = self.heap.get(current_closure_handle) {
                                     if (index as usize) >= c.upvalues.len() {
                                         return Err(PulseError::InternalError(format!("Upvalue index {} out of bounds (len {})", index, c.upvalues.len())));
                                     }
                                     upvalues.push(c.upvalues[index as usize]);
                                }
                            }
                        }

                        let closure = Closure {
                            function: *function.clone(),
                            upvalues,
                        };
                        let handle = self.heap.alloc(Object::Closure(closure));
                        self.push(Value::Obj(handle));
                        return Ok(VMStatus::Running);
                    }

                    Op::GetUpvalue => {
                        let slot = self.read_byte();
                        let closure_handle = self.frames.last().ok_or(PulseError::InternalError("No frame".into()))?.closure;
                        let val = if let Some(Object::Closure(c)) = self.heap.get(closure_handle) {
                            if (slot as usize) >= c.upvalues.len() {
                                return Err(PulseError::InternalError(format!("Upvalue index {} out of bounds (len {})", slot, c.upvalues.len())));
                            }
                            let uv_handle = c.upvalues[slot as usize];
                            if let Some(Object::Upvalue(uv)) = self.heap.get(uv_handle) {
                                if let Some(loc) = uv.location {
                                    self.stack[loc].clone()
                                } else {
                                    uv.closed.as_ref().ok_or_else(|| PulseError::InternalError("Closed upvalue missing value".into()))?.clone()
                                }
                            } else { return Err(PulseError::RuntimeError(format!("Invalid upvalue: expected Upvalue at index {}, found {:?}", uv_handle.0, self.heap.get(uv_handle)))); }
                        } else { return Err(PulseError::RuntimeError("No closure in frame".into())); };

                        self.push(val);
                        return Ok(VMStatus::Running);
                    }

                    Op::SetUpvalue => {
                        let slot = self.read_byte();
                        let val = self.peek(0).clone();
                        let closure_handle = self.frames.last().ok_or(PulseError::InternalError("No frame".into()))?.closure;
                        if let Some(Object::Closure(c)) = self.heap.get(closure_handle) {
                            if (slot as usize) >= c.upvalues.len() {
                                return Err(PulseError::InternalError(format!("Upvalue index {} out of bounds (len {})", slot, c.upvalues.len())));
                            }
                            let uv_handle = c.upvalues[slot as usize];
                            if let Some(Object::Upvalue(uv)) = self.heap.get_mut(uv_handle) {
                                if let Some(loc) = uv.location {
                                    self.stack[loc] = val;
                                } else {
                                    uv.closed = Some(val);
                                }
                            } else { return Err(PulseError::RuntimeError(format!("Invalid upvalue: expected Upvalue at index {}, found {:?}", uv_handle.0, self.heap.get(uv_handle)))); }
                        } else { return Err(PulseError::RuntimeError("No closure in frame".into())); };
                        return Ok(VMStatus::Running);
                    }

                    Op::CloseUpvalue => {
                        self.close_upvalues(self.stack.len() - 1);
                        self.pop()?;
                        return Ok(VMStatus::Running);
                    }

                    Op::GetLocal => {
                        let slot = self.read_byte();
                        let frame_start = self.frames.last().map(|f| f.stack_start).unwrap_or(0);
                        let idx = frame_start + slot as usize;
                        // Check bounds?
                         if idx >= self.stack.len() {
                                return Err(PulseError::StackUnderflow);
                        }
                        let val = self.stack[idx].clone();
                        self.push(val);
                        return Ok(VMStatus::Running);
                    }

                    Op::SetLocal => {
                        let slot = self.read_byte();
                        let val = self.peek(0).clone();
                        let frame_start = self.frames.last().map(|f| f.stack_start).unwrap_or(0);
                        let idx = frame_start + slot as usize;
                        if idx >= self.stack.len() {
                            return Err(PulseError::StackUnderflow);
                        }
                        self.stack[idx] = val;
                        return Ok(VMStatus::Running);
                    }

                    Op::GetGlobal => {
                        let name_idx = self.read_u16() as usize; // Changed from read_byte() to read_u16() to support larger indices
                        let constant = self.get_current_chunk_const(name_idx)?;
                        let name = match constant {
                            Constant::String(s) => s.clone(),
                            _ => return Err(PulseError::RuntimeError("Global name must be string".into())),
                        };

                        // First check the cache
                        if let Some(cached_val) = self.global_cache.get(&name) {
                            self.push(cached_val.clone());
                            return Ok(VMStatus::Running);
                        } else {
                            // Not in cache, look in globals and builtins
                            let val = self.globals.get(&name)
                                .or_else(|| self.builtins.get(&name))
                                .ok_or_else(|| PulseError::UndefinedVariable(name.clone()))?
                                .clone();

                            // Cache the value for future access
                            self.global_cache.insert(name.clone(), val.clone());
                            self.push(val);
                            return Ok(VMStatus::Running);
                        }
                    }

                    Op::SetGlobal => {
                        let name_idx = self.read_u16() as usize; // Changed from read_byte() to read_u16() to support larger indices
                        let constant = self.get_current_chunk_const(name_idx)?;
                        let name = match constant {
                             Constant::String(s) => s.clone(),
                             _ => return Err(PulseError::RuntimeError("Global name must be string".into())),
                        };
                        let val = self.peek(0).clone();
                        if self.globals.contains_key(&name) {
                            self.globals.insert(name.clone(), val);
                            // Invalidate cache entry
                            self.global_cache.remove(&name);
                            return Ok(VMStatus::Running);
                        } else if self.builtins.contains_key(&name) {
                            return Err(PulseError::RuntimeError(format!("Cannot modify immutable builtin: {}", name)));
                        } else {
                            return Err(PulseError::UndefinedVariable(name));
                        }
                    }

                    Op::DefineGlobal => {
                        let name_idx = self.read_u16() as usize; // Changed from read_byte() to read_u16() to support larger indices
                        let constant = self.get_current_chunk_const(name_idx)?;
                        let name = match constant {
                             Constant::String(s) => s.clone(),
                             _ => return Err(PulseError::RuntimeError("Global name must be string".into())),
                        };
                        let val = self.pop()?;
                        self.globals.insert(name.clone(), val);
                        // Invalidate cache entry if it exists
                        self.global_cache.remove(&name);
                        return Ok(VMStatus::Running);
                    }

                    Op::Print => {
                        let val = self.pop()?;
                        self.print_value(&val);
                        println!();
                        return Ok(VMStatus::Running);
                    }

                    Op::PrintMulti => {
                        // Count how many values to print (read from next byte)
                        let count = self.read_byte() as usize;

                        // Collect values in reverse order (since stack is LIFO)
                        let mut values = Vec::with_capacity(count);
                        for _ in 0..count {
                            values.push(self.pop()?);
                        }
                        values.reverse(); // Reverse to get correct order

                        // Print all values separated by spaces
                        for (i, val) in values.iter().enumerate() {
                            if i > 0 {
                                print!(" ");
                            }
                            self.print_value(val);
                        }
                        println!();
                        return Ok(VMStatus::Running);
                    }

                    Op::Negate => {
                        let val = self.pop()?;
                        match val {
                            Value::Int(n) => match n.checked_neg() {
                                Some(neg) => self.push(Value::Int(neg)),
                                None => return Err(PulseError::RuntimeError("Integer overflow in negation".into())),
                            },
                            Value::Float(n) => self.push(Value::Float(-n)),
                            _ => return Err(PulseError::TypeMismatch{expected: "number".into(), got: "other".into()}),
                        }
                        return Ok(VMStatus::Running);
                    }
                    Op::Not => {
                        let val = self.pop()?;
                        let b = self.is_truthy(&val);
                        self.push(Value::Bool(!b));
                        return Ok(VMStatus::Running);
                    }

                    Op::Loop => {
                        let offset = self.read_u16();
                        let frame = self.frames.last_mut().ok_or(PulseError::InternalError("No frame for loop".into()))?;
                         match frame.ip.checked_sub(offset as usize) {
                            Some(new_ip) => frame.ip = new_ip,
                            None => return Err(PulseError::RuntimeError("Loop jump underflow".into())),
                        }
                        return Ok(VMStatus::Running);
                    }


                    Op::SelfId => {
                        self.push(Value::Pid(self.pid));
                        return Ok(VMStatus::Running);
                    }

                    Op::Send => {
                        let msg = self.pop()?;
                        let target_val = self.pop()?;

                        let target = match target_val {
                            Value::Pid(pid) => pid,
                            _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: target_val.type_name()}),
                        };

                        // Convert msg (Value) to Constant (owned) for safe transfer using deep copy
                        let msg_const = self.value_to_constant(&msg, 0)?;

                        return Ok(VMStatus::Send { target, msg: msg_const });
                    }

                    Op::Receive => {
                        // Return Blocked to signal Runtime to check mailbox
                        return Ok(VMStatus::Blocked);
                    }

                    Op::Spawn => {
                        let closure_val = self.pop()?;
                        let closure_const = self.value_to_constant(&closure_val, 0)?;
                        let captured_upvalues = self.extract_closure_upvalues(&closure_val)?;

                        let mut globals = std::collections::HashMap::new();
                        for (k, v) in &self.globals {
                            if let Ok(c) = self.value_to_constant(v, 0) {
                                globals.insert(k.clone(), c);
                            }
                        }
                        return Ok(VMStatus::Spawn(closure_const, captured_upvalues, globals));
                    }

                    Op::SpawnCall => {
                        let arity = self.read_byte() as usize;
                        let mut args = Vec::with_capacity(arity + 1);
                        for _ in 0..(arity + 1) {
                            args.push(self.pop()?);
                        }
                        args.reverse();

                        let mut arg_consts = Vec::with_capacity(args.len());
                        for a in &args {
                            arg_consts.push(self.value_to_constant(a, 0)?);
                        }

                        let callable_val = &args[0];
                        let captured_upvalues = self.extract_closure_upvalues(callable_val)?;

                        let mut globals = std::collections::HashMap::new();
                        for (k, v) in &self.globals {
                            if let Ok(c) = self.value_to_constant(v, 0) {
                                globals.insert(k.clone(), c);
                            }
                        }
                        return Ok(VMStatus::SpawnCall(arg_consts, captured_upvalues, globals));
                    }

                    Op::SpawnLink => {
                        let closure_val = self.pop()?;
                        let closure_const = self.value_to_constant(&closure_val, 0)?;
                        let captured_upvalues = self.extract_closure_upvalues(&closure_val)?;

                        let mut globals = std::collections::HashMap::new();
                        for (k, v) in &self.globals {
                            if let Ok(c) = self.value_to_constant(v, 0) {
                                globals.insert(k.clone(), c);
                            }
                        }
                        return Ok(VMStatus::SpawnLink(closure_const, captured_upvalues, globals));
                    }

                    Op::Link => {
                        let target_val = self.pop()?;
                        let target = match target_val {
                            Value::Pid(pid) => pid,
                            _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: target_val.type_name()}),
                        };
                        return Ok(VMStatus::Link(target));
                    }

                    Op::Monitor => {
                        let target_val = self.pop()?;
                        let target = match target_val {
                            Value::Pid(pid) => pid,
                            _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: target_val.type_name()}),
                        };
                        return Ok(VMStatus::Monitor(target));
                    }

                    Op::Register => {
                        let pid_val = self.pop()?;
                        let name_val = self.pop()?;

                        let pid = match pid_val {
                            Value::Pid(p) => p,
                            _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: pid_val.type_name()}),
                        };

                        let name = match name_val {
                            Value::Obj(h) => self.get_string(h)?,
                            _ => return Err(PulseError::TypeMismatch{expected: "string name".into(), got: name_val.type_name()}),
                        };

                        return Ok(VMStatus::Register(name, pid));
                    }

                    Op::Unregister => {
                        let name_val = self.pop()?;
                        let name = match name_val {
                            Value::Obj(h) => self.get_string(h)?,
                            _ => return Err(PulseError::TypeMismatch{expected: "string name".into(), got: name_val.type_name()}),
                        };
                        return Ok(VMStatus::Unregister(name));
                    }

                    Op::WhereIs => {
                        let name_val = self.pop()?;
                        let name = match name_val {
                            Value::Obj(h) => self.get_string(h)?,
                            _ => return Err(PulseError::TypeMismatch{expected: "string name".into(), got: name_val.type_name()}),
                        };
                        return Ok(VMStatus::WhereIs(name));
                    }

                    Op::Import => {
                        let path_idx = self.read_u16() as usize; // Changed from read_byte() to read_u16() to support larger indices
                        let constant = self.get_current_chunk_const(path_idx)?;
                        let path = match constant {
                            Constant::String(s) => s.clone(),
                            _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: "other constant".into() }),
                        };

                        let resolved = self.resolve_path(&path)?;
                        if let Some(handle) = self.loaded_modules.get(&resolved) {
                            // Module already loaded, push it to stack
                            self.push(Value::Obj(*handle));
                            return Ok(VMStatus::Running);
                        } else {
                            // Trigger import
                            return Ok(VMStatus::Import(resolved));
                        }
                    }
                    Op::BuildList => {
                        let count = self.read_byte() as usize;
                        let mut items = Vec::with_capacity(count);
                        // Pop items in reverse order
                        for _ in 0..count {
                            items.push(self.pop()?);
                        }
                        items.reverse(); // Restore valid order

                        let handle = self.heap.alloc(Object::List(items));
                        self.push(Value::Obj(handle));
                        return Ok(VMStatus::Running);
                    }

                    Op::BuildMap => {
                        let count = self.read_byte() as usize;
                        let mut map = HashMap::with_capacity(count);

                        for _ in 0..count {
                            let val = self.pop()?;
                            let key_val = self.pop()?;

                            let key = match key_val {
                                Value::Obj(h) => self.get_string(h)?,
                                _ => return Err(PulseError::TypeMismatch{expected: "string key".into(), got: key_val.type_name()}),
                            };

                            map.insert(key, val);
                        }

                        let handle = self.heap.alloc(Object::Map(map));
                        self.push(Value::Obj(handle));
                        return Ok(VMStatus::Running);
                    }

                    Op::GetIndex => {
                        let index_val = self.pop()?;
                        let target_val = self.pop()?;

                        match target_val {
                            Value::Obj(handle) => {
                                let obj = self.heap.get(handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
                                match obj {
                                    Object::List(vec) => {
                                        let idx = index_val.as_int()?;
                                        if idx < 0 || idx >= vec.len() as i64 {
                                            return Err(PulseError::RuntimeError(format!("Index out of bounds: {}", idx)));
                                        }
                                        let val = vec[idx as usize].clone();
                                        self.push(val);
                                    },
                                    Object::Map(map) => {
                                        let key = match index_val {
                                            Value::Obj(h) => self.get_string(h)?,
                                            _ => return Err(PulseError::TypeMismatch{expected: "string key".into(), got: index_val.type_name()}),
                                        };
                                        let val = map.get(&key).unwrap_or(&Value::Unit).clone();
                                        self.push(val);
                                    },
                                     Object::Module(map) => {
                                        let key = match index_val {
                                            Value::Obj(h) => self.get_string(h)?,
                                            _ => return Err(PulseError::TypeMismatch{expected: "string key".into(), got: index_val.type_name()}),
                                        };
                                        let val = map.get(&key).unwrap_or(&Value::Unit).clone();
                                        self.push(val);
                                    },
                            Object::Instance(inst) => {
                                        let key = match index_val {
                                            Value::Obj(h) => self.get_string(h)?,
                                            _ => return Err(PulseError::TypeMismatch{expected: "string key".into(), got: index_val.type_name()}),
                                        };
                                        if let Some(val) = inst.fields.get(&key) {
                                            self.push(val.clone());
                                        } else {
                                            // Check methods
                                            if let Some(method_val) = inst.class.methods.get(&key) {
                                                if let Value::Obj(h) = method_val {
                                                    if let Some(Object::Closure(c)) = self.heap.get(*h) {
                                                        let bound = BoundMethod {
                                                            receiver: Value::Obj(handle), // The instance handle
                                                            method: c.function.clone(), // Clone function metadata
                                                        };
                                                        let bound_handle = self.heap.alloc(Object::BoundMethod(bound));
                                                        self.push(Value::Obj(bound_handle));
                                                        return Ok(VMStatus::Running);
                                                    }
                                                }
                                            }
                                            // Method/Field not found
                                            // Method/Field not found
                                            return Err(PulseError::RuntimeError(format!("Method/Field '{}' not found in instance", key)));
                                        }
                                    },
                                    _ => return Err(PulseError::TypeMismatch{expected: "List, Map, Module, or Instance".into(), got: "other object".into()}),
                                }
                            },
                            _ => {
                                return Err(PulseError::TypeMismatch{expected: "List, Map, or Instance".into(), got: target_val.type_name()});
                            }
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::SetIndex => {
                        let val = self.pop()?; // Value to set
                        let index_val = self.pop()?; // Index/Key
                        let target_val = self.pop()?; // List/Map

                        match target_val {
                            Value::Obj(handle) => {
                                // Pre-resolve string key if index is a string object
                                // We must do this before borrowing heap mutably for target.
                                let key_string = if let Value::Obj(h) = index_val {
                                     Some(self.get_string(h)?)
                                } else {
                                     None
                                };

                                let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
                                match obj {
                                    Object::List(vec) => {
                                        let idx = index_val.as_int()?;
                                        if idx < 0 || idx >= vec.len() as i64 {
                                            return Err(PulseError::RuntimeError(format!("Index out of bounds: {}", idx)));
                                        }
                                        vec[idx as usize] = val.clone();
                                    },
                                    Object::Map(map) => {
                                         if let Some(key) = key_string {
                                             map.insert(key, val);
                                         } else {
                                             return Err(PulseError::TypeMismatch{expected: "string key".into(), got: index_val.type_name()});
                                         }
                                    },
                                    Object::Instance(inst) => {
                                         if let Some(key) = key_string {
                                             inst.fields.insert(key, val);
                                         } else {
                                             return Err(PulseError::TypeMismatch{expected: "string key".into(), got: index_val.type_name()});
                                         }
                                    },
                                    _ => return Err(PulseError::TypeMismatch{expected: "List, Map, or Instance".into(), got: "other object".into()}),
                                }
                            },
                            _ => return Err(PulseError::TypeMismatch{expected: "List or Map".into(), got: target_val.type_name()}),
                        }

                        self.push(val); // Expression result is assigned value
                        return Ok(VMStatus::Running);
                    }

                    Op::Unit => {
                        self.push(Value::Unit);
                        return Ok(VMStatus::Running);
                    }

                    Op::Try => {
                        // Read handler offset (u16)
                        let offset = self.read_u16() as usize;
                        let frame = self.frames.last().ok_or(PulseError::InternalError("Expected a value".into()))?;
                        let handler_ip = frame.ip + offset;

                        let exception_frame = ExceptionFrame {
                            handler_ip,
                            stack_depth: self.stack.len(),
                            frame_depth: self.frames.len(),
                        };
                        self.exception_frames.push(exception_frame);
                        return Ok(VMStatus::Running);
                    }

                    Op::EndTry => {
                        // Pop exception frame if present
                        self.exception_frames.pop();
                        return Ok(VMStatus::Running);
                    }

                    Op::Throw => {
                        let exception_val = self.pop()?;
                        self.unwind_to_handler(exception_val)?;
                        return Ok(VMStatus::Running);
                    }

                    Op::BuildClass => {
                        let name_idx = self.read_u16() as usize; // Changed from read_byte() to read_u16() to support larger indices
                        let has_super = self.read_byte();

                        let name = match self.get_current_chunk_const(name_idx)? {
                            Constant::String(s) => s,
                            _ => return Err(PulseError::RuntimeError("Class name must be string".into())),
                        };

                        let superclass = if has_super == 1 {
                            let super_idx = self.read_u16() as usize; // Changed from read_byte() to read_u16() to support larger indices
                            let super_val = self.get_current_chunk_const(super_idx)?;
                            // For now, we'll represent the superclass as a placeholder
                            // In a full implementation, we'd look up the actual class
                            Some(Box::new(Object::String(format!("superclass_placeholder_{}",
                                match super_val {
                                    Constant::String(s) => s,
                                    _ => "unknown".to_string(),
                                }))))
                        } else {
                            None
                        };

                        // Read method count and method names
                        // Read method count and method names - REMOVED
                        // Methods are now added via Op::Method
                        let methods = HashMap::new();

                        let class_obj = Object::Class(pulse_core::object::Class {
                            name,
                            methods, // Will be populated when class is instantiated
                            superclass,
                        });

                        let handle = self.heap.alloc(class_obj);
                        self.push(Value::Obj(handle));
                        return Ok(VMStatus::Running);
                    }

                    Op::GetSuper => {
                        let name_idx = self.read_u16() as usize;
                        let constant = self.get_current_chunk_const(name_idx)?;
                        let name = match constant {
                            Constant::String(s) => s.clone(),
                            _ => return Err(PulseError::RuntimeError("Method name must be string".into())),
                        };

                        let superclass_val = self.pop()?;
                        let receiver = self.pop()?;

                        match superclass_val {
                            Value::Obj(handle) => {
                                if let Some(Object::Class(class)) = self.heap.get(handle) {
                                     // Look up method in superclass
                                     if let Some(method_val) = class.methods.get(&name) {
                                         if let Value::Obj(h) = method_val {
                                             if let Some(Object::Closure(c)) = self.heap.get(*h) {
                                                 let bound = BoundMethod {
                                                     receiver,
                                                     method: c.function.clone(),
                                                 };
                                                 let bound_handle = self.heap.alloc(Object::BoundMethod(bound));
                                                 self.push(Value::Obj(bound_handle));
                                                 return Ok(VMStatus::Running);
                                             }
                                         }
                                         return Err(PulseError::RuntimeError(format!("Invalid method '{}'", name)));
                                     } else {
                                         return Err(PulseError::RuntimeError(format!("Undefined property '{}' in superclass", name)));
                                     }
                                } else {
                                    return Err(PulseError::TypeMismatch{expected: "superclass".into(), got: "other object".into()});
                                }
                            },
                            _ => return Err(PulseError::TypeMismatch{expected: "superclass".into(), got: superclass_val.type_name()}),
                        }
                    }

                    Op::Method => {
                        let name_idx = self.read_u16() as usize;
                        let constant = self.get_current_chunk_const(name_idx)?;
                        let name = match constant {
                            Constant::String(s) => s.clone(),
                            _ => return Err(PulseError::RuntimeError("Method name must be string".into())),
                        };

                        let closure_val = self.pop()?;
                        let class_val = self.peek(0); // Peek class

                        match class_val {
                            Value::Obj(handle) => {
                                // We need mutable access to the class
                                // But class is Arc inside Instance? No, Object::Class holds struct Class directly.
                                // We need to modify the heap object.
                                if let Some(Object::Class(ref mut class)) = self.heap.get_mut(*handle) {
                                    class.methods.insert(name, closure_val);
                                } else {
                                    return Err(PulseError::RuntimeError("Expected class on stack for method definition".into()));
                                }
                            },
                             _ => return Err(PulseError::TypeMismatch{expected: "class".into(), got: class_val.type_name()}),
                        }
                         return Ok(VMStatus::Running);
                    }

                    Op::CreateSharedMemory => {
                        // Pop the initial value for the shared memory
                        let initial_value = self.pop()?;

                        // Create a shared memory object
                        let shared_mem = pulse_core::object::SharedMemory {
                            value: initial_value,
                            locked: false,
                        };

                        // Use shared heap if available, otherwise fall back to local heap
                        // This enables zero-copy access across actors
                        if let Some(ref shared_heap) = self.shared_heap {
                            let handle = shared_heap.alloc(pulse_core::object::Object::SharedMemory(shared_mem));
                            // Push a special marker indicating this is a shared handle
                            self.push(Value::Obj(ObjHandle::from_shared_handle(handle.0)));
                        } else {
                            // Fallback to local heap (no zero-copy across actors)
                            let handle = self.heap.alloc(pulse_core::object::Object::SharedMemory(shared_mem));
                            self.push(Value::Obj(handle));
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::ReadSharedMemory => {
                        // Pop the shared memory reference
                        let shared_mem_val = self.pop()?;

                        // Try to read from shared heap first (zero-copy path)
                        if let Value::Obj(handle) = shared_mem_val {
                            if let Some(shared_handle_idx) = handle.to_shared_handle() {
                                if let Some(ref shared_heap) = self.shared_heap {
                                    if let Some(sm) = shared_heap.get(SharedHandle(shared_handle_idx)) {
                                        // Return the value directly (for primitives this copies, for objects it copies the handle)
                                        self.push(sm.value);
                                        return Ok(VMStatus::Running);
                                    }
                                }
                            }
                        }

                        // Fallback: read from local heap (slower path for backward compatibility)
                        let handle = match shared_mem_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "shared memory reference".into(), got: shared_mem_val.type_name()}),
                        };

                        // Read the value from shared memory
                        let obj = self.heap.get(handle).ok_or(PulseError::RuntimeError("Invalid shared memory handle".into()))?;
                        if let pulse_core::object::Object::SharedMemory(shared_mem) = obj {
                            self.push(shared_mem.value);  // Don't clone - Value is Copy for primitives
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "shared memory".into(), got: "other object".into()});
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::WriteSharedMemory => {
                        // Pop the value to write and the shared memory reference
                        let value_to_write = self.pop()?;
                        let shared_mem_val = self.pop()?;

                        // Try to write to shared heap first (zero-copy path)
                        if let Value::Obj(handle) = shared_mem_val {
                            if let Some(shared_handle_idx) = handle.to_shared_handle() {
                                if let Some(ref shared_heap) = self.shared_heap {
                                    if shared_heap.set(SharedHandle(shared_handle_idx), value_to_write) {
                                        // Push the written value as the result
                                        self.push(value_to_write);
                                        return Ok(VMStatus::Running);
                                    }
                                }
                            }
                        }

                        // Fallback: write to local heap
                        let handle = match shared_mem_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "shared memory reference".into(), got: shared_mem_val.type_name()}),
                        };

                        // Write the value to shared memory
                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid shared memory handle".into()))?;
                        if let pulse_core::object::Object::SharedMemory(ref mut shared_mem) = obj {
                            shared_mem.value = value_to_write;  // Value is Copy
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "shared memory".into(), got: "other object".into()});
                        }

                        // Push the written value as the result
                        self.push(value_to_write);
                        return Ok(VMStatus::Running);
                    }

                    Op::LockSharedMemory => {
                        // Pop the shared memory reference
                        let shared_mem_val = self.pop()?;

                        // Try to lock in shared heap first
                        if let Value::Obj(handle) = shared_mem_val {
                            if let Some(shared_handle_idx) = handle.to_shared_handle() {
                                if let Some(ref shared_heap) = self.shared_heap {
                                    let locked = shared_heap.try_lock(SharedHandle(shared_handle_idx));
                                    self.push(Value::Bool(locked));
                                    return Ok(VMStatus::Running);
                                }
                            }
                        }

                        // Fallback to local heap
                        let handle = match shared_mem_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "shared memory reference".into(), got: shared_mem_val.type_name()}),
                        };

                        // Attempt to lock the shared memory
                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid shared memory handle".into()))?;
                        if let pulse_core::object::Object::SharedMemory(ref mut shared_mem) = obj {
                            if shared_mem.locked {
                                self.push(Value::Bool(false));
                            } else {
                                shared_mem.locked = true;
                                self.push(Value::Bool(true));
                            }
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "shared memory".into(), got: "other object".into()});
                        }

                        return Ok(VMStatus::Running);
                    }

                    Op::UnlockSharedMemory => {
                        // Pop the shared memory reference
                        let shared_mem_val = self.pop()?;

                        // Try to unlock in shared heap first
                        if let Value::Obj(handle) = shared_mem_val {
                            if let Some(shared_handle_idx) = handle.to_shared_handle() {
                                if let Some(ref shared_heap) = self.shared_heap {
                                    let unlocked = shared_heap.unlock(SharedHandle(shared_handle_idx));
                                    self.push(Value::Bool(unlocked));
                                    return Ok(VMStatus::Running);
                                }
                            }
                        }

                        // Fallback to local heap
                        let handle = match shared_mem_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "shared memory reference".into(), got: shared_mem_val.type_name()}),
                        };

                        // Unlock the shared memory
                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid shared memory handle".into()))?;
                        if let pulse_core::object::Object::SharedMemory(ref mut shared_mem) = obj {
                            shared_mem.locked = false;
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "shared memory".into(), got: "other object".into()});
                        }

                        // Push success indicator
                        self.push(Value::Bool(true));
                        return Ok(VMStatus::Running);
                    }

                    // Atomic Operations
                    Op::AtomicCreate => {
                        // Pop the initial value for the atomic int
                        let initial_val = self.pop()?;
                        let initial = match initial_val {
                            Value::Int(i) => i,
                            _ => return Err(PulseError::TypeMismatch{expected: "int".into(), got: initial_val.type_name()}),
                        };

                        // Create atomic int
                        let atomic_int = pulse_core::object::AtomicInt::new(initial);
                        let handle = self.heap.alloc(pulse_core::object::Object::AtomicInt(atomic_int));
                        self.push(Value::Obj(handle));
                        return Ok(VMStatus::Running);
                    }

                    Op::AtomicLoad => {
                        // Pop the atomic int reference
                        let atomic_val = self.pop()?;
                        let handle = match atomic_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: atomic_val.type_name()}),
                        };

                        let obj = self.heap.get(handle).ok_or(PulseError::RuntimeError("Invalid atomic handle".into()))?;
                        if let pulse_core::object::Object::AtomicInt(atomic_int) = obj {
                            let value = atomic_int.load(std::sync::atomic::Ordering::SeqCst);
                            self.push(Value::Int(value));
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: "other object".into()});
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::AtomicStore => {
                        // Pop the value to store and the atomic int reference
                        let value_to_store = self.pop()?;
                        let atomic_val = self.pop()?;

                        let value = match value_to_store {
                            Value::Int(i) => i,
                            _ => return Err(PulseError::TypeMismatch{expected: "int".into(), got: value_to_store.type_name()}),
                        };

                        let handle = match atomic_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: atomic_val.type_name()}),
                        };

                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid atomic handle".into()))?;
                        if let pulse_core::object::Object::AtomicInt(ref atomic_int) = obj {
                            // Store returns (), so we need to load first to get old value
                            let old_value = atomic_int.load(std::sync::atomic::Ordering::SeqCst);
                            atomic_int.store(value, std::sync::atomic::Ordering::SeqCst);
                            self.push(Value::Int(old_value));
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: "other object".into()});
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::AtomicAdd => {
                        // Pop the value to add and the atomic int reference
                        let add_val = self.pop()?;
                        let atomic_val = self.pop()?;

                        let add = match add_val {
                            Value::Int(i) => i,
                            _ => return Err(PulseError::TypeMismatch{expected: "int".into(), got: add_val.type_name()}),
                        };

                        let handle = match atomic_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: atomic_val.type_name()}),
                        };

                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid atomic handle".into()))?;
                        if let pulse_core::object::Object::AtomicInt(ref atomic_int) = obj {
                            let old_value = atomic_int.fetch_add(add, std::sync::atomic::Ordering::SeqCst);
                            self.push(Value::Int(old_value));
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: "other object".into()});
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::AtomicSub => {
                        // Pop the value to subtract and the atomic int reference
                        let sub_val = self.pop()?;
                        let atomic_val = self.pop()?;

                        let sub = match sub_val {
                            Value::Int(i) => i,
                            _ => return Err(PulseError::TypeMismatch{expected: "int".into(), got: sub_val.type_name()}),
                        };

                        let handle = match atomic_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: atomic_val.type_name()}),
                        };

                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid atomic handle".into()))?;
                        if let pulse_core::object::Object::AtomicInt(ref atomic_int) = obj {
                            let old_value = atomic_int.fetch_sub(sub, std::sync::atomic::Ordering::SeqCst);
                            self.push(Value::Int(old_value));
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: "other object".into()});
                        }
                        return Ok(VMStatus::Running);
                    }

                    Op::AtomicCompareAndSwap => {
                        // Pop new value, expected value, and atomic int reference
                        let new_val = self.pop()?;
                        let expected_val = self.pop()?;
                        let atomic_val = self.pop()?;

                        let new = match new_val {
                            Value::Int(i) => i,
                            _ => return Err(PulseError::TypeMismatch{expected: "int".into(), got: new_val.type_name()}),
                        };

                        let expected = match expected_val {
                            Value::Int(i) => i,
                            _ => return Err(PulseError::TypeMismatch{expected: "int".into(), got: expected_val.type_name()}),
                        };

                        let handle = match atomic_val {
                            Value::Obj(h) => h,
                            _ => return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: atomic_val.type_name()}),
                        };

                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid atomic handle".into()))?;
                        if let pulse_core::object::Object::AtomicInt(ref atomic_int) = obj {
                            let result = atomic_int.compare_exchange(expected, new, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst);
                            match result {
                                Ok(_) => self.push(Value::Bool(true)),
                                Err(_) => self.push(Value::Bool(false)),
                            }
                        } else {
                            return Err(PulseError::TypeMismatch{expected: "atomic int".into(), got: "other object".into()});
                        }
                        return Ok(VMStatus::Running);
                    }

                    // Memory Fences
                    Op::MemoryFenceAcquire => {
                        // Acquire fence - ensures all reads after this see writes before matching release
                        std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
                        return Ok(VMStatus::Running);
                    }

                    Op::MemoryFenceRelease => {
                        // Release fence - ensures all writes before this are visible to threads doing acquire
                        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
                        return Ok(VMStatus::Running);
                    }

                    Op::MemoryFenceSeqCst => {
                        // Full memory barrier (Sequentially consistent)
                        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
                        return Ok(VMStatus::Running);
                    }
                }
            }
        }

        op_match!(op)
    }

    fn read_byte(&mut self) -> u8 {
        // Assume caller ensures valid frame
        let frame = self.frames.last_mut().unwrap_or_else(|| unreachable!());
        let ip = frame.ip;
        let closure_handle = frame.closure;
        frame.ip += 1;

        let obj = self.heap.get(closure_handle).unwrap_or_else(|| unreachable!());
        let chunk = match obj {
            Object::Closure(c) => &c.function.chunk,
            _ => unreachable!(),
        };
        chunk.code[ip]
    }

    fn read_u16(&mut self) -> u16 {
        let b1 = self.read_byte();
        let b2 = self.read_byte();
        u16::from_le_bytes([b1, b2])
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => *b,
            Value::Unit => false,
            _ => true,
        }
    }

    fn capture_upvalue(&mut self, local_idx: usize) -> ObjHandle {
        // 1. Search for existing open upvalue
        for handle in &self.open_upvalues {
            if let Some(Object::Upvalue(uv)) = self.heap.get(*handle) {
                if uv.location == Some(local_idx) {
                    return *handle;
                }
            }
        }

        // 2. Create new open upvalue
        let handle = self
            .heap
            .alloc(Object::Upvalue(pulse_core::object::Upvalue {
                location: Some(local_idx),
                closed: None,
            }));

        // 3. Keep it in open_upvalues list
        self.open_upvalues.push(handle);
        handle
    }

    fn close_upvalues(&mut self, last_idx: usize) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let handle = self.open_upvalues[i];
            let should_close = if let Some(Object::Upvalue(uv)) = self.heap.get(handle) {
                uv.location.is_some_and(|loc| loc >= last_idx)
            } else {
                false
            };

            if should_close {
                self.open_upvalues.remove(i);
                // Move value from stack to upvalue object
                if let Some(Object::Upvalue(uv)) = self.heap.get_mut(handle) {
                    let loc = uv.location.expect("Upvalue missing location");
                    let value = self.stack[loc];
                    uv.closed = Some(value);
                    uv.location = None;
                }
            } else {
                i += 1;
            }
        }
    }

    fn unwind_to_handler(&mut self, exception_val: Value) -> PulseResult<()> {
        // Find the nearest exception handler
        if let Some(exception_frame) = self.exception_frames.pop() {
            // Unwind call frames
            while self.frames.len() > exception_frame.frame_depth {
                self.frames.pop();
            }

            // Unwind stack
            while self.stack.len() > exception_frame.stack_depth {
                self.stack.pop();
            }

            // Push exception value onto stack for catch block
            self.push(exception_val);

            // Jump to handler
            if let Some(frame) = self.frames.last_mut() {
                frame.ip = exception_frame.handler_ip;
            }

            Ok(())
        } else {
            // No handler, propagate error
            Err(PulseError::RuntimeError("Uncaught exception".to_string()))
        }
    }

    pub fn push(&mut self, value: Value) {
        // Stack overflow protection - should be handled by caller
        if self.stack.len() >= MAX_STACK_DEPTH {
            // Ideally we'd return a result here, but since this is called frequently,
            // we will let the execution loop catch stack overflow by bounding locals properly.
            // For now, fail gracefully or push past it if absolutely necessary,
            // though returning Err in run() is safer.
        }
        self.stack.push(value);
    }

    /// Check if GC should be triggered and run it if needed
    pub fn maybe_collect_garbage(&mut self) {
        let (bytes_allocated, next_gc) = self.heap.get_allocation_stats();
        if bytes_allocated > next_gc {
            self.collect_garbage();
            // Update GC threshold after collection
            let (new_bytes, _) = self.heap.get_allocation_stats();
            let new_threshold = std::cmp::max(GC_HEAP_THRESHOLD, new_bytes * 2);
            self.heap.set_next_gc(new_threshold);
        }
    }

    pub fn pop(&mut self) -> PulseResult<Value> {
        self.stack.pop().ok_or(PulseError::StackUnderflow)
    }

    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - 1 - distance]
    }
    fn get_string(&self, handle: ObjHandle) -> PulseResult<String> {
        let obj = self
            .heap
            .get(handle)
            .ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
        match obj {
            Object::String(s) => Ok(s.clone()),
            _ => Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: "object".into(),
            }),
        }
    }

    pub fn print_value(&self, val: &Value) {
        match val {
            Value::Obj(handle) => {
                if let Some(obj) = self.heap.get(*handle) {
                    match obj {
                        Object::String(s) => print!("{}", s),
                        Object::NativeFn(n) => print!("<native fn {}>", n.name),
                        Object::List(vec) => {
                            print!("[");
                            for (i, val) in vec.iter().enumerate() {
                                if i > 0 {
                                    print!(", ");
                                }
                                self.print_value(val);
                            }
                            print!("]");
                        }
                        Object::Map(map) => {
                            print!("{{");
                            for (i, (k, v)) in map.iter().enumerate() {
                                if i > 0 {
                                    print!(", ");
                                }
                                print!("{}: ", k);
                                self.print_value(v);
                            }
                            print!("}}");
                        }

                        Object::Function(f) => print!("<fn {}>", f.name),
                        Object::SharedBuffer(_) => print!("<shared buffer>"),
                        Object::Closure(c) => print!("<fn {}>", c.function.name),
                        Object::Upvalue(_) => print!("<upvalue>"),
                        Object::Module(m) => print!("<module len={}>", m.len()),
                        Object::Class(c) => print!("<class {}>", c.name),
                        Object::Instance(i) => print!("<instance {}>", i.class.name),
                        Object::BoundMethod(_) => print!("<bound method>"),
                        Object::Set(s) => print!("<set len={}>", s.len()),
                        Object::Queue(q) => print!("<queue len={}>", q.len()),
                        Object::SharedMemory(sm) => print!("<shared memory locked={}>", sm.locked),
                        Object::AtomicInt(ai) => {
                            print!("<atomic {}>", ai.load(std::sync::atomic::Ordering::SeqCst))
                        }
                        Object::Socket(_) => print!("<socket>"),
                        Object::Listener(_) => print!("<listener>"),
                        Object::Regex(_) => print!("<regex>"),
                        Object::WebSocket(_) => print!("<websocket>"),
                    }
                } else {
                    print!("<freed object>");
                }
            }
            Value::Int(i) => print!("{}", i),
            Value::Float(f) => print!("{}", f),
            Value::Bool(b) => print!("{}", b),
            Value::Unit => print!("unit"),
            Value::Pid(pid) => print!("<actor {:?}>", pid),
        }
    }

    pub fn collect_garbage(&mut self) {
        // 1. Mark Roots
        self.mark_roots();

        // 2. Trace (Tri-color marking)
        self.heap.trace();

        // 3. Sweep
        self.heap.sweep();
    }

    fn mark_roots(&mut self) {
        // Stack
        for val in &self.stack {
            if let Value::Obj(handle) = val {
                self.heap.mark_object(*handle);
            }
        }

        // Globals
        for val in self.globals.values() {
            if let Value::Obj(handle) = val {
                self.heap.mark_object(*handle);
            }
        }

        // Builtins
        for val in self.builtins.values() {
            if let Value::Obj(handle) = val {
                self.heap.mark_object(*handle);
            }
        }

        // Call Frames (closures being executed)
        for frame in &self.frames {
            self.heap.mark_object(frame.closure);
        }

        // Open Upvalues
        for handle in &self.open_upvalues {
            self.heap.mark_object(*handle);
        }
    }

    fn resolve_path(&self, path: &str) -> PulseResult<String> {
        // Simple resolution: if relative, use current module path as base
        if path.starts_with("./") || path.starts_with("../") {
            let frame = self.frames.last().ok_or(PulseError::InternalError("Expected a value".into()))?;
            let closure = self.heap.get(frame.closure).ok_or(PulseError::InternalError("Expected a value".into()))?;
            if let Object::Closure(c) = closure {
                if let Some(base) = &c.function.module_path {
                    let base_path = std::path::Path::new(base);
                    if let Some(parent) = base_path.parent() {
                        let resolved = parent.join(path);
                        return Ok(resolved.to_string_lossy().to_string());
                    }
                }
            }
        }
        // Absolute or current dir
        Ok(path.to_string())
    }

    /// Enable debugging mode
    pub fn enable_debug(&mut self) {
        if self.debug_ctx.is_none() {
            self.debug_ctx = Some(crate::debug::DebugContext::new());
        }
    }

    /// Set breakpoint at source line
    pub fn set_breakpoint(&mut self, line: usize) {
        self.enable_debug();
        if let Some(ref mut ctx) = self.debug_ctx {
            ctx.set_breakpoint_line(line);
        }
    }

    /// Remove breakpoint at source line
    pub fn remove_breakpoint(&mut self, line: usize) {
        if let Some(ref mut ctx) = self.debug_ctx {
            ctx.remove_breakpoint_line(line);
        }
    }

    /// Continue execution from paused state
    pub fn debug_continue(&mut self) {
        if let Some(ref mut ctx) = self.debug_ctx {
            ctx.resume();
        }
    }

    /// Step one instruction
    pub fn debug_step(&mut self) {
        if let Some(ref mut ctx) = self.debug_ctx {
            ctx.step_in();
        }
    }

    /// Get stack contents for inspection
    pub fn get_stack(&self) -> Vec<String> {
        self.stack.iter().map(|v| self.format_value(v)).collect()
    }

    /// Get current frame info
    pub fn get_frame_info(&self) -> Option<(usize, usize)> {
        self.frames.last().map(|f| (f.ip, f.stack_start))
    }

    /// Format a value for display
    fn format_value(&self, val: &Value) -> String {
        match val {
            Value::Int(i) => format!("Int({})", i),
            Value::Float(f) => format!("Float({})", f),
            Value::Bool(b) => format!("Bool({})", b),
            Value::Unit => "Unit".to_string(),
            Value::Pid(p) => format!("Pid({:?})", p),
            Value::Obj(h) => {
                if let Some(obj) = self.heap.get(*h) {
                    match obj {
                        Object::String(s) => format!("String({:?})", s),
                        Object::List(_) => "<list>".to_string(),
                        Object::Map(_) => "<map>".to_string(),
                        Object::Closure(_) => "<closure>".to_string(),
                        Object::Function(_) => "<function>".to_string(),
                        Object::NativeFn(n) => format!("<native {}>", n.name),
                        Object::Upvalue(_) => "<upvalue>".to_string(),
                        Object::Module(_) => "<module>".to_string(),
                        Object::Class(c) => format!("<class {}>", c.name),
                        Object::Instance(i) => format!("<instance {}>", i.class.name),
                        Object::BoundMethod(_) => "<bound method>".to_string(),
                        Object::Set(s) => format!("<set len={}>", s.len()),
                        Object::Queue(q) => format!("<queue len={}>", q.len()),

                        Object::SharedMemory(_sm) => "SharedMemory".to_string(),
                        Object::AtomicInt(_) => "AtomicInt".to_string(),
                        Object::Socket(_) => "Socket".to_string(),
                        Object::Listener(_) => "Listener".to_string(),
                        Object::SharedBuffer(_) => "SharedBuffer".to_string(),
                        Object::Regex(_) => "Regex".to_string(),
                        Object::WebSocket(_) => "WebSocket".to_string(),
                    }
                } else {
                    "<invalid handle>".to_string()
                }
            }
        }
    }

    /// List all breakpoints
    pub fn list_breakpoints(&self) -> Vec<String> {
        self.debug_ctx
            .as_ref()
            .map(|c| c.list_breakpoints())
            .unwrap_or_default()
    }
}

impl HeapInterface for VM {
    fn alloc_object(&mut self, obj: Object) -> ObjHandle {
        // Trigger GC if needed
        // Simple heuristic: if heap size > threshold
        let handle = self.heap.alloc(obj);

        let (bytes_allocated, next_gc) = self.heap.get_allocation_stats();
        if bytes_allocated > next_gc {
            self.collect_garbage();
        }

        handle
    }

    fn get_object(&self, handle: ObjHandle) -> Option<&Object> {
        self.heap.get(handle)
    }

    fn get_mut_object(&mut self, handle: ObjHandle) -> Option<&mut Object> {
        self.heap.get_mut(handle)
    }

    fn collect_garbage(&mut self) {
        // 1. Mark Roots

        // Stack
        // We can't iterate self.stack while mutating self.heap easily if we are not careful.
        // But fields are disjoint.

        // Mark stack
        for val in &self.stack {
            if let Value::Obj(h) = val {
                self.heap.mark_object(*h);
            }
        }

        // Mark globals
        for val in self.globals.values() {
            if let Value::Obj(h) = val {
                self.heap.mark_object(*h);
            }
        }

        // Mark Frames (Closures)
        for frame in &self.frames {
            self.heap.mark_object(frame.closure);
            if let Some(_path) = &frame.module_path {
                // Strings are objects? No, module_path is Option<String>.
                // If it was ObjHandle, we'd mark it. String is owned here.
            }
            if let Some(globals) = &frame.prev_globals {
                for val in globals.values() {
                    if let Value::Obj(h) = val {
                        self.heap.mark_object(*h);
                    }
                }
            }
        }

        // Mark Open Upvalues
        for handle in &self.open_upvalues {
            self.heap.mark_object(*handle);
        }

        // Mark Loaded Modules
        for handle in self.loaded_modules.values() {
            self.heap.mark_object(*handle);
        }

        // Mark Global Cache
        for val in self.global_cache.values() {
            if let Value::Obj(h) = val {
                self.heap.mark_object(*h);
            }
        }

        // Builtins?
        for val in self.builtins.values() {
            if let Value::Obj(h) = val {
                self.heap.mark_object(*h);
            }
        }

        // 2. Trace
        self.heap.trace();

        // 3. Sweep
        let _freed = self.heap.sweep();

        // 4. Update Stats
        let (allocated, _) = self.heap.get_allocation_stats();
        let next_gc = std::cmp::max(allocated * 2, 1024 * 1024);
        self.heap.set_next_gc(next_gc);
    }

    fn get_allocation_stats(&self) -> (usize, usize) {
        self.heap.get_allocation_stats()
    }

    fn set_next_gc(&mut self, size: usize) {
        self.heap.set_next_gc(size);
    }
}

// Helper to print value with just HeapInterface
