use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_memory_fence_visibility() {
    // This test demonstrates proper visibility with memory fences
    // Using atomic operations with proper ordering to ensure visibility
    
    let shared_flag = Arc::new(AtomicBool::new(false));
    let shared_data = Arc::new(AtomicI64::new(0));
    let ready = Arc::new(AtomicBool::new(false));
    
    let flag_clone1 = shared_flag.clone();
    let data_clone1 = shared_data.clone();
    let ready_clone1 = ready.clone();
    
    // Writer thread
    let writer = thread::spawn(move || {
        // Write data first
        data_clone1.store(42, Ordering::SeqCst);
        // Then set flag with release semantics
        flag_clone1.store(true, Ordering::SeqCst);
        // Signal that we're ready
        ready_clone1.store(true, Ordering::SeqCst);
    });
    
    let flag_clone2 = shared_flag.clone();
    let data_clone2 = shared_data.clone();
    let ready_clone2 = ready.clone();
    
    // Reader thread
    let reader = thread::spawn(move || {
        // Wait for ready signal
        while !ready_clone2.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
        }
        
        // Read flag with acquire semantics  
        let flag = flag_clone2.load(Ordering::SeqCst);
        
        // If flag is true, we should see the data as 42 (not 0)
        // The SeqCst fence ensures this visibility
        if flag {
            let data = data_clone2.load(Ordering::SeqCst);
            assert_eq!(data, 42, "Data should be visible after flag is set");
        }
    });
    
    writer.join().unwrap();
    reader.join().unwrap();
}

#[test]
fn test_acquire_release_fence() {
    // Test acquire/release fence semantics explicitly
    
    let data = Arc::new(AtomicI64::new(0));
    let ready = Arc::new(AtomicBool::new(false));
    
    let data_clone1 = data.clone();
    let ready_clone1 = ready.clone();
    
    // Writer - uses release fence
    let writer = thread::spawn(move || {
        data_clone1.store(100, Ordering::Release);
        ready_clone1.store(true, Ordering::Release);
    });
    
    let data_clone2 = data.clone();
    let ready_clone2 = ready.clone();
    
    // Reader - uses acquire fence  
    let reader = thread::spawn(move || {
        // Spin until ready
        while !ready_clone2.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(1));
        }
        
        // Now data should be visible due to acquire fence
        let value = data_clone2.load(Ordering::Acquire);
        assert_eq!(value, 100);
    });
    
    writer.join().unwrap();
    reader.join().unwrap();
}

#[test]
fn test_fence_with_shared_memory() {
    // Test fence operations with the VM's shared heap
    
    use pulse_vm::shared_heap::{create_shared_heap, SharedHandle};
    use pulse_core::object::{Object, SharedMemory};
    use pulse_core::Value;
    
    let heap = create_shared_heap();
    
    // Allocate shared memory
    let mem = SharedMemory {
        value: Value::Int(0),
        locked: false,
    };
    let handle = heap.alloc(Object::SharedMemory(mem));
    
    let heap_clone = heap.clone();
    let handle_val = handle.0;
    
    // Writer thread
    let writer = thread::spawn(move || {
        // Write value with release fence
        heap_clone.set(SharedHandle(handle_val), Value::Int(123));
        heap_clone.release_fence();
    });
    
    let heap_clone2 = heap.clone();
    let handle_val2 = handle.0;
    
    // Reader thread
    let reader = thread::spawn(move || {
        heap_clone2.acquire_fence();
        if let Some(sm) = heap_clone2.get(SharedHandle(handle_val2)) {
            assert_eq!(sm.value, Value::Int(123));
        } else {
            panic!("Failed to get shared memory");
        }
    });
    
    writer.join().unwrap();
    reader.join().unwrap();
}
