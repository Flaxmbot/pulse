//! LLVM Codegen for Pulse

use crate::ast::*;
use crate::types::Type;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{FunctionValue, PointerValue, IntValue, BasicValueEnum};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::OptimizationLevel;
use std::collections::HashMap;

pub struct LLVMCodegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    variables: HashMap<String, PointerValue<'ctx>>,
}

impl<'ctx> LLVMCodegen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let mut codegen = Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
        };
        codegen.declare_builtins();
        codegen
    }

    fn declare_builtins(&mut self) {
        let i64_type = self.context.i64_type();
        let void_type = self.context.void_type();
        
        // pulse_println(i64) -> void
        let println_type = void_type.fn_type(&[i64_type.into()], false);
        self.module.add_function("pulse_println", println_type, None);

        // pulse_clock() -> f64
        let clock_type = self.context.f64_type().fn_type(&[], false);
        self.module.add_function("pulse_clock", clock_type, None);
    }

    pub fn gen_script(&mut self, script: &Script) -> Result<(), String> {
        for decl in &script.declarations {
            self.gen_declaration(decl)?;
        }
        Ok(())
    }

    fn gen_declaration(&mut self, decl: &Decl) -> Result<(), String> {
        match decl {
            Decl::Function(name, params, ret, body) => {
                self.gen_function(name, params, ret, body)?;
            },
            Decl::Stmt(stmt) => {
                // For now, statements in the global scope are not allowed in AOT
                // unless we wrap them in a main function.
                return Err("Top-level statements not yet supported in AOT codegen".into());
            },
            _ => return Err(format!("Unsupported declaration in AOT codegen: {:?}", decl)),
        }
        Ok(())
    }

    fn gen_function(&mut self, name: &str, params: &[crate::types::TypedParam], ret: &Option<Type>, body: &[Stmt]) -> Result<FunctionValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = params.iter().map(|_| i64_type.into()).collect();
        let fn_type = i64_type.fn_type(&param_types, false);
        let function = self.module.add_function(name, fn_type, None);

        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        self.variables.clear();
        for (i, arg) in function.get_param_iter().enumerate() {
            let arg_name = &params[i].name;
            let alloca = self.create_entry_block_alloca(function, arg_name)?;
            self.builder.build_store(alloca, arg).map_err(|e| e.to_string())?;
            self.variables.insert(arg_name.clone(), alloca);
        }

        for stmt in body {
            self.gen_statement(stmt)?;
        }

        // Default return 0
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder.build_return(Some(&i64_type.const_int(0, false))).map_err(|e| e.to_string())?;
        }

        Ok(function)
    }

    fn gen_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    let val = self.gen_expression(e)?;
                    self.builder.build_return(Some(&val)).map_err(|e| e.to_string())?;
                } else {
                    self.builder.build_return(None).map_err(|e| e.to_string())?;
                }
            },
            Stmt::Expression(expr) => {
                self.gen_expression(expr)?;
            },
            Stmt::If(condition, then_body, else_body) => {
                let cond = self.gen_expression(condition)?.into_int_value();
                let zero = self.context.i64_type().const_int(0, false);
                let cond_bool = self.builder.build_int_compare(inkwell::IntPredicate::NE, cond, zero, "ifcond").map_err(|e| e.to_string())?;

                let function = self.builder.get_insert_block().unwrap().get_parent().unwrap();
                let then_bb = self.context.append_basic_block(function, "then");
                let else_bb = self.context.append_basic_block(function, "else");
                let merge_bb = self.context.append_basic_block(function, "ifmerge");

                self.builder.build_conditional_branch(cond_bool, then_bb, else_bb).map_err(|e| e.to_string())?;

                // Then block
                self.builder.position_at_end(then_bb);
                for s in then_body {
                    self.gen_statement(s)?;
                }
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(merge_bb).map_err(|e| e.to_string())?;
                }

                // Else block
                self.builder.position_at_end(else_bb);
                if let Some(eb) = else_body {
                    for s in eb {
                        self.gen_statement(s)?;
                    }
                }
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(merge_bb).map_err(|e| e.to_string())?;
                }

                // Merge block
                self.builder.position_at_end(merge_bb);
            },
            Stmt::While(condition, body) => {
                let function = self.builder.get_insert_block().unwrap().get_parent().unwrap();
                let cond_bb = self.context.append_basic_block(function, "whilecond");
                let body_bb = self.context.append_basic_block(function, "whilebody");
                let end_bb = self.context.append_basic_block(function, "whileend");

                self.builder.build_unconditional_branch(cond_bb).map_err(|e| e.to_string())?;

                // Cond block
                self.builder.position_at_end(cond_bb);
                let cond = self.gen_expression(condition)?.into_int_value();
                let zero = self.context.i64_type().const_int(0, false);
                let cond_bool = self.builder.build_int_compare(inkwell::IntPredicate::NE, cond, zero, "whilecond").map_err(|e| e.to_string())?;
                self.builder.build_conditional_branch(cond_bool, body_bb, end_bb).map_err(|e| e.to_string())?;

                // Body block
                self.builder.position_at_end(body_bb);
                for s in body {
                    self.gen_statement(s)?;
                }
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(cond_bb).map_err(|e| e.to_string())?;
                }

                // End block
                self.builder.position_at_end(end_bb);
            },
            _ => return Err(format!("Unsupported statement in AOT codegen: {:?}", stmt)),
        }
        Ok(())
    }

    fn gen_expression(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expr::Literal(pulse_core::Constant::Int(i)) => {
                Ok(self.context.i64_type().const_int(*i as u64, false).into())
            },
            Expr::Binary(left, op, right) => {
                let l = self.gen_expression(left)?.into_int_value();
                let r = self.gen_expression(right)?.into_int_value();
                match op {
                    BinOp::Add => Ok(self.builder.build_int_add(l, r, "addtmp").map_err(|e| e.to_string())?.into()),
                    BinOp::Sub => Ok(self.builder.build_int_sub(l, r, "subtmp").map_err(|e| e.to_string())?.into()),
                    BinOp::Mul => Ok(self.builder.build_int_mul(l, r, "multmp").map_err(|e| e.to_string())?.into()),
                    BinOp::Greater => {
                        let res = self.builder.build_int_compare(inkwell::IntPredicate::SGT, l, r, "cmptmp").map_err(|e| e.to_string())?;
                        Ok(self.builder.build_int_z_extend(res, self.context.i64_type(), "booltmp").map_err(|e| e.to_string())?.into())
                    },
                    BinOp::Less => {
                        let res = self.builder.build_int_compare(inkwell::IntPredicate::SLT, l, r, "cmptmp").map_err(|e| e.to_string())?;
                        Ok(self.builder.build_int_z_extend(res, self.context.i64_type(), "booltmp").map_err(|e| e.to_string())?.into())
                    },
                    _ => Err(format!("Unsupported binary op in AOT: {:?}", op)),
                }
            },
            Expr::Variable(name) => {
                let var = self.variables.get(name).ok_or(format!("Undefined variable: {}", name))?;
                self.builder.build_load(self.context.i64_type(), *var, name).map_err(|e| e.to_string())
            },
            Expr::Call(callee, args) => {
                let Expr::Variable(func_name) = callee.as_ref() else {
                    return Err("Complex callee not yet supported in AOT".into());
                };
                let function = self.module.get_function(func_name).ok_or(format!("Undefined function: {}", func_name))?;
                
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.gen_expression(arg)?.into());
                }

                Ok(self.builder.build_call(function, &compiled_args, "calltmp").map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| self.context.i64_type().const_int(0, false).into()))
            },
            _ => Err(format!("Unsupported expression in AOT: {:?}", expr)),
        }
    }

    fn create_entry_block_alloca(&self, function: FunctionValue<'ctx>, name: &str) -> Result<PointerValue<'ctx>, String> {
        let builder = self.context.create_builder();
        let entry = function.get_first_basic_block().ok_or("Function has no basic blocks")?;
        match entry.get_first_instruction() {
            Some(inst) => builder.position_before(&inst),
            None => builder.position_at_end(entry),
        }
        builder.build_alloca(self.context.i64_type(), name).map_err(|e| e.to_string())
    }

    pub fn verify(&self) -> bool {
        self.module.verify().is_ok()
    }

    pub fn print_to_stderr(&self) {
        self.module.print_to_stderr();
    }
}
