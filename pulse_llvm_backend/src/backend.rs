use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::targets::{InitializationConfig, Target};
use inkwell::values::{FunctionValue, BasicValueEnum, PointerValue, BasicValue};
use std::collections::HashMap;

use pulse_core::{Chunk, Op, Constant};

pub struct LLVMBackend<'ctx> {
    #[allow(dead_code)]
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    #[allow(dead_code)]
    functions: HashMap<String, FunctionValue<'ctx>>,
    stack_ptr: Option<PointerValue<'ctx>>,
    stack_top: i32,
}

impl<'ctx> LLVMBackend<'ctx> {
    pub fn new(context: &'ctx Context) -> Result<Self, String> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("Failed to initialize native target: {}", e))?;

        let module = context.create_module("pulse_module");
        let builder = context.create_builder();

        let execution_engine = module
            .create_execution_engine()
            .map_err(|e| format!("Failed to create execution engine: {}", e))?;

        Ok(LLVMBackend {
            context,
            module,
            builder,
            execution_engine,
            functions: HashMap::new(),
            stack_ptr: None,
            stack_top: 0,
        })
    }

    fn init_vm_stack(&mut self) {
        let stack_type = self.context.i64_type().array_type(1024);
        let stack_ptr = self.builder.build_alloca(stack_type, "vm_stack").unwrap();
        self.stack_ptr = Some(stack_ptr);
    }

    fn push_value(&mut self, value: BasicValueEnum<'ctx>) {
        if let Some(stack_ptr) = self.stack_ptr {
            let idx = self.context.i32_type().const_int(self.stack_top as u64, false);
            let array_type = self.context.i64_type().array_type(1024);
            let stack_element_ptr = unsafe {
                self.builder.build_gep(
                    array_type,
                    stack_ptr,
                    &[idx],
                    "stack_element_ptr",
                ).unwrap()
            };
            self.builder.build_store(stack_element_ptr, value).unwrap();
            self.stack_top += 1;
        }
    }

