use crate::mailbox::{Message, SystemMessage};
use crate::runtime::RuntimeHandle;
use pulse_core::object::Object;
use pulse_core::{ActorId, Constant, ObjHandle, PulseError, Value};
use pulse_vm::{VMStatus, VM};
use std::collections::HashSet;
use tokio::fs;
use tokio::sync::mpsc;

#[derive(Debug, PartialEq)]
pub enum ActorStatus {
    Starting,
    Running,
    Waiting,
    Terminated,
}

/// Resource limits for actors
#[derive(Debug, Clone, Copy)]
pub struct ActorLimits {
    /// Maximum number of messages in mailbox (default: 10000)
    pub max_mailbox_size: usize,
    /// Maximum size of a single message in bytes (default: 1048576 = 1MB)
    pub max_message_size_bytes: usize,
}

impl Default for ActorLimits {
    fn default() -> Self {
        Self {
            max_mailbox_size: 10000,
            max_message_size_bytes: 1048576, // 1MB
        }
    }
}

impl ActorLimits {
    /// Create new limits with custom values
    pub fn new(max_mailbox_size: usize, max_message_size_bytes: usize) -> Self {
        Self {
            max_mailbox_size,
            max_message_size_bytes,
        }
    }

    /// Check if message size is within limits
    pub fn check_message_size(&self, size_bytes: usize) -> Result<(), PulseError> {
        if size_bytes > self.max_message_size_bytes {
            Err(PulseError::MessageTooLarge {
                size: size_bytes,
                max: self.max_message_size_bytes,
            })
        } else {
            Ok(())
        }
    }
}

pub struct Actor {
    pub id: ActorId,
    pub vm: VM,
    pub links: HashSet<ActorId>,
    pub monitors: HashSet<ActorId>,
    pub trapping_exits: bool,
    pub runtime: RuntimeHandle,
    /// Resource limits for this actor
    pub limits: ActorLimits,
    /// Current mailbox size (for tracking against limits)
    pub mailbox_size: std::sync::atomic::AtomicUsize,
}

