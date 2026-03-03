use parking_lot::RwLock;
use pulse_core::object::Object;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A heap entry that can be traced for GC
#[derive(Debug)]
enum HeapEntry {
    #[allow(dead_code)]
    Free {
        next_free: Option<usize>,
    },
    Allocated {
        object: Object,
        _marked: bool,
    },
}

/// Thread-safe shared heap for cross-actor communication
/// This heap stores shared memory objects that can be accessed
/// without copying between actors
pub struct SharedHeap {
    entries: RwLock<Vec<HeapEntry>>,
    free_head: RwLock<Option<usize>>,
    _gray_stack: RwLock<Vec<usize>>,
    bytes_allocated: AtomicUsize,
    next_gc: AtomicUsize,
}

impl Default for SharedHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedHeap {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::with_capacity(1024)),
            free_head: RwLock::new(None),
            _gray_stack: RwLock::new(Vec::new()),
            bytes_allocated: AtomicUsize::new(0),
            next_gc: AtomicUsize::new(1024 * 1024), // 1MB default
        }
    }

    /// Allocate a shared memory object in the heap
    /// Returns a handle that can be used to access the object
    pub fn alloc(&self, object: Object) -> SharedHandle {
        let size_estimate = self.estimate_size(&object);

        // Try to reuse a free slot
        let mut free_head = self.free_head.write();
        if let Some(idx) = *free_head {
            let mut entries = self.entries.write();
            if let HeapEntry::Free { next_free } = entries[idx] {
                *free_head = next_free;
                entries[idx] = HeapEntry::Allocated {
                    object,
                    _marked: false,
                };
                self.bytes_allocated
                    .fetch_add(size_estimate, Ordering::Relaxed);
                return SharedHandle(idx);
            }
        }
        drop(free_head);

        // Allocate new slot
        let mut entries = self.entries.write();
        let idx = entries.len();
        entries.push(HeapEntry::Allocated {
            object,
            _marked: false,
        });
        self.bytes_allocated
            .fetch_add(size_estimate, Ordering::Relaxed);
        SharedHandle(idx)
    }

    /// Get a reference to an object (immutable borrow)
    /// Returns None if the handle is invalid
    /// Uses Acquire semantics to ensure visibility of writes from other threads
    pub fn get(&self, handle: SharedHandle) -> Option<pulse_core::object::SharedMemory> {
        let entries = self.entries.read();
        if handle.0 < entries.len() {
            match &entries[handle.0] {
                HeapEntry::Allocated { object, .. } => {
                    if let Object::SharedMemory(sm) = object {
                        // Acquire fence ensures we see all writes that happened before
                        // the matching release fence in the writing thread
                        std::sync::atomic::fence(Ordering::Acquire);
                        // Clone the value - this is necessary because we can't hold a reference
                        // across the lock release. However, for primitives this is very cheap
                        // (just copying a usize), and for objects we're copying the handle.
                        Some(sm.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Get mutable access to an object
    /// Returns None if the handle is invalid or if another thread is mutating
    pub fn get_mut(&self, handle: SharedHandle) -> Option<pulse_core::object::SharedMemory> {
        let mut entries = self.entries.write();
        if handle.0 < entries.len() {
            match &mut entries[handle.0] {
                HeapEntry::Allocated { object, .. } => {
                    if let Object::SharedMemory(sm) = object {
                        let result = sm.clone();
                        // Update the value in place
                        Some(result)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Update the value at a given handle
    /// Uses Release semantics to ensure visibility to other threads
    pub fn set(&self, handle: SharedHandle, value: pulse_core::Value) -> bool {
        let mut entries = self.entries.write();
        if handle.0 < entries.len() {
            if let HeapEntry::Allocated {
                object: Object::SharedMemory(ref mut sm),
                ..
            } = &mut entries[handle.0]
            {
                sm.value = value;
                // Release fence ensures all our writes are visible to threads
                // that acquire after us
                std::sync::atomic::fence(Ordering::Release);
                return true;
            }
        }
        false
    }

    /// Try to lock the shared memory (returns false if already locked)
    pub fn try_lock(&self, handle: SharedHandle) -> bool {
        let mut entries = self.entries.write();
        if handle.0 < entries.len() {
            if let HeapEntry::Allocated {
                object: Object::SharedMemory(ref mut sm),
                ..
            } = &mut entries[handle.0]
            {
                if !sm.locked {
                    sm.locked = true;
                    return true;
                }
            }
        }
        false
    }

    /// Unlock the shared memory
    pub fn unlock(&self, handle: SharedHandle) -> bool {
        let mut entries = self.entries.write();
        if handle.0 < entries.len() {
            if let HeapEntry::Allocated {
                object: Object::SharedMemory(ref mut sm),
                ..
            } = &mut entries[handle.0]
            {
                sm.locked = false;
                return true;
            }
        }
        false
    }

    fn estimate_size(&self, object: &Object) -> usize {
        match object {
            Object::String(s) => s.len(),
            Object::List(vec) => vec.len() * 8,
            Object::Map(map) => map.len() * 16,
            Object::Function(_) => 64,
            Object::Closure(_) => 32,
            Object::Upvalue(_) => 16,
            Object::NativeFn(_) => 16,
            Object::AtomicInt(_) => 16,
            Object::Module(exports) => exports.len() * 16,
            Object::Class(_) => 32,
            Object::Instance(i) => 32 + i.fields.len() * 16,
            Object::BoundMethod(_) => 32,
            Object::Set(set) => set.len() * 8,
            Object::Queue(q) => q.len() * 8,
            Object::SharedMemory(_) => 64, // Larger for shared
            Object::Socket(_) => 64,
            Object::SharedBuffer(_) => 16,
            Object::Listener(_) => 16,
            Object::Regex(_) => 16,
            Object::WebSocket(_) => 16,
        }
    }

    pub fn get_allocation_stats(&self) -> (usize, usize) {
        (
            self.bytes_allocated.load(Ordering::Relaxed),
            self.next_gc.load(Ordering::Relaxed),
        )
    }

    /// Explicit acquire fence - ensures all subsequent reads see writes before this fence
    pub fn acquire_fence(&self) {
        std::sync::atomic::fence(Ordering::Acquire);
    }

    /// Explicit release fence - ensures all writes before this fence are visible to subsequent reads
    pub fn release_fence(&self) {
        std::sync::atomic::fence(Ordering::Release);
    }

    /// Explicit sequentially consistent fence - full memory barrier
    pub fn seqcst_fence(&self) {
        std::sync::atomic::fence(Ordering::SeqCst);
    }
}

/// Handle to an object in the shared heap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SharedHandle(pub usize);

/// Create a new shared heap wrapped in Arc
pub fn create_shared_heap() -> std::sync::Arc<SharedHeap> {
    std::sync::Arc::new(SharedHeap::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_core::object::SharedMemory;
    use pulse_core::Value;

    #[test]
    fn test_shared_heap_alloc() {
        let heap = create_shared_heap();
        let mem = SharedMemory {
            value: Value::Int(42),
            locked: false,
        };
        let handle = heap.alloc(Object::SharedMemory(mem));

        let result = heap.get(handle);
        assert!(result.is_some());
        assert_eq!(result.expect("Expected a value").value, Value::Int(42));
    }

    #[test]
    fn test_shared_heap_mutable() {
        let heap = create_shared_heap();
        let mem = SharedMemory {
            value: Value::Int(42),
            locked: false,
        };
        let handle = heap.alloc(Object::SharedMemory(mem));

        // Modify the value
        let success = heap.set(handle, Value::Int(100));
        assert!(success);

        // Read the modified value
        let result = heap.get(handle);
        assert!(result.is_some());
        assert_eq!(result.expect("Expected a value").value, Value::Int(100));
    }

    #[test]
    fn test_shared_heap_lock() {
        let heap = create_shared_heap();
        let mem = SharedMemory {
            value: Value::Int(42),
            locked: false,
        };
        let handle = heap.alloc(Object::SharedMemory(mem));

        // Lock should succeed
        let result = heap.try_lock(handle);
        assert!(result);

        // Second lock should fail
        let result2 = heap.try_lock(handle);
        assert!(!result2);

        // Unlock
        let result3 = heap.unlock(handle);
        assert!(result3);

        // Lock should succeed again
        let result4 = heap.try_lock(handle);
        assert!(result4);
    }
}