    fn pop_value(&mut self) -> Option<BasicValueEnum<'ctx>> {
        if self.stack_top > 0 {
            self.stack_top -= 1;
            if let Some(stack_ptr) = self.stack_ptr {
                let idx = self.context.i32_type().const_int(self.stack_top as u64, false);
                let array_type = self.context.i64_type().array_type(1024);
                let stack_element_ptr = unsafe {
                    self.builder.build_gep(
                        array_type,
                        stack_ptr,
                        &[idx],
                        "stack_element_ptr",
                    ).unwrap()
                };
                let value = self.builder.build_load(
                    self.context.i64_type(),
                    stack_element_ptr,
                    "popped_value",
                ).unwrap();
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn peek_value(&mut self) -> Option<BasicValueEnum<'ctx>> {
        if self.stack_top > 0 {
            if let Some(stack_ptr) = self.stack_ptr {
                let idx = self.context.i32_type().const_int((self.stack_top - 1) as u64, false);
                let array_type = self.context.i64_type().array_type(1024);
                let stack_element_ptr = unsafe {
                    self.builder.build_gep(
                        array_type,
                        stack_ptr,
                        &[idx],
                        "stack_peek_ptr",
                    ).unwrap()
                };
                let value = self.builder.build_load(
                    self.context.i64_type(),
                    stack_element_ptr,
                    "peek_value",
                ).unwrap();
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn compile_chunk(&mut self, chunk: &Chunk) -> Result<FunctionValue<'ctx>, String> {
        self.init_vm_stack();

        let fn_type = self.context.i64_type().fn_type(&[], false);
        let function = self.module.add_function("compiled_chunk", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        let mut ip = 0;
        while ip < chunk.code.len() {
            let op = Op::from(chunk.code[ip]);
            self.compile_instruction(op, chunk, &mut ip)?;
        }

        let _ = self.builder.build_return(Some(&self.context.i64_type().const_int(0, false)));

        Ok(function)
    }

    fn compile_instruction(&mut self, op: Op, chunk: &Chunk, ip: &mut usize) -> Result<(), String> {
        match op {
            Op::Halt => {
                let _ = self.builder.build_return(Some(&self.context.i64_type().const_int(0, false)));
            }
            Op::Const => {
                *ip += 1;
                let const_idx = chunk.code[*ip] as usize;
                let constant = &chunk.constants[const_idx];
                
                let llvm_val = match constant {
                    Constant::Int(i) => {
                        self.context.i64_type().const_int(*i as u64, false).as_basic_value_enum()
                    }
                    Constant::Float(f) => {
                        self.context.f64_type().const_float(*f).as_basic_value_enum()
                    }
                    Constant::Bool(b) => {
                        self.context.bool_type().const_int(if *b { 1 } else { 0 }, false).as_basic_value_enum()
                    }
                    Constant::String(_) => {
                        self.context.ptr_type(inkwell::AddressSpace::default()).const_null().as_basic_value_enum()
                    }
                    Constant::Unit => {
                        self.context.i64_type().const_zero().as_basic_value_enum()
                    }
                    _ => {
                        self.context.i64_type().const_zero().as_basic_value_enum()
                    }
                };
                self.push_value(llvm_val);
            }
            Op::Add => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_add(
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "add_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Sub => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_sub(
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "sub_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Mul => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_mul(
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "mul_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Div => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_signed_div(
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "div_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Mod => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_signed_rem(
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "mod_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Eq => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "eq_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Neq => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "neq_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Gt => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_compare(
                            inkwell::IntPredicate::SGT,
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "gt_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Lt => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_int_compare(
                            inkwell::IntPredicate::SLT,
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "lt_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::And => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_and(
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "and_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Or => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = self.builder.build_or(
                            left_val.into_int_value(),
                            right_val.into_int_value(),
                            "or_result"
                        ).unwrap().as_basic_value_enum();
                        self.push_value(result);
                    }
                }
            }
            Op::Not => {
                if let Some(val) = self.pop_value() {
                    let result = self.builder.build_not(
                        val.into_int_value(),
                        "not_result"
                    ).unwrap().as_basic_value_enum();
                    self.push_value(result);
                }
            }
            Op::Negate => {
                if let Some(val) = self.pop_value() {
                    let result = self.builder.build_int_neg(
                        val.into_int_value(),
                        "neg_result"
                    ).unwrap().as_basic_value_enum();
                    self.push_value(result);
                }
            }
            Op::Print => { let _ = self.pop_value(); }
            Op::PrintMulti => { *ip += 1; let count = chunk.code[*ip]; for _ in 0..count { let _ = self.pop_value(); } }
            Op::Return => {
                if let Some(return_val) = self.pop_value() {
                    let _ = self.builder.build_return(Some(&return_val));
                } else {
                    let _ = self.builder.build_return(Some(&self.context.i64_type().const_zero()));
                }
            }
            Op::Jump => { *ip += 1; let offset_high = chunk.code[*ip] as usize; *ip += 1; let offset_low = chunk.code[*ip] as usize; *ip = (offset_high << 8) | offset_low; return Ok(()); }
            Op::JumpIfFalse => { *ip += 1; let _ = chunk.code[*ip]; *ip += 1; let _ = chunk.code[*ip]; let _ = self.pop_value(); }
            Op::Loop => {}
            Op::Call => { *ip += 1; let arg_count = chunk.code[*ip] as usize; for _ in 0..arg_count { let _ = self.pop_value(); } }
            Op::GetLocal | Op::GetGlobal => { *ip += 1; self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::SetLocal | Op::SetGlobal => { *ip += 1; let _ = self.pop_value(); }
            Op::DefineGlobal => { *ip += 1; let _ = self.pop_value(); }
            Op::BuildList => { *ip += 1; let count = chunk.code[*ip] as usize; for _ in 0..count { let _ = self.pop_value(); } self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::BuildMap => { *ip += 1; let count = chunk.code[*ip] as usize; for _ in 0..(count*2) { let _ = self.pop_value(); } self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::GetIndex => { let _ = self.pop_value(); let _ = self.pop_value(); self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::SetIndex => { let _ = self.pop_value(); let _ = self.pop_value(); let _ = self.pop_value(); }
            Op::Len | Op::IsList | Op::IsMap | Op::Slice | Op::MapContainsKey => { if self.pop_value().is_some() { self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); } }
            Op::Slide => { *ip += 1; let _ = chunk.code[*ip]; let _ = self.pop_value(); }
            Op::ToString => { if self.pop_value().is_some() { self.push_value(self.context.ptr_type(inkwell::AddressSpace::default()).const_null().as_basic_value_enum()); } }
            Op::Unit => { self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::Dup => { if let Some(val) = self.peek_value() { self.push_value(val); } }
            Op::Pop => { let _ = self.pop_value(); }
            Op::Spawn => { *ip += 1; self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::Send => { let _ = self.pop_value(); let _ = self.pop_value(); }
            Op::Receive | Op::SelfId => { self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::Import => { *ip += 1; self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::Try => { *ip += 1; *ip += 1; }
            Op::Throw => { let _ = self.pop_value(); }
            Op::EndTry => {}
            Op::BuildClass => { *ip += 1; let _ = chunk.code[*ip]; *ip += 1; let has_super = chunk.code[*ip]; if has_super != 0 { *ip += 1; } *ip += 1; let method_count = chunk.code[*ip] as usize; for _ in 0..method_count { let _ = self.pop_value(); } self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::GetSuper => { *ip += 1; let _ = chunk.code[*ip]; let _ = self.pop_value(); let _ = self.pop_value(); self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::Method => { *ip += 1; let _ = chunk.code[*ip]; let _ = self.pop_value(); let _ = self.pop_value(); }
            Op::Closure => { *ip += 1; self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::GetUpvalue => { *ip += 1; let _ = chunk.code[*ip]; self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::SetUpvalue => { *ip += 1; let _ = chunk.code[*ip]; let _ = self.pop_value(); }
            Op::CloseUpvalue => { let _ = self.pop_value(); }
            Op::CreateSharedMemory => { let _ = self.pop_value(); self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::ReadSharedMemory => { let _ = self.pop_value(); let _ = self.pop_value(); self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::WriteSharedMemory => { let _ = self.pop_value(); let _ = self.pop_value(); let _ = self.pop_value(); }
            Op::LockSharedMemory | Op::UnlockSharedMemory => { let _ = self.pop_value(); }
            Op::SpawnLink => { *ip += 1; self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            Op::Link => { let _ = self.pop_value(); let _ = self.pop_value(); }
            Op::Monitor => { let _ = self.pop_value(); }
            Op::Register => { *ip += 1; let _ = self.pop_value(); }
            Op::Unregister => { *ip += 1; }
            Op::WhereIs => { *ip += 1; self.push_value(self.context.i64_type().const_zero().as_basic_value_enum()); }
            // Atomic operations not supported in AOT mode yet
            _ => {}
        }

        *ip += 1;
        Ok(())
    }

    pub fn execute_function(&self, func: FunctionValue) -> Result<i64, String> {
        let result = unsafe {
            self.execution_engine.run_function(func, &[]).as_int(false)
        };
        Ok(result as i64)
    }

    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }

    pub fn print_ir(&self) {
        println!("{}", self.module.print_to_string().to_string());
    }
}