impl Actor {
    pub fn new(id: ActorId, vm: VM, runtime: RuntimeHandle) -> Self {
        Self {
            id,
            vm,
            links: HashSet::new(),
            monitors: HashSet::new(),
            trapping_exits: false,
            runtime, // Needed to spawn/send from within VM
            limits: ActorLimits::default(),
            mailbox_size: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Set custom actor limits
    pub fn with_limits(mut self, limits: ActorLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Check if mailbox has space for a new message
    pub fn can_accept_message(&self) -> bool {
        let current = self.mailbox_size.load(std::sync::atomic::Ordering::Relaxed);
        current < self.limits.max_mailbox_size
    }

    /// Increment mailbox size counter
    pub fn increment_mailbox_size(&self) {
        self.mailbox_size
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Decrement mailbox size counter
    pub fn decrement_mailbox_size(&self) {
        self.mailbox_size
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn run(
        mut self,
        mut mailbox: mpsc::UnboundedReceiver<Message>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async move {
            loop {
                // Priority: Check mailbox first if we are in a "receivable" state?
                // Actually, we should run the VM. The VM returns Yield or Blocked.

                // Run for a time slice (steps)
                let status = self.vm.run(1000).await;

                match status {
                    VMStatus::Running => {
                        // Should not happen with run(steps) unless steps > 0, but loop handles it.
                        tokio::task::yield_now().await;
                    }
                    VMStatus::Yielded => {
                        // Yielded generally means "give other tasks a chance" or "I did some I/O"
                        // Check mailbox for non-blocking messages?
                        // For now, just yield to tokio scheduler.
                        // But we should process system messages (kills, etc.)
                        while let Ok(msg) = mailbox.try_recv() {
                            self.handle_message(msg);
                        }
                        tokio::task::yield_now().await;
                    }
                    VMStatus::Blocked => {
                        // VM is waiting for a message (Receive instruction)
                        // We must await the mailbox.
                        if let Some(msg) = mailbox.recv().await {
                            self.handle_message(msg);
                        } else {
                            // Mailbox closed, runtime shutting down?
                            break;
                        }
                    }
                    VMStatus::Send { target, msg } => {
                        // Async send
                        let _ = self.runtime.send(target, Message::User(msg)).await;
                    }
                    VMStatus::Spawn(closure, captured_upvalues, globals) => {
                        let child_pid = self
                            .runtime
                            .spawn_from_actor(closure, captured_upvalues, globals, Vec::new())
                            .await;
                        self.vm.push(Value::Pid(child_pid));
                    }
                    VMStatus::SpawnCall(args, captured_upvalues, globals) => {
                        // SpawnCall args: [closure/function as arg 0, ... actual arguments]
                        // We need to extract the callable from args[0]
                        if args.is_empty() {
                            tracing::error!("SpawnCall called with no arguments");
                            break;
                        }
                        let closure = args[0].clone();
                        let actual_args = args[1..].to_vec();

                        let child_pid = self
                            .runtime
                            .spawn_from_actor(closure, captured_upvalues, globals, actual_args)
                            .await;
                        self.vm.push(Value::Pid(child_pid));
                    }
                    VMStatus::SpawnLink(closure, captured_upvalues, globals) => {
                        let child_pid = self
                            .runtime
                            .spawn_from_actor(closure, captured_upvalues, globals, Vec::new())
                            .await;

                        // Link
                        self.links.insert(child_pid);
                        // We need to tell child to link back.
                        let _ = self
                            .runtime
                            .send(child_pid, Message::System(SystemMessage::Link(self.id)))
                            .await;

                        self.vm.push(Value::Pid(child_pid));
                    }
                    VMStatus::Link(target) => {
                        self.links.insert(target);
                        // Notify target
                        let _ = self
                            .runtime
                            .send(target, Message::System(SystemMessage::Link(self.id)))
                            .await;
                    }
                    VMStatus::Monitor(target) => {
                        let _ = self.runtime.monitor(self.id, target).await;
                    }
                    VMStatus::Register(name, target) => {
                        let res = self.runtime.register(name, target).await;
                        self.vm.push(Value::Bool(res));
                    }
                    VMStatus::Unregister(name) => {
                        self.runtime.unregister(&name).await;
                        self.vm.push(Value::Unit);
                    }
                    VMStatus::WhereIs(name) => {
                        let pid = self.runtime.whereis(&name).await;
                        match pid {
                            Some(p) => self.vm.push(Value::Pid(p)),
                            None => self.vm.push(Value::Unit),
                        }
                    }
                    VMStatus::Import(path) => {
                        let module_path = path.clone();

                        // Check if it's a standard library module
                        if module_path.starts_with("std/") {
                            if let Some(handle) =
                                crate::stdlib::load_std_module(&module_path, &mut self.vm)
                            {
                                self.vm.push(Value::Obj(handle));
                                self.vm.loaded_modules.insert(module_path, handle);
                            } else {
                                // Module not found
                                self.vm.push(Value::Unit);
                            }
                            continue;
                        }

                        // Async import support for user files
                        // 1. Read the file
                        // 2. Compile it
                        // 3. Run it as a module
                        // 4. Get the module handle and push to original VM's stack

                        // Read file asynchronously
                        let source = match fs::read_to_string(&module_path).await {
                            Ok(s) => s,
                            Err(_e) => {
                                // File read error - push nil and continue
                                // File read error - push nil -> actually better to return Error status?
                                // For import, if file not found, we should probably error out the actor or at least return nil and let user handle.
                                // Currently import instruction doesn't return result, it returns module handle.
                                // Let's push Unit (nil) but NOT print to stderr.
                                self.vm.push(Value::Unit);
                                continue;
                            }
                        };

                        // Compile the module
                        let chunk =
                            match pulse_compiler::compile(&source, Some(module_path.clone())) {
                                Ok(c) => c,
                                Err(_e) => {
                                    // eprintln!("Failed to compile module '{}': {:?}", module_path, e);
                                    self.vm.push(Value::Unit);
                                    continue;
                                }
                            };

                        // Get the shared heap from the runtime
                        let shared_heap = self.runtime.shared_heap();

                        // Create a new VM to run the module
                        let mut module_vm = VM::new(chunk, self.id, Some(shared_heap));

                        // Mark as module so return captures exports
                        if let Some(Object::Closure(c)) = module_vm.heap.get_mut(ObjHandle(0)) {
                            c.function.module_path = Some(module_path.clone());
                        }
                        // Set is_module on the frame
                        if let Some(frame) = module_vm.frames.first_mut() {
                            frame.is_module = true;
                            frame.module_path = Some(module_path.clone());
                        }

                        // Run the module to completion (or until it yields/halt)
                        loop {
                            let status = module_vm.run(10000).await;

                            match status {
                                VMStatus::Running | VMStatus::Yielded => {
                                    // Continue running
                                    tokio::task::yield_now().await;
                                }
                                VMStatus::Halted => {
                                    // Module finished executing - good
                                    break;
                                }
                                VMStatus::Error(_e) => {
                                    // Runtime error in module
                                    break;
                                }
                                VMStatus::Import(sub_path) => {
                                    // Handle nested import recursively
                                    let sub_source = match fs::read_to_string(&sub_path).await {
                                        Ok(s) => s,
                                        Err(_e) => {
                                            // Nested read error
                                            break;
                                        }
                                    };

                                    let sub_chunk = match pulse_compiler::compile(
                                        &sub_source,
                                        Some(sub_path.clone()),
                                    ) {
                                        Ok(c) => c,
                                        Err(_e) => {
                                            // Nested compile error
                                            break;
                                        }
                                    };

                                    // Replace module VM with new one
                                    let nested_heap = self.runtime.shared_heap();
                                    module_vm = VM::new(sub_chunk, self.id, Some(nested_heap));
                                }
                                _ => {
                                    // Other statuses - just continue
                                    tokio::task::yield_now().await;
                                }
                            }
                        }

                        // Get the loaded module handle
                        if let Some(handle) = module_vm.loaded_modules.get(&module_path) {
                            // Push the module to the original VM's stack
                            self.vm.push(Value::Obj(*handle));

                            // Also register it in the original VM's loaded_modules
                            self.vm.loaded_modules.insert(module_path, *handle);
                        } else {
                            // Module didn't register itself (possibly no return statement)
                            // Push unit as fallback
                            self.vm.push(Value::Unit);
                        }
                    }
                    VMStatus::Halted => {
                        // Normal exit
                        self.runtime.exit(self.id, "normal".to_string()).await;
                        break;
                    }
                    VMStatus::Error(e) => {
                        self.runtime.exit(self.id, format!("{}", e)).await;
                        break;
                    }
                    _ => {}
                }
            }
        })
    }

    fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::User(constant) => {
                // Convert Constant to Value and push to VM
                let val = self.constant_to_value(constant);
                self.vm.push(val);
            }
            Message::System(sys) => {
                match sys {
                    SystemMessage::Link(pid) => {
                        self.links.insert(pid);
                    }
                    SystemMessage::Exit(_pid, _reason) => {
                        if self.trapping_exits {
                            // Convert to message
                            // push {'EXIT', pid, reason}
                        } else {
                            // Die
                            // We should probably trigger Error status or similar
                            // For now, simple termination
                            //  self.vm.status = ...?
                            // This usually happens in the loop.
                        }
                    }

                    SystemMessage::MonitorExit(_pid, _reason) => {
                        // Push {'DOWN', ...}
                    }
                    _ => {}
                }
            }
        }
    }

    fn constant_to_value(&mut self, c: Constant) -> Value {
        self.vm.constant_to_value(&c)
    }
}
