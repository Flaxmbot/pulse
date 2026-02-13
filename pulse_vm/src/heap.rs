use pulse_core::object::{Object, ObjHandle, HeapInterface};


#[derive(Debug)]
enum ObjectEntry {
    Free { next_free: Option<usize> },
    Allocated { object: Object, marked: bool },
}

pub struct Heap {
    objects: Vec<ObjectEntry>,
    free_head: Option<usize>,
    gray_stack: Vec<usize>, // For marking
    scratch_buffer: Vec<ObjHandle>, // Reusable buffer for tracing children
    // For Phase 0: Enhanced memory management
    bytes_allocated: usize,
    next_gc: usize,  // When to trigger next GC
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::with_capacity(1024),
            free_head: None,
            gray_stack: Vec::new(),
            scratch_buffer: Vec::new(),
            bytes_allocated: 0,
            next_gc: 1024 * 1024, // 1MB default
        }
    }

    pub fn alloc(&mut self, object: Object) -> ObjHandle {
        // Estimate object size for GC triggering
        let size_estimate = match &object {
            Object::String(s) => s.len(),
            Object::List(vec) => vec.len() * 8, // Rough estimate
            Object::Map(map) => map.len() * 16, // Rough estimate
            Object::Function(_) => 64, // Rough estimate
            Object::Closure(_) => 32, // Rough estimate
            Object::Upvalue(_) => 16, // Rough estimate
            Object::NativeFn(_) => 16, // Rough estimate
            Object::Module(exports) => exports.len() * 16, // Rough estimate
            Object::Class(_) => 32, // Rough estimate
            Object::Instance(i) => 32 + i.fields.len() * 16, // Rough estimate
            Object::BoundMethod(_) => 32, // Rough estimate
            Object::Set(set) => set.len() * 8, // Rough estimate
            Object::Queue(q) => q.len() * 8, // Rough estimate

            Object::SharedMemory(_) => 16, // Rough estimate
            Object::Socket(_) => 64, // Struct + OS resource overhead
            Object::SharedBuffer(_) => 16, // Wrapper
            Object::Listener(_) => 16,
        };
        
        if let Some(idx) = self.free_head {
            // Re-use free slot
            if let ObjectEntry::Free { next_free } = self.objects[idx] {
                self.free_head = next_free;
                self.objects[idx] = ObjectEntry::Allocated { object, marked: false };
                self.bytes_allocated += size_estimate;
                return ObjHandle(idx);
            } else {
                panic!("Corrupted free list");
            }
        } else {
            // Append new slot
            let idx = self.objects.len();
            self.objects.push(ObjectEntry::Allocated { object, marked: false });
            self.bytes_allocated += size_estimate;
            ObjHandle(idx)
        }
    }
}


#[async_trait::async_trait]
impl HeapInterface for Heap {
    fn alloc_object(&mut self, obj: Object) -> ObjHandle {
        self.alloc(obj)
    }

    fn get_object(&self, handle: ObjHandle) -> Option<&Object> {
        self.get(handle)
    }

    fn get_mut_object(&mut self, handle: ObjHandle) -> Option<&mut Object> {
        self.get_mut(handle)
    }

    fn collect_garbage(&mut self) {
        panic!("Heap cannot collect garbage without root tracing (VM context needed).");
    }
    
    fn get_allocation_stats(&self) -> (usize, usize) {
        (self.bytes_allocated, self.next_gc)
    }
    
    fn set_next_gc(&mut self, size: usize) {
        self.next_gc = size;
    }
}

impl Heap {
    pub fn get(&self, handle: ObjHandle) -> Option<&Object> {
        if handle.0 < self.objects.len() {
            match &self.objects[handle.0] {
                ObjectEntry::Allocated { object, .. } => Some(object),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, handle: ObjHandle) -> Option<&mut Object> {
        if handle.0 < self.objects.len() {
            match &mut self.objects[handle.0] {
                ObjectEntry::Allocated { object, .. } => Some(object),
                _ => None,
            }
        } else {
            None
        }
    }
    
    // --- GC ---
    
    pub fn mark_object(&mut self, handle: ObjHandle) {
        let idx = handle.0;
        if idx >= self.objects.len() { return; }
        
        if let ObjectEntry::Allocated { marked, .. } = &mut self.objects[idx] {
            if *marked { return; }
            *marked = true;
            self.gray_stack.push(idx);
        }
    }

    pub fn trace(&mut self) {
        while let Some(idx) = self.gray_stack.pop() {
            // Re-use scratch buffer to avoid allocation
            // We temporarily take the buffer out to safely borrow it mutably alongside immutable 'objects'
            let mut children = std::mem::take(&mut self.scratch_buffer);
            children.clear();
            
            if let Some(ObjectEntry::Allocated { object, .. }) = self.objects.get(idx) {
                object.visit_references(|h| children.push(h));
            }
            
            // Now mark the children
            for handle in &children {
                self.mark_object(*handle);
            }
            
            // Put buffer back
            self.scratch_buffer = children;
        }
    }
    
    // Sweep phase: returns bytes freed (simulation)
    pub fn sweep(&mut self) -> usize {
        let mut freed = 0;
        for i in 0..self.objects.len() {
             let is_garbage = match &self.objects[i] {
                ObjectEntry::Allocated { marked, .. } => !marked,
                _ => false,
             };

             if is_garbage {
                 // Free it
                 if let ObjectEntry::Allocated { object, .. } = &self.objects[i] {
                     // Subtract estimated size
                     let size_estimate = match object {
                         Object::String(s) => s.len(),
                         Object::List(vec) => vec.len() * 8,
                         Object::Map(map) => map.len() * 16,
                         Object::Function(_) => 64,
                         Object::Closure(_) => 32,
                         Object::Upvalue(_) => 16,
                         Object::NativeFn(_) => 16,
                         Object::Module(exports) => exports.len() * 16,
                         Object::Class(_) => 32,
                         Object::Set(set) => set.len() * 8,
                         Object::Queue(q) => q.len() * 8,

                         Object::SharedMemory(_) => 16,
                         Object::Socket(_) => 64, // This is the existing one
                         Object::SharedBuffer(_) => 16,
                         Object::Instance(i) => 32 + i.fields.len() * 16,
                         Object::BoundMethod(_) => 32,
                         Object::Listener(_) => 16,
                     };
                     self.bytes_allocated = self.bytes_allocated.saturating_sub(size_estimate);
                 }
                 
                 self.objects[i] = ObjectEntry::Free { next_free: self.free_head };
                 self.free_head = Some(i);
                 freed += 1;
             } else {
                 // Unmark for next cycle
                 if let ObjectEntry::Allocated { marked, .. } = &mut self.objects[i] {
                     *marked = false;
                 }
             }
        }
        freed
    }
    
    pub fn get_allocation_stats(&self) -> (usize, usize) {
        (self.bytes_allocated, self.next_gc)
    }
    
    pub fn set_next_gc(&mut self, size: usize) {
        self.next_gc = size;
    }
}
