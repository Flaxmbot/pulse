use pulse_core::object::{Object, ObjHandle, HeapInterface};
use pulse_core::Value;


#[derive(Debug)]
enum ObjectEntry {
    Free { next_free: Option<usize> },
    Allocated { object: Object, marked: bool },
}

pub struct Heap {
    objects: Vec<ObjectEntry>,
    free_head: Option<usize>,
    gray_stack: Vec<usize>, // For marking
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::with_capacity(1024),
            free_head: None,
            gray_stack: Vec::new(),
        }
    }

    pub fn alloc(&mut self, object: Object) -> ObjHandle {
        if let Some(idx) = self.free_head {
            // Re-use free slot
            if let ObjectEntry::Free { next_free } = self.objects[idx] {
                self.free_head = next_free;
                self.objects[idx] = ObjectEntry::Allocated { object, marked: false };
                return ObjHandle(idx);
            } else {
                panic!("Corrupted free list");
            }
        } else {
            // Append new slot
            let idx = self.objects.len();
            self.objects.push(ObjectEntry::Allocated { object, marked: false });
            ObjHandle(idx)
        }
    }
}

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
            // We need to look at the object at `idx`
            // But we can't borrow `self.objects` while mutating `self` (calling mark_object).
            // Classic borrow checker issue.
            
            // Workaround: Extract handles to mark, THEN mark them.
            // Or use indices.
            
            let handles_to_mark = {
                if let ObjectEntry::Allocated { object, .. } = &self.objects[idx] {
                    match object {
                        Object::List(vec) => {
                            vec.iter().filter_map(|v| if let Value::Obj(h) = v { Some(*h) } else { None }).collect::<Vec<_>>()
                        },
                        Object::Map(map) => {
                            map.values().filter_map(|v| if let Value::Obj(h) = v { Some(*h) } else { None }).collect::<Vec<_>>()
                        },
                        Object::Function(_) | Object::Closure(_) | Object::String(_) | Object::NativeFn(_) => Vec::new(),
                    }
                } else {
                    Vec::new()
                }
            };
            
            for handle in handles_to_mark {
                self.mark_object(handle);
            }
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
}
