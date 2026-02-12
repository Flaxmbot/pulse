use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::targets::{InitializationConfig, Target};
use inkwell::values::{FunctionValue, BasicValueEnum, PointerValue};
use inkwell::types::{BasicType, BasicTypeEnum};
use std::collections::HashMap;

use pulse_core::{Chunk, Op, Constant, Value as PulseValue};

pub struct LLVMBackend<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    // Stack pointer for the virtual machine
    stack_ptr: Option<PointerValue<'ctx>>,
    stack_top: i32,
}

impl<'ctx> LLVMBackend<'ctx> {
    pub fn new(context: &'ctx Context) -> Result<Self, String> {
        // Initialize LLVM targets
        Target::initialize_native_target()
            .map_err(|e| format!("Failed to initialize native target: {}", e))?;
        Target::initialize_native_asmprinter()
            .map_err(|e| format!("Failed to initialize asm printer: {}", e))?;

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

    /// Initialize the virtual machine stack
    fn init_vm_stack(&mut self) {
        // Create a stack array (for demonstration, we'll use a fixed size)
        let stack_size = self.context.i32_type().const_int(1024, false);
        let stack_type = self.context.i64_type().array_type(stack_size.into());
        let stack_ptr = self.builder.build_alloca(stack_type, "vm_stack");
        self.stack_ptr = Some(stack_ptr);
    }

    /// Push a value onto the LLVM-managed stack
    fn push_value(&mut self, value: BasicValueEnum<'ctx>) {
        if let Some(stack_ptr) = self.stack_ptr {
            // Calculate the address of the current top of stack
            let idx = self.context.i32_type().const_int(self.stack_top as u64, false);
            let stack_element_ptr = unsafe {
                self.builder.build_gep(
                    self.context.i64_type().array_type(self.context.i32_type().const_int(1024, false)).ptr_type(inkwell::AddressSpace::default()),
                    stack_ptr,
                    &[self.context.i32_type().const_zero(), idx],
                    "stack_element_ptr",
                )
            };
            
            // Store the value at that address
            self.builder.build_store(stack_element_ptr, value);
            self.stack_top += 1;
        }
    }

    /// Pop a value from the LLVM-managed stack
    fn pop_value(&mut self) -> Option<BasicValueEnum<'ctx>> {
        if self.stack_top > 0 {
            self.stack_top -= 1;
            if let Some(stack_ptr) = self.stack_ptr {
                // Calculate the address of the element to pop
                let idx = self.context.i32_type().const_int(self.stack_top as u64, false);
                let stack_element_ptr = unsafe {
                    self.builder.build_gep(
                        self.context.i64_type().array_type(self.context.i32_type().const_int(1024, false)).ptr_type(inkwell::AddressSpace::default()),
                        stack_ptr,
                        &[self.context.i32_type().const_zero(), idx],
                        "stack_element_ptr",
                    )
                };
                
                // Load the value from that address
                let value = self.builder.build_load(
                    self.context.i64_type(),
                    stack_element_ptr,
                    "popped_value",
                );
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Compile Pulse source code to LLVM IR
    pub fn compile_chunk(&mut self, chunk: &Chunk) -> Result<FunctionValue<'ctx>, String> {
        // Initialize the VM stack
        self.init_vm_stack();

        // Create a function to hold the compiled code
        let fn_type = self.context.i64_type().fn_type(&[], false);
        let function = self.module.add_function("compiled_chunk", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        // Process the bytecode instructions
        let mut ip = 0;
        while ip < chunk.code.len() {
            let op = Op::from(chunk.code[ip]);
            self.compile_instruction(op, chunk, &mut ip)?;
        }

        // Return 0 from function
        self.builder.build_return(Some(&self.context.i64_type().const_int(0, false)));

        Ok(function)
    }

    /// Compile a single bytecode instruction to LLVM IR
    fn compile_instruction(&mut self, op: Op, chunk: &Chunk, ip: &mut usize) -> Result<(), String> {
        match op {
            Op::Halt => {
                // Return from the current function
                self.builder.build_return(Some(&self.context.i64_type().const_int(0, false)));
            }
            Op::Const => {
                *ip += 1; // Move past the opcode
                let const_idx = chunk.code[*ip] as usize;
                
                // Get the constant value
                let constant = &chunk.constants[const_idx];
                
                // Convert to LLVM value based on type
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
                    Constant::String(s) => {
                        // For now, we'll just return a null pointer for strings
                        self.context.ptr_type(inkwell::AddressSpace::default()).const_null().as_basic_value_enum()
                    }
                    Constant::Unit => {
                        self.context.i64_type().const_zero().as_basic_value_enum()
                    }
                };
                
                // Push to stack
                self.push_value(llvm_val);
            }
            Op::Add => {
                // Pop two values from stack, add them, push result
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        // Perform addition based on types
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_add(left_int, right_int, "add_result").as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_add(left_float, right_float, "add_result").as_basic_value_enum()
                        } else {
                            // For mixed types, convert to float and add
                            let left_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                left_val.into_int_value(),
                                self.context.f64_type(),
                                "left_to_float",
                            ).as_basic_value_enum();
                            let right_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                right_val.into_int_value(),
                                self.context.f64_type(),
                                "right_to_float",
                            ).as_basic_value_enum();
                            self.builder.build_float_add(
                                left_float.into_float_value(),
                                right_float.into_float_value(),
                                "add_result"
                            ).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Sub => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_sub(left_int, right_int, "sub_result").as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_sub(left_float, right_float, "sub_result").as_basic_value_enum()
                        } else {
                            // For mixed types, convert to float and subtract
                            let left_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                left_val.into_int_value(),
                                self.context.f64_type(),
                                "left_to_float",
                            ).as_basic_value_enum();
                            let right_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                right_val.into_int_value(),
                                self.context.f64_type(),
                                "right_to_float",
                            ).as_basic_value_enum();
                            self.builder.build_float_sub(
                                left_float.into_float_value(),
                                right_float.into_float_value(),
                                "sub_result"
                            ).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Mul => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_mul(left_int, right_int, "mul_result").as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_mul(left_float, right_float, "mul_result").as_basic_value_enum()
                        } else {
                            // For mixed types, convert to float and multiply
                            let left_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                left_val.into_int_value(),
                                self.context.f64_type(),
                                "left_to_float",
                            ).as_basic_value_enum();
                            let right_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                right_val.into_int_value(),
                                self.context.f64_type(),
                                "right_to_float",
                            ).as_basic_value_enum();
                            self.builder.build_float_mul(
                                left_float.into_float_value(),
                                right_float.into_float_value(),
                                "mul_result"
                            ).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Div => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_signed_div(left_int, right_int, "div_result").as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_div(left_float, right_float, "div_result").as_basic_value_enum()
                        } else {
                            // For mixed types, convert to float and divide
                            let left_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                left_val.into_int_value(),
                                self.context.f64_type(),
                                "left_to_float",
                            ).as_basic_value_enum();
                            let right_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                right_val.into_int_value(),
                                self.context.f64_type(),
                                "right_to_float",
                            ).as_basic_value_enum();
                            self.builder.build_float_div(
                                left_float.into_float_value(),
                                right_float.into_float_value(),
                                "div_result"
                            ).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Mod => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_signed_rem(left_int, right_int, "mod_result").as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_rem(left_float, right_float, "mod_result").as_basic_value_enum()
                        } else {
                            // For mixed types, convert to float and mod
                            let left_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                left_val.into_int_value(),
                                self.context.f64_type(),
                                "left_to_float",
                            ).as_basic_value_enum();
                            let right_float = self.builder.build_cast(
                                inkwell::values::InstructionOpcode::SIToFP,
                                right_val.into_int_value(),
                                self.context.f64_type(),
                                "right_to_float",
                            ).as_basic_value_enum();
                            self.builder.build_float_rem(
                                left_float.into_float_value(),
                                right_float.into_float_value(),
                                "mod_result"
                            ).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Eq => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                left_int,
                                right_int,
                                "eq_result"
                            ).as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OEQ,
                                left_float,
                                right_float,
                                "eq_result"
                            ).as_basic_value_enum()
                        } else {
                            // For different types, they're not equal
                            self.context.bool_type().const_int(0, false).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Neq => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::NE,
                                left_int,
                                right_int,
                                "neq_result"
                            ).as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::ONE,
                                left_float,
                                right_float,
                                "neq_result"
                            ).as_basic_value_enum()
                        } else {
                            // For different types, they're not equal
                            self.context.bool_type().const_int(1, false).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Gt => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::SGT,
                                left_int,
                                right_int,
                                "gt_result"
                            ).as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OGT,
                                left_float,
                                right_float,
                                "gt_result"
                            ).as_basic_value_enum()
                        } else {
                            // For different types, default to false
                            self.context.bool_type().const_int(0, false).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::Lt => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        let result = if left_val.is_int_value() && right_val.is_int_value() {
                            let left_int = left_val.into_int_value();
                            let right_int = right_val.into_int_value();
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::SLT,
                                left_int,
                                right_int,
                                "lt_result"
                            ).as_basic_value_enum()
                        } else if left_val.is_float_value() && right_val.is_float_value() {
                            let left_float = left_val.into_float_value();
                            let right_float = right_val.into_float_value();
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OLT,
                                left_float,
                                right_float,
                                "lt_result"
                            ).as_basic_value_enum()
                        } else {
                            // For different types, default to false
                            self.context.bool_type().const_int(0, false).as_basic_value_enum()
                        };
                        
                        self.push_value(result);
                    }
                }
            }
            Op::And => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        // Convert values to booleans (non-zero is true)
                        let left_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            left_val.into_int_value(),
                            self.context.i64_type().const_zero(),
                            "left_bool"
                        );
                        let right_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            right_val.into_int_value(),
                            self.context.i64_type().const_zero(),
                            "right_bool"
                        );
                        
                        let result = self.builder.build_and(left_bool, right_bool, "and_result");
                        self.push_value(result.as_basic_value_enum());
                    }
                }
            }
            Op::Or => {
                if let Some(right_val) = self.pop_value() {
                    if let Some(left_val) = self.pop_value() {
                        // Convert values to booleans (non-zero is true)
                        let left_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            left_val.into_int_value(),
                            self.context.i64_type().const_zero(),
                            "left_bool"
                        );
                        let right_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            right_val.into_int_value(),
                            self.context.i64_type().const_zero(),
                            "right_bool"
                        );
                        
                        let result = self.builder.build_or(left_bool, right_bool, "or_result");
                        self.push_value(result.as_basic_value_enum());
                    }
                }
            }
            Op::Not => {
                if let Some(val) = self.pop_value() {
                    // Convert value to boolean (non-zero is true, then negate)
                    let bool_val = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        val.into_int_value(),
                        self.context.i64_type().const_zero(),
                        "not_result"
                    );
                    self.push_value(bool_val.as_basic_value_enum());
                }
            }
            Op::Negate => {
                if let Some(val) = self.pop_value() {
                    let result = if val.is_int_value() {
                        let int_val = val.into_int_value();
                        self.builder.build_int_neg(int_val, "neg_result").as_basic_value_enum()
                    } else if val.is_float_value() {
                        let float_val = val.into_float_value();
                        self.builder.build_float_neg(float_val, "neg_result").as_basic_value_enum()
                    } else {
                        // For other types, return zero
                        self.context.i64_type().const_zero().as_basic_value_enum()
                    };
                    
                    self.push_value(result);
                }
            }
            Op::Print => {
                // For now, just pop the value to print (in a real implementation, we'd call printf)
                let _ = self.pop_value();
            }
            Op::PrintMulti => {
                *ip += 1; // Read the count of arguments to print
                let count = chunk.code[*ip];
                
                // Pop the specified number of values (in a real implementation, we'd print them)
                for _ in 0..count {
                    let _ = self.pop_value();
                }
            }
            Op::Return => {
                // Return from current function
                if let Some(return_val) = self.pop_value() {
                    self.builder.build_return(Some(&return_val));
                } else {
                    self.builder.build_return(Some(&self.context.i64_type().const_zero()));
                }
            }
            // Handle other opcodes as needed
            _ => {
                // For now, we'll just handle the most common opcodes
                // Additional opcodes can be implemented as needed
            }
        }

        *ip += 1; // Move to next instruction
        Ok(())
    }

    /// Execute the compiled function
    pub fn execute_function(&self, func: FunctionValue) -> Result<i64, String> {
        let result = unsafe {
            self.execution_engine
                .run_function(func, &[])
                .as_int()
        };
        Ok(result as i64)
    }

    /// Get the LLVM module for inspection or further processing
    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }

    /// Print the generated LLVM IR
    pub fn print_ir(&self) {
        println!("{}", self.module.print_to_string().to_string());
    }
}