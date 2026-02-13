use crate::lexer::Token;
use pulse_core::{Chunk, Op, PulseError, PulseResult, Constant}; 
use pulse_core::object::Function;
use crate::parser::Parser;
use std::sync::Arc;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
enum Precedence {
    None,
    Assignment,  // =
    Or,          // or
    And,         // and
    Equality,    // == !=
    Comparison,  // < > <= >=
    Term,        // + -
    Factor,      // * /
    Unary,       // ! -
    Call,        // . ()
    Primary,
}

impl Precedence {
    fn next(&self) -> Self {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

type ParseFn<'a, 'b> = fn(&mut Compiler<'a, 'b>, bool) -> PulseResult<()>;

struct ParseRule<'a, 'b> {
    prefix: Option<ParseFn<'a, 'b>>,
    infix: Option<ParseFn<'a, 'b>>,
    precedence: Precedence,
}

struct Local {
    name: Token,
    depth: i32,
    is_captured: bool,
}

struct Loop {
    start_ip: usize,
    break_jumps: Vec<usize>,
}

struct CompilerUpvalue {
    index: u8,
    is_local: bool,
}

#[derive(PartialEq, Clone, Copy)]
pub enum FunctionType {
    Script,
    Function,
    Method,
    Initializer,
}

pub struct Compiler<'a, 'b> {
    parser: *mut Parser<'a>,
    enclosing: *mut Compiler<'a, 'b>,
    chunk: Chunk,
    locals: Vec<Local>,
    upvalues: Vec<CompilerUpvalue>,
    scope_depth: i32,
    loops: Vec<Loop>,
    function_type: FunctionType,
    module_path: Option<String>,
}

pub fn compile(source: &str, module_path: Option<String>) -> PulseResult<Chunk> {
    let mut parser = Parser::new(source);
    let mut compiler = Compiler::new(&mut parser as *mut Parser, std::ptr::null_mut(), FunctionType::Script, module_path);
    compiler.compile_script()
}

impl<'a, 'b> Compiler<'a, 'b> {
    pub fn new(parser: *mut Parser<'a>, enclosing: *mut Compiler<'a, 'b>, function_type: FunctionType, module_path: Option<String>) -> Self {
        let mut locals = Vec::new();
        // Reserve slot 0
        locals.push(Local {
            name: if function_type == FunctionType::Method || function_type == FunctionType::Initializer {
                Token::Identifier("this".to_string())
            } else {
                Token::Identifier("".to_string())
            },
            depth: 0,
            is_captured: false,
        });

        Self {
            parser,
            enclosing,
            chunk: Chunk::new(),
            locals,
            upvalues: Vec::new(),
            scope_depth: 0,
            loops: Vec::new(),
            function_type,
            module_path,
        }
    }

    fn parser(&mut self) -> &mut Parser<'a> {
        unsafe { &mut *self.parser }
    }

    pub fn compile_script(&mut self) -> PulseResult<Chunk> {
        self.parser().advance()?;
        
        while !self.matches(Token::Eof)? {
            self.declaration()?;
        }
        
        self.emit_byte(Op::Unit as u8);
        self.emit_byte(Op::Return as u8); 
        Ok(self.chunk.clone())
    }

    // --- Declarations ---
    fn declaration(&mut self) -> PulseResult<()> {
        if self.matches(Token::Fn)? || self.matches(Token::Def)? {
            self.fun_declaration()
        } else if self.matches(Token::Let)? {
            self.var_declaration()
        } else if self.matches(Token::Actor)? {
            self.actor_declaration()
        } else if self.matches(Token::Class)? {
            self.class_declaration()
        } else if self.matches(Token::Shared)? {
            self.shared_memory_declaration()
        } else {
            self.statement()
        }
    }

        

    fn fun_declaration(&mut self) -> PulseResult<()> {
        let global = self.parse_variable("Expect function name.")?;
        let name = if let Token::Identifier(s) = &self.parser().previous { s.clone() } else { "".into() };
        self.function(FunctionType::Function, name)?;
        self.define_variable(global); 
        Ok(())
    }

    fn function(&mut self, function_type: FunctionType, name: String) -> PulseResult<()> {
        // Create new compiler for body
        let mut compiler = Compiler::new(self.parser, self as *mut Compiler, function_type, self.module_path.clone());
        compiler.begin_scope(); 

        compiler.consume(Token::LeftParen, "Expect '(' after function name.")?;
        
        let mut arity = 0;
        if !compiler.check(Token::RightParen) {
            loop {
                arity += 1;
                if compiler.locals.len() > 255 {
                    return Err(PulseError::CompileError("Cannot have more than 255 parameters.".into(), 0));
                }
                
                let param_constant = compiler.parse_variable("Expect parameter name.")?;
                compiler.define_variable(param_constant);
                
                // Optional type annotation: `: Type`
                if compiler.matches(Token::Colon)? {
                    // Parse and ignore type annotation for now (future: type checking pass)
                    let _ = compiler.parse_type()?;
                }

                if !compiler.matches(Token::Comma)? {
                    break;
                }
            }
        }
        compiler.consume(Token::RightParen, "Expect ')' after parameters.")?;
        
        // Optional return type annotation: `-> Type`
        if compiler.matches(Token::Arrow)? {
            let _ = compiler.parse_type()?;
        }
        
        compiler.consume(Token::LeftBrace, "Expect '{' before function body.")?;
        compiler.block()?;
        
        // Emit return nil in case user didn't
        compiler.emit_return();
        
        let chunk = compiler.chunk.clone();
        let upvalue_count = compiler.upvalues.len();
        let function = Function {
            arity,
            chunk: Arc::new(chunk),
            name,
            upvalue_count,
            module_path: self.module_path.clone(),
        };
        
        // Add function to Parent (self) constants
        let idx = self.chunk.add_constant(Constant::Function(Box::new(function)));
        self.emit_byte(Op::Closure as u8);
        self.emit_u16(idx as u16);
        
        // Emit upvalue capturing info
        for i in 0..upvalue_count {
            self.emit_byte(if compiler.upvalues[i].is_local { 1 } else { 0 });
            self.emit_byte(compiler.upvalues[i].index);
        }
        
        Ok(())
    }
    
    // Helper to emit return
    fn emit_return(&mut self) {
        if self.function_type == FunctionType::Initializer {
            self.emit_byte(Op::GetLocal as u8);
            self.emit_byte(0); // Return 'this'
        } else {
            self.emit_byte(Op::Unit as u8); // Default return
        }
        self.emit_byte(Op::Return as u8);
    }

    fn var_declaration(&mut self) -> PulseResult<()> {
        let global = self.parse_variable("Expect variable name.")?;
        
        if self.matches(Token::Equal)? {
            self.expression()?;
        } else {
            self.emit_constant(Constant::Unit); // Default to nil/unit?
        }
        
        self.consume(Token::Semicolon, "Expect ';' after variable declaration.")?;
        self.define_variable(global);
        Ok(())
    }

    fn actor_declaration(&mut self) -> PulseResult<()> {
        let global = self.parse_variable("Expect actor name.")?;
        let name = if let Token::Identifier(s) = &self.parser().previous { s.clone() } else { "".into() };

        // Create a function that represents the actor behavior
        // This is a simplified approach - in a real implementation, actors would be more complex
        self.actor_function(FunctionType::Function, name)?;
        self.define_variable(global);
        Ok(())
    }

    fn class_declaration(&mut self) -> PulseResult<()> {
        self.consume_identifier("Expected class name")?;
        let class_name = if let Token::Identifier(s) = &self.parser().previous {
            s.clone()
        } else {
            return Err(PulseError::CompileError("Expected class name".into(), 0));
        };

        let name_idx = self.identifier_constant(&Token::Identifier(class_name.clone()))?;
        
        self.declare_variable()?;

        self.emit_byte(Op::BuildClass as u8);
        self.emit_u16(name_idx);
        
        // Parse superclass
        if self.matches(Token::Extends)? {
            self.consume_identifier("Expected superclass name")?;
            let super_name = if let Token::Identifier(s) = &self.parser().previous {
                s.clone()
            } else {
                return Err(PulseError::CompileError("Expected superclass name".into(), 0));
            };
            
            // Push superclass onto stack?
            // Op::BuildClass expects [Superclass] if has_super?
            // Current VM Op::BuildClass doesn't pop superclass. It takes index?
            // Let's check VM implementation...
            // It reads has_super, super_idx.
            // So we just emit index.
            
            self.emit_byte(1); // has_super
            let super_idx = self.identifier_constant(&Token::Identifier(super_name))?;
            self.emit_u16(super_idx);
        } else {
            self.emit_byte(0); // no superclass
        }

        self.define_variable(name_idx);

        // Push class back on stack to attach methods
        self.named_variable(Token::Identifier(class_name.clone()), false)?;

        self.consume(Token::LeftBrace, "Expect '{' before class body.")?;

        // Create scope for super
        self.begin_scope();
        // TODO: Register 'super' if inheritance exists

        while !self.check(Token::RightBrace) && !self.check(Token::Eof) {
            if self.matches(Token::Fn)? || self.matches(Token::Def)? {
                self.consume_identifier("Expected method name")?;
                let method_name = if let Token::Identifier(s) = &self.parser().previous { s.clone() } else { "".into() };
                let method_name_idx = self.identifier_constant(&Token::Identifier(method_name.clone()))?;

                if method_name == "init" {
                   self.function(FunctionType::Initializer, format!("{}.{}", class_name, method_name))?;
                } else {
                   self.function(FunctionType::Method, format!("{}.{}", class_name, method_name))?;
                }
                
                self.emit_byte(Op::Method as u8);
                self.emit_u16(method_name_idx);
            } else {
                return Err(PulseError::CompileError("Expected method definition in class.".into(), 0));
            }
        }

        self.consume(Token::RightBrace, "Expect '}' after class body.")?;
        self.end_scope();
        
        self.emit_byte(Op::Pop as u8); // Pop class from stack

        Ok(())
    }

    fn actor_function(&mut self, function_type: FunctionType, name: String) -> PulseResult<()> {
        // Create new compiler for actor body
        let mut compiler = Compiler::new(self.parser, self as *mut Compiler, function_type, self.module_path.clone());
        compiler.begin_scope();

        compiler.consume(Token::LeftBrace, "Expect '{' before actor body.")?;
        compiler.block()?;

        // Emit return in actor
        compiler.emit_return();

        let chunk = compiler.chunk.clone();
        let upvalue_count = compiler.upvalues.len();
        let function = Function {
            arity: 0, // Actors typically don't take parameters in this form
            chunk: Arc::new(chunk),
            name,
            upvalue_count,
            module_path: self.module_path.clone(),
        };

        // Add function to Parent (self) constants
        let idx = self.chunk.add_constant(Constant::Function(Box::new(function)));
        self.emit_byte(Op::Closure as u8);
        self.emit_u16(idx as u16);

        // Emit upvalue capturing info
        for i in 0..upvalue_count {
            self.emit_byte(if compiler.upvalues[i].is_local { 1 } else { 0 });
            self.emit_byte(compiler.upvalues[i].index);
        }

        Ok(())
    }

    fn shared_memory_declaration(&mut self) -> PulseResult<()> {
        // Parse: shared memory IDENTIFIER = expression;
        self.consume(Token::Memory, "Expect 'memory' after 'shared'.")?;
        
        let global = self.parse_variable("Expect shared memory name.")?;
        let _name = if let Token::Identifier(s) = &self.parser().previous { s.clone() } else { "".into() };

        self.consume(Token::Equal, "Expect '=' after shared memory name.")?;
        
        // Parse the initial value for the shared memory
        self.expression()?;
        
        self.consume(Token::Semicolon, "Expect ';' after shared memory declaration.")?;
        
        // Emit instruction to create shared memory
        self.emit_byte(Op::CreateSharedMemory as u8);
        
        // Define the shared memory in global scope
        self.emit_byte(Op::DefineGlobal as u8);
        self.emit_u16(global);
        
        Ok(())
    }

    // --- Statements ---
    fn statement(&mut self) -> PulseResult<()> {
        if self.matches(Token::Print)? {
            self.print_statement()?;
        } else if self.matches(Token::If)? {
            self.if_statement()?;
        } else if self.matches(Token::While)? {
            self.while_statement()?;
        } else if self.matches(Token::For)? {
            self.for_statement()?;
        } else if self.matches(Token::Break)? {
            self.break_statement()?;
        } else if self.matches(Token::Continue)? {
            self.continue_statement()?;
        } else if self.matches(Token::Send)? {
            self.send_statement()?;
        } else if self.matches(Token::Link)? {
            self.link_statement()?;
        } else if self.matches(Token::Monitor)? {
            self.monitor_statement()?;
        } else if self.matches(Token::SpawnLink)? {
            self.spawn_link_statement()?;
        } else if self.matches(Token::Return)? {
            self.return_statement()?;
        } else if self.matches(Token::Try)? {
            self.try_statement()?;
        } else if self.matches(Token::Throw)? {
            self.throw_statement()?;
        } else if self.matches(Token::Lock)? {
            self.lock_statement()?;
        } else if self.matches(Token::Unlock)? {
            self.unlock_statement()?;
        } else if self.matches(Token::LeftBrace)? {
            self.begin_scope();
            self.block()?;
            self.end_scope();
        } else {
            self.expression_statement()?;
        }
        Ok(())
    }

    fn send_statement(&mut self) -> PulseResult<()> {
        // send target, msg
        self.expression()?; // target
        self.consume(Token::Comma, "Expect ',' after 'send' target.")?;
        self.expression()?; // msg
        
        self.emit_byte(Op::Send as u8);
        Ok(())
    }

    fn link_statement(&mut self) -> PulseResult<()> {
        // link target_pid
        self.expression()?; // target PID
        self.consume(Token::Semicolon, "Expect ';' after link statement.")?;
        self.emit_byte(Op::Link as u8);
        Ok(())
    }

    fn monitor_statement(&mut self) -> PulseResult<()> {
        // monitor target_pid
        self.expression()?; // target PID
        self.consume(Token::Semicolon, "Expect ';' after monitor statement.")?;
        self.emit_byte(Op::Monitor as u8);
        Ok(())
    }

    fn spawn_link_statement(&mut self) -> PulseResult<()> {
        // spawn_link expression
        self.expression()?; // expression to spawn
        self.consume(Token::Semicolon, "Expect ';' after spawn_link statement.")?;
        self.emit_byte(Op::SpawnLink as u8);
        Ok(())
    }

    fn print_statement(&mut self) -> PulseResult<()> {
        // Check if print is followed by parentheses (multi-arg form)
        if self.matches(Token::LeftParen)? {
            // Multi-argument print: print(arg1, arg2, ...)
            let mut arg_count = 0;
            
            if !self.check(Token::RightParen) {
                loop {
                    self.expression()?;
                    arg_count += 1;
                    
                    if !self.matches(Token::Comma)? {
                        break;
                    }
                }
            }
            
            self.consume(Token::RightParen, "Expect ')' after print arguments.")?;
            self.consume(Token::Semicolon, "Expect ';' after print statement.")?;
            
            // Emit multi-argument print
            self.emit_byte(Op::PrintMulti as u8);
            self.emit_byte(arg_count as u8);
        } else {
            // Single-argument print: print value;
            self.expression()?;
            self.consume(Token::Semicolon, "Expect ';' after value.")?;
            self.emit_byte(Op::Print as u8);
        }
        Ok(())
    }

    fn try_statement(&mut self) -> PulseResult<()> {
        // try { ... } catch e { ... }
        self.consume(Token::LeftBrace, "Expect '{' after 'try'.")?;
        
        // Emit Op::Try with placeholder for handler offset
        self.emit_byte(Op::Try as u8);
        let try_offset = self.chunk.code.len();
        self.emit_byte(0xff); // Placeholder for handler offset (u16)
        self.emit_byte(0xff);
        
        // Compile try block
        self.begin_scope();
        self.block()?;
        self.end_scope();
        
        // Emit Op::EndTry after try block
        self.emit_byte(Op::EndTry as u8);
        
        // Jump over catch block on success
        let success_jump = self.emit_jump(Op::Jump as u8);
        
        // Patch try handler offset to point here (catch block)
        let handler_ip = self.chunk.code.len();
        let offset = handler_ip - try_offset - 2; // Offset from after Op::Try u16
        if offset > u16::MAX as usize {
            return Err(PulseError::CompileError("Try block too large".into(), 0));
        }
        self.chunk.code[try_offset] = ((offset >> 8) & 0xff) as u8;
        self.chunk.code[try_offset + 1] = (offset & 0xff) as u8;
        
        // Consume catch
        self.consume(Token::Catch, "Expect 'catch' after try block.")?;
        
        // Parse exception variable
        self.consume_identifier("Expect exception variable name.")?;
        
        // Begin catch scope and define exception variable
        self.begin_scope();
        self.declare_variable()?;
        self.mark_initialized();
        
        self.consume(Token::LeftBrace, "Expect '{' after catch variable.")?;
        self.block()?;
        self.end_scope();
        
        // Patch success jump
        self.patch_jump(success_jump)?;
        
        Ok(())
    }

    fn throw_statement(&mut self) -> PulseResult<()> {
        // throw expression;
        self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after throw expression.")?;
        self.emit_byte(Op::Throw as u8);
        Ok(())
    }

    fn if_statement(&mut self) -> PulseResult<()> {
        self.consume(Token::LeftParen, "Expect '(' after 'if'.")?;
        self.expression()?;
        self.consume(Token::RightParen, "Expect ')' after condition.")?;

        let then_jump = self.emit_jump(Op::JumpIfFalse as u8);
        self.emit_byte(Op::Pop as u8); // Pop condition
        self.statement()?;

        let else_jump = self.emit_jump(Op::Jump as u8);

        self.patch_jump(then_jump)?;
        self.emit_byte(Op::Pop as u8); // Pop condition if false

        if self.matches(Token::Else)? {
            self.statement()?;
        }
        
        self.patch_jump(else_jump)?;
        Ok(())
    }

    fn while_statement(&mut self) -> PulseResult<()> {
        let loop_start = self.chunk.code.len();
        
        // Push Loop context
        self.loops.push(Loop {
            start_ip: loop_start,
            break_jumps: Vec::new(),
        });

        self.consume(Token::LeftParen, "Expect '(' after 'while'.")?;
        self.expression()?;
        self.consume(Token::RightParen, "Expect ')' after condition.")?;

        let exit_jump = self.emit_jump(Op::JumpIfFalse as u8);
        self.emit_byte(Op::Pop as u8);
        
        self.statement()?;
        
        self.emit_loop(loop_start)?;
        
        self.patch_jump(exit_jump)?;
        self.emit_byte(Op::Pop as u8);
        
        // Pop Loop context and patch breaks
        if let Some(loop_ctx) = self.loops.pop() {
            for break_jump in loop_ctx.break_jumps {
                self.patch_jump(break_jump)?;
            }
        }
        Ok(())
    }

    fn for_statement(&mut self) -> PulseResult<()> {
        // Check for Python-style: for x in iterable { }
        // vs C-style: for (init; cond; incr) body
        
        self.begin_scope();
        
        // Check if next is identifier followed by 'in' (Python style)
        if let Token::Identifier(var_name) = &self.parser().current.clone() {
            let name = var_name.clone();
            self.advance()?;
            
            if self.matches(Token::In)? {
                // Python-style: for x in iterable { }
                return self.for_in_statement(name);
            } else {
                // Not Python-style, but we consumed identifier
                // This is a syntax error for traditional for
                return Err(PulseError::CompileError(
                    "Expect '(' after 'for' or 'in' after identifier.".into(), 
                    self.parser().line()
                ));
            }
        }
        
        // C-style: for (init; cond; incr) body
        self.consume(Token::LeftParen, "Expect '(' after 'for'.")?;
        
        // Init
        if self.matches(Token::Semicolon)? {
            // No init
        } else if self.matches(Token::Let)? {
            self.var_declaration()?;
        } else {
            self.expression_statement()?;
        }
        
        let mut loop_start = self.chunk.code.len();
        
        // Cond
        let mut exit_jump = None;
        if !self.matches(Token::Semicolon)? {
            self.expression()?;
            self.consume(Token::Semicolon, "Expect ';' after loop condition.")?;
            
            exit_jump = Some(self.emit_jump(Op::JumpIfFalse as u8));
            self.emit_byte(Op::Pop as u8);
        }
        
        // Incr
        if !self.matches(Token::RightParen)? {
            let body_jump = self.emit_jump(Op::Jump as u8);
            
            let increment_start = self.chunk.code.len();
            self.expression()?;
            self.emit_byte(Op::Pop as u8);
            self.consume(Token::RightParen, "Expect ')' after for clauses.")?;
            
            self.emit_loop(loop_start)?;
            loop_start = increment_start;
            self.patch_jump(body_jump)?;
        }
        
        // Push Loop context
        self.loops.push(Loop {
            start_ip: loop_start,
            break_jumps: Vec::new(),
        });

        self.statement()?;
        
        self.emit_loop(loop_start)?;
        
        if let Some(jump) = exit_jump {
            self.patch_jump(jump)?;
            self.emit_byte(Op::Pop as u8);
        }
        
        // Pop Loop context and patch breaks
        if let Some(loop_ctx) = self.loops.pop() {
            for break_jump in loop_ctx.break_jumps {
                self.patch_jump(break_jump)?;
            }
        }
        
        self.end_scope();
        Ok(())
    }
    
    fn for_in_statement(&mut self, var_name: String) -> PulseResult<()> {
        // for x in iterable { body }
        // Compiles to:
        // let __iter = iterable;
        // let __idx = 0;
        // while __idx < len(__iter) {
        //     let x = __iter[__idx];
        //     body
        //     __idx = __idx + 1;
        // }
        
        // Evaluate iterable
        self.expression()?;
        
        // Store iterable in a hidden local
        self.add_local(Token::Identifier("__iter".to_string()))?;
        self.mark_initialized();
        
        // Store index (0) in a hidden local
        self.emit_constant(Constant::Int(0));
        self.add_local(Token::Identifier("__idx".to_string()))?;
        self.mark_initialized();
        
        let loop_start = self.chunk.code.len();
        
        // Condition: __idx < len(__iter)
        let idx_token = Token::Identifier("__idx".to_string());
        let idx_slot = self.resolve_local(&idx_token)?;
        let iter_token = Token::Identifier("__iter".to_string());
        let iter_slot = self.resolve_local(&iter_token)?;
        
        // Stack order for __idx < len:
        // 1. Push __idx
        // 2. Push __iter
        // 3. Len (peeks __iter, pushes len) -> stack: [__idx, __iter, len]
        // 4. Slide(1) to remove __iter -> stack: [__idx, len]
        // 5. Lt compares second-from-top < top -> __idx < len ✓
        
        self.emit_byte(Op::GetLocal as u8);
        self.emit_byte(idx_slot);     // Stack: [..., __idx]
        
        self.emit_byte(Op::GetLocal as u8);
        self.emit_byte(iter_slot);    // Stack: [..., __idx, __iter]
        
        self.emit_byte(Op::Len as u8); // Stack: [..., __idx, __iter, len]
        
        self.emit_byte(Op::Slide as u8);
        self.emit_byte(1);             // Stack: [..., __idx, len]
        
        self.emit_byte(Op::Lt as u8);  // Stack: [..., __idx < len]
        
        let exit_jump = self.emit_jump(Op::JumpIfFalse as u8);
        self.emit_byte(Op::Pop as u8); // Pop condition result
        
        // Push Loop context
        self.loops.push(Loop {
            start_ip: loop_start,
            break_jumps: Vec::new(),
        });
        
        // Get current element: __iter[__idx]
        self.emit_byte(Op::GetLocal as u8);
        self.emit_byte(iter_slot);
        self.emit_byte(Op::GetLocal as u8);
        self.emit_byte(idx_slot);
        self.emit_byte(Op::GetIndex as u8);
        
        // Store in loop variable
        self.add_local(Token::Identifier(var_name))?;
        self.mark_initialized();
        
        // Body
        self.consume(Token::LeftBrace, "Expect '{' after 'for ... in ...'.")?;
        self.block()?;
        
        // Pop loop variable
        self.emit_byte(Op::Pop as u8);
        
        // Increment: __idx = __idx + 1
        self.emit_byte(Op::GetLocal as u8);
        self.emit_byte(idx_slot);
        self.emit_constant(Constant::Int(1));
        self.emit_byte(Op::Add as u8);
        self.emit_byte(Op::SetLocal as u8);
        self.emit_byte(idx_slot);
        self.emit_byte(Op::Pop as u8);
        
        // Loop back
        self.emit_loop(loop_start)?;
        
        // Exit
        self.patch_jump(exit_jump)?;
        self.emit_byte(Op::Pop as u8);
        
        // Patch breaks
        if let Some(loop_ctx) = self.loops.pop() {
            for break_jump in loop_ctx.break_jumps {
                self.patch_jump(break_jump)?;
            }
        }
        
        // end_scope() handles popping all locals including __iter, __idx, and loop var
        self.end_scope();
        Ok(())
    }

    fn break_statement(&mut self) -> PulseResult<()> {
        self.consume(Token::Semicolon, "Expect ';' after 'break'.")?;
        
        if self.loops.is_empty() {
             return Err(PulseError::CompileError("Cannot use 'break' outside of a loop.".into(), 0));
        }
        let jump = self.emit_jump(Op::Jump as u8);
        self.loops.last_mut().unwrap().break_jumps.push(jump);
        Ok(())
    }

    fn return_statement(&mut self) -> PulseResult<()> {
        if self.function_type == FunctionType::Script {
            return Err(PulseError::CompileError("Cannot return from top-level script.".into(), 0));
        }
        
        if self.matches(Token::Semicolon)? {
            self.emit_return();
        } else {
            if self.function_type == FunctionType::Initializer {
                return Err(PulseError::CompileError("Cannot return a value from an initializer.".into(), 0));
            }
            self.expression()?;
            self.consume(Token::Semicolon, "Expect ';' after return value.")?;
            self.emit_byte(Op::Return as u8);
        }
        Ok(())
    }

    fn continue_statement(&mut self) -> PulseResult<()> {
        self.consume(Token::Semicolon, "Expect ';' after 'continue'.")?;
        
        let start_ip = self.loops.last().map(|loop_ctx| loop_ctx.start_ip);

        if let Some(ip) = start_ip {
             self.emit_loop(ip)?;
        } else {
            return Err(PulseError::CompileError("Cannot use 'continue' outside of a loop.".into(), 0));
        }
        Ok(())
    }

    fn lock_statement(&mut self) -> PulseResult<()> {
        // Parse: lock(expression);
        self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after lock statement.")?;
        self.emit_byte(Op::LockSharedMemory as u8);
        Ok(())
    }

    fn unlock_statement(&mut self) -> PulseResult<()> {
        // Parse: unlock(expression);
        self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after unlock statement.")?;
        self.emit_byte(Op::UnlockSharedMemory as u8);
        Ok(())
    }

    fn block(&mut self) -> PulseResult<()> {
        while !self.check(Token::RightBrace) && !self.check(Token::Eof) {
            self.declaration()?;
        }
        self.consume(Token::RightBrace, "Expect '}' after block.")?;
        Ok(())
    }

    fn expression_statement(&mut self) -> PulseResult<()> {
        self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after expression.")?;
        self.emit_byte(Op::Pop as u8); // Pop result if used as stmt
        Ok(())
    }

    // --- Expressions ---
    fn expression(&mut self) -> PulseResult<()> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn number(&mut self, _can_assign: bool) -> PulseResult<()> {
        let previous = self.parser().previous.clone();
        match &previous {
            Token::Int(n) => self.emit_constant(Constant::Int(*n)),
            Token::Float(n) => self.emit_constant(Constant::Float(*n)),
            _ => return Err(PulseError::CompileError("Expected number".into(), 0)),
        }
        Ok(())
    }

    fn grouping(&mut self, _can_assign: bool) -> PulseResult<()> {
        self.expression()?;
        self.consume(Token::RightParen, "Expect ')' after expression.")?;
        Ok(())
    }

    fn unary(&mut self, _can_assign: bool) -> PulseResult<()> {
        let operator_type = self.parser().previous.clone();

        // Compile operand
        self.parse_precedence(Precedence::Unary)?;

        // Emit operator instruction
        match operator_type {
            Token::Minus => self.emit_byte(Op::Negate as u8),
            Token::Bang => self.emit_byte(Op::Not as u8),
            _ => return Err(PulseError::CompileError("Invalid unary operator".into(), 0)),
        }
        Ok(())
    }

    fn match_expression(&mut self, _can_assign: bool) -> PulseResult<()> {
        self.expression()?; // Compile subject
        self.consume(Token::LeftBrace, "Expect '{' after match subject.")?;
        
        let mut end_jumps = Vec::new();
        
        while !self.check(Token::RightBrace) && !self.check(Token::Eof) {
            // Match Arm
            self.begin_scope(); // Scope for pattern variables
            self.emit_byte(Op::Dup as u8); // Dup Subject for this arm
            
            let locals_before = self.locals.len();
            let mut failure_jumps = self.compile_pattern()?;
            let locals_after = self.locals.len();
            
            // Guard Clause
            if self.matches(Token::If)? {
                self.expression()?; // Compile Guard Condition
                let fail_guard = self.emit_jump(Op::JumpIfFalse as u8);
                self.emit_byte(Op::Pop as u8); // Pop True
                
                let success_jump = self.emit_jump(Op::Jump as u8); // Jump to Body
                
                // Handle Guard Failure
                self.patch_jump(fail_guard)?;
                self.emit_byte(Op::Pop as u8); // Pop False
                
                // Cleanup locals created by pattern (without result on stack yet)
                let pop_count = locals_after - locals_before;
                for _ in 0..pop_count {
                    self.emit_byte(Op::Pop as u8);
                }
                
                // Restore Subject for next arm (Original subject is below locals)
                // But we just popped locals. So Original Subject is on top.
                // We need to Dup it because next arm expects [Subject] and will Pop it on failure.
                self.emit_byte(Op::Dup as u8); 
                
                failure_jumps.push(self.emit_jump(Op::Jump as u8));
                
                self.patch_jump(success_jump)?;
            }
            
            self.consume(Token::FatArrow, "Expect '=>' after pattern.")?;
            
            // Body
            if self.check(Token::LeftBrace) {
                self.consume(Token::LeftBrace, "Expect '{' start of arm body.")?;
                self.block()?; 
            } else {
                self.expression()?;
                if self.matches(Token::Comma)? {} 
            }
            
            // End Scope manually to preserve Result
            self.scope_depth -= 1;
            let pop_count = self.locals.len() - locals_before;
            // Pop locals UNDER the result
            self.emit_byte(Op::Slide as u8);
            self.emit_byte(pop_count as u8);
             
            // Remove locals from compiler state
            while self.locals.len() > locals_before {
                self.locals.pop();
            }
            
            // Jump to End
            end_jumps.push(self.emit_jump(Op::Jump as u8));
            
            // Handle Failures
            // All failure jumps land here.
            // Stack state: [Subject] (Because compile_pattern contract guarantees Subject restoration on failure)
            for jump in failure_jumps {
                self.patch_jump(jump)?;
            }
            // Pop Subject (failed match for this arm)
            self.emit_byte(Op::Pop as u8);
            
            // Loop continues to next arm
        }
        
        self.consume(Token::RightBrace, "Expect '}' after match arms.")?;
        
        // Final fallback: Pop Subject (original) and return Unit
        self.emit_byte(Op::Pop as u8);
        self.emit_byte(Op::Unit as u8);
        
        // Patch End Jumps
        for jump in end_jumps {
            self.patch_jump(jump)?;
        }
        
        Ok(())
    }

    // Contract: 
    // Start: [Target]
    // Success: [] (Target consumed)
    // Failure (returns jumps): [Target] (Target preserved)
    fn compile_pattern(&mut self) -> PulseResult<Vec<usize>> {
        let mut failure_jumps = Vec::new();
        
        if self.matches(Token::LeftBracket)? {
            // List Pattern: [a, b] or [h | t]
            
            // 1. IsList
            self.emit_byte(Op::Dup as u8); // Dup Target for IsList check
            self.emit_byte(Op::IsList as u8);
            let fail_is_list = self.emit_jump(Op::JumpIfFalse as u8);
            self.emit_byte(Op::Pop as u8); // Pop True
            
            // On Fail IsList: Stack [Target, False].
            // We patch later to Pop False.
            
            let mut index = 0;
            let mut has_tail = false;
            
            if !self.check(Token::RightBracket) {
                loop {
                    // Check for Tail
                    if self.matches(Token::Pipe)? {
                        has_tail = true;
                        
                        // Tail Pattern: [ ... | tail ]
                        // Extract Tail from `index`
                        self.emit_byte(Op::Dup as u8); // Dup List
                        self.emit_byte(Op::Len as u8); // Push Len
                        self.emit_constant(Constant::Int(index as i64));
                        // Gte (>=) is Not Lt (<)
                        self.emit_byte(Op::Lt as u8); 
                        self.emit_byte(Op::Not as u8);
                        let fail_len = self.emit_jump(Op::JumpIfFalse as u8);
                        self.emit_byte(Op::Pop as u8); // Pop True

                        // Extract Tail
                        self.emit_byte(Op::Dup as u8); // Dup List
                        self.emit_constant(Constant::Int(index as i64));
                        self.emit_byte(Op::Slice as u8); // Pops List, Index. Pushes Tail.
                        
                        // Match Tail
                        let sub_failures = self.compile_pattern()?;
                        
                        // Handle Sub-Failures (Stack: [List, Tail])
                        // Tail match failure leaves Tail on stack.
                        // We need to Pop Tail, then we have [List].
                        
                        let success_jump = self.emit_jump(Op::Jump as u8); // Jump over cleanup
                        
                        for jump in sub_failures {
                            self.patch_jump(jump)?;
                            self.emit_byte(Op::Pop as u8); // Pop Tail
                            failure_jumps.push(self.emit_jump(Op::Jump as u8)); // Jump to outer failure
                        }

                        // Handle Len Failure (Stack: [List, False])
                        self.patch_jump(fail_len)?;
                        self.emit_byte(Op::Pop as u8); // Pop False
                        failure_jumps.push(self.emit_jump(Op::Jump as u8)); // Jump to outer failure
                        
                        self.patch_jump(success_jump)?;
                        
                        break; 
                    }
                    
                    // Element Match at `index`
                    
                    // 1. Check Len > index (i.e. >= index + 1)
                    self.emit_byte(Op::Dup as u8);
                    self.emit_byte(Op::Len as u8);
                    self.emit_constant(Constant::Int((index + 1) as i64));
                    // Gte (>=) is Not Lt (<)
                    self.emit_byte(Op::Lt as u8);
                    self.emit_byte(Op::Not as u8);
                    let fail_len = self.emit_jump(Op::JumpIfFalse as u8);
                    self.emit_byte(Op::Pop as u8); // Pop True
                    
                    // 2. Extract Item
                    self.emit_byte(Op::Dup as u8); // Dup List
                    self.emit_constant(Constant::Int(index as i64));
                    self.emit_byte(Op::GetIndex as u8); // Pushes Item
                    
                    // 3. Match Item
                    let sub_failures = self.compile_pattern()?;
                    
                    // 4. Handle Sub-Failures
                    // Stack: [List, Item]
                    
                    let success_jump = self.emit_jump(Op::Jump as u8);
                    
                    for jump in sub_failures {
                        self.patch_jump(jump)?;
                        self.emit_byte(Op::Pop as u8); // Pop Item
                        failure_jumps.push(self.emit_jump(Op::Jump as u8)); // Jump to outer failure (stack [List])
                    }
                    
                    self.patch_jump(fail_len)?; // Stack [List, False]
                    self.emit_byte(Op::Pop as u8); // Pop False
                    failure_jumps.push(self.emit_jump(Op::Jump as u8)); // Jump to outer failure (stack [List])
                    
                    self.patch_jump(success_jump)?;
                    
                    index += 1;
                    
                    if self.check(Token::RightBracket) {
                        break;
                    }
                    
                    if self.matches(Token::Comma)? {
                        continue;
                    }
                    
                    if self.check(Token::Pipe) {
                        continue;
                    }
                    
                    return Err(PulseError::CompileError("Expect ',' or ']' in list pattern.".into(), self.parser().line()));
                }
            }
            self.consume(Token::RightBracket, "Expect ']' after list pattern.")?;
            
            // Exact Match Check (if no tail)
            if !has_tail {
                // Check Len == index
                self.emit_byte(Op::Dup as u8);
                self.emit_byte(Op::Len as u8);
                self.emit_constant(Constant::Int(index as i64));
                self.emit_byte(Op::Eq as u8);
                let fail_exact = self.emit_jump(Op::JumpIfFalse as u8);
                self.emit_byte(Op::Pop as u8); // Pop True
                
                // Success path falls through
                let success_jump = self.emit_jump(Op::Jump as u8);
                
                self.patch_jump(fail_exact)?;
                self.emit_byte(Op::Pop as u8); // Pop False
                failure_jumps.push(self.emit_jump(Op::Jump as u8));
                
                self.patch_jump(success_jump)?;
            }
            
            // Finally: Success. Pop List.
            // Stack: [List].
            self.emit_byte(Op::Pop as u8);
            
            // Handle IsList Failure (Stack: [Target, False])
            let success_jump = self.emit_jump(Op::Jump as u8);
            
            self.patch_jump(fail_is_list)?;
            self.emit_byte(Op::Pop as u8); // Pop False
            failure_jumps.push(self.emit_jump(Op::Jump as u8)); // Stack [Target]
            
            self.patch_jump(success_jump)?;
            
        } else if self.matches(Token::LeftBrace)? {
            // Map Pattern: {key: pat, key2: pat}
            
            // 1. IsMap
            self.emit_byte(Op::Dup as u8);
            self.emit_byte(Op::IsMap as u8);
            let fail_is_map = self.emit_jump(Op::JumpIfFalse as u8);
            self.emit_byte(Op::Pop as u8); // Pop True
            
            while !self.check(Token::RightBrace) {
                // Parse Key
                let key_str = if let Token::Identifier(s) = &self.parser().current {
                    s.clone()
                } else if let Token::String(s) = &self.parser().current {
                    s.clone()
                } else {
                     return Err(PulseError::CompileError("Expect identifier or string key in map pattern.".into(), 0));
                };
                self.advance()?; // Consume Key
                
                self.consume(Token::Colon, "Expect ':' after map key.")?;
                
                // 1. Check Key Exists
                self.emit_byte(Op::Dup as u8); // Dup Map
                self.emit_constant(Constant::String(key_str.clone()));
                self.emit_byte(Op::MapContainsKey as u8); // Pops Key, Peeks Map -> Pushes Bool
                let fail_key = self.emit_jump(Op::JumpIfFalse as u8);
                self.emit_byte(Op::Pop as u8); // Pop True
                
                // 2. Extract Value
                self.emit_byte(Op::Dup as u8); // Dup Map
                self.emit_constant(Constant::String(key_str));
                self.emit_byte(Op::GetIndex as u8); // Pushes Value
                
                // 3. Match Value
                let sub_failures = self.compile_pattern()?;
                
                // 4. Handle Sub-Failures
                let success_jump = self.emit_jump(Op::Jump as u8);
                
                for jump in sub_failures {
                    self.patch_jump(jump)?;
                    self.emit_byte(Op::Pop as u8); // Pop Value
                    failure_jumps.push(self.emit_jump(Op::Jump as u8)); // Jump to outer failure
                }
                
                // Key Failure:
                self.patch_jump(fail_key)?; // Stack [Map, False]
                self.emit_byte(Op::Pop as u8); // Pop False
                failure_jumps.push(self.emit_jump(Op::Jump as u8)); // Jump to outer failure
                
                self.patch_jump(success_jump)?;
                
                if self.check(Token::RightBrace) {
                    break;
                }
                self.consume(Token::Comma, "Expect ',' or '}' in map pattern.")?;
            }
            self.consume(Token::RightBrace, "Expect '}' after map pattern.")?;
            
            // Success. Pop Map.
            self.emit_byte(Op::Pop as u8);
            
            // Handle IsMap Failure
            let success_jump = self.emit_jump(Op::Jump as u8);
            
            self.patch_jump(fail_is_map)?;
            self.emit_byte(Op::Pop as u8); // Pop False
            failure_jumps.push(self.emit_jump(Op::Jump as u8));
            
            self.patch_jump(success_jump)?;
        } else if let Token::Identifier(_) = self.parser().current {
            // Variable Pattern
            let name = self.parser().current.clone();
            self.advance()?;
            // Removed self.begin_scope(); to allow match arm to manage scope
            self.add_local(name)?;
            self.mark_initialized();
            
            // Variable consumes the value logically (binds it).
            // But checking `match_expression`, it expects [Subject] to remain on FAILURE?
            // On SUCCESS, `compile_pattern` consumes it?
            
            // Wait. `compile_pattern` contract:
            // Success: [] (Target consumed)
            // Failure (returns jumps): [Target] (Target preserved)
            
            // Variable match NEVER fails.
            // So we just need to consume the stack value?
            // `SetLocal` peaks.
            // But `add_local` just marks the slot.
            // The value is *already* on the stack (Subject).
            // We just mark it as the local.
            // So we do NOTHING at runtime!
            // Just compiler bookkeeping.
            
            // Stack: [Target]. Target is now Local 'x'.
            // Success!
            
            // Wait, we need to CONSUME the target from the "Expression Stack" view?
            // The contract says Success = [].
            // If we leave it on stack as local, it's "Consumed" from temp stack point of view
            // but physically there.
            // When scope ends, we Pop.
            // Correct.
        } else {
            // Literal
            self.emit_byte(Op::Dup as u8); // Dup Target
            if matches!(self.parser().current, Token::Int(_) | Token::Float(_)) {
                self.advance()?;
                self.number(false)?;
            } else if matches!(self.parser().current, Token::String(_)) {
                self.advance()?;
                self.string(false)?;
            } else if matches!(self.parser().current, Token::True | Token::False | Token::Nil) {
                self.advance()?;
                self.literal(false)?;
            } else {
                 return Err(PulseError::CompileError("Expect pattern.".into(), 0));
            }
             
            self.emit_byte(Op::Eq as u8);
            let fail = self.emit_jump(Op::JumpIfFalse as u8);
            self.emit_byte(Op::Pop as u8); // Pop True
            self.emit_byte(Op::Pop as u8); // Pop Target
            // Success: []
            
            // Fail Check:
            // [Target, False] -> Pop False -> [Target]. Correct.
            // But `fail_jumps` expect to land where we need to POP target?
            // No, `compile_pattern` returns jumps where Stack is [Target].
            
            // So for `fail`:
            // 1. Pop False. 
            // 2. Jump to exit.
            
            // We can't insert code at `fail` target easily without block structure.
            // Hand-code:
            // JumpIfFalse -> FailBlock
            // SuccessBlock: ...
            // FailBlock: Pop, Return Fail.
            
            // Since we return `fail` offset, the caller will patch it.
            // Caller patches `fail` -> `Next Arm`.
            // `Next Arm` expects `[Subject]`.
            
             // At `fail` (JumpIfFalse target): Stack is `[Target, False]`.
             // Caller expects `[Target]`.
             // We MUST Pop False.
             
             // So we patch `fail` to `pop_false_block`.
             // `pop_false_block`: Pop. Return.
             
             // We can emit this block at the end of `literal`?
             // Yes.
             let success_jump = self.emit_jump(Op::Jump as u8);
             
             // Fail Block
             self.patch_jump(fail)?;
             self.emit_byte(Op::Pop as u8); // Pop False
             failure_jumps.push(self.emit_jump(Op::Jump as u8));
             
             // Success Block
             self.patch_jump(success_jump)?;
        }
        
        Ok(failure_jumps)
    }
    
    fn mark_initialized(&mut self) {
        if self.scope_depth == 0 { return; }
        if let Some(local) = self.locals.last_mut() {
            local.depth = self.scope_depth;
        }
    }

    fn string(&mut self, _can_assign: bool) -> PulseResult<()> {
        let s = match &self.parser().previous {
            Token::String(s) => s.clone(),
            _ => return Err(PulseError::CompileError("Expected string".into(), 0)),
        };
        self.emit_constant(Constant::String(s));
        Ok(())
    }

    fn interpolated_string(&mut self, _can_assign: bool) -> PulseResult<()> {
        use crate::lexer::StringPart;
        
        let parts = match &self.parser().previous {
            Token::InterpolatedString(p) => p.clone(),
            _ => return Err(PulseError::CompileError("Expected interpolated string".into(), 0)),
        };
        
        if parts.is_empty() {
            self.emit_constant(Constant::String(String::new()));
            return Ok(());
        }
        
        // Compile first part
        let mut first = true;
        for part in parts {
            match part {
                StringPart::Literal(s) => {
                    self.emit_constant(Constant::String(s));
                }
                StringPart::Expr(expr_src) => {
                    // Parse and compile the expression
                    let mut expr_parser = crate::parser::Parser::new(&expr_src);
                    expr_parser.advance()?;
                    
                    // Save current parser
                    let saved_parser = self.parser as *mut crate::parser::Parser<'a>;
                    self.parser = &mut expr_parser as *mut crate::parser::Parser<'_> as *mut crate::parser::Parser<'a>;
                    
                    self.expression()?;
                    
                    // Restore parser
                    self.parser = saved_parser;
                    
                    // Convert to string with Op::ToString (we'll add this)
                    self.emit_byte(Op::ToString as u8);
                }
            }
            
            if first {
                first = false;
            } else {
                // Concatenate with previous
                self.emit_byte(Op::Add as u8);
            }
        }
        
        Ok(())
    }

    fn literal(&mut self, _can_assign: bool) -> PulseResult<()> {
        match self.parser().previous {
            Token::True => self.emit_constant(Constant::Bool(true)),
            Token::False => self.emit_constant(Constant::Bool(false)),
            Token::Nil => self.emit_byte(Op::Unit as u8),
            _ => return Err(PulseError::CompileError("Expected literal".into(), 0)),
        }
        Ok(())
    }

    fn variable(&mut self, can_assign: bool) -> PulseResult<()> {
        let previous = self.parser().previous.clone();
        if let Token::This = previous {
            // Handle 'this' keyword
            let slot = self.resolve_local(&Token::Identifier("this".to_string()))?;
            self.emit_byte(Op::GetLocal as u8);
            self.emit_byte(slot);
            Ok(())
        } else {
            self.named_variable(previous, can_assign)
        }
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) -> PulseResult<()> {
        if let Ok(local_idx) = self.resolve_local(&name) {
            if can_assign && self.matches(Token::Equal)? {
                self.expression()?;
                self.emit_byte(Op::SetLocal as u8);
                self.emit_byte(local_idx);
            } else {
                self.emit_byte(Op::GetLocal as u8);
                self.emit_byte(local_idx);
            }
        } else if let Some(upvalue_idx) = self.resolve_upvalue(&name) {
            if can_assign && self.matches(Token::Equal)? {
                self.expression()?;
                self.emit_byte(Op::SetUpvalue as u8);
                self.emit_byte(upvalue_idx);
            } else {
                self.emit_byte(Op::GetUpvalue as u8);
                self.emit_byte(upvalue_idx);
            }
        } else {
            // Global
            let global_idx = self.identifier_constant(&name)?;
            if can_assign && self.matches(Token::Equal)? {
                self.expression()?;
                self.emit_byte(Op::SetGlobal as u8);
                self.emit_u16(global_idx as u16);
            } else {
                self.emit_byte(Op::GetGlobal as u8);
                self.emit_u16(global_idx as u16);
            }
        }
        Ok(())
    }

    fn list_literal(&mut self, _can_assign: bool) -> PulseResult<()> {
        let mut item_count = 0;
        if !self.check(Token::RightBracket) {
            loop {
                if self.check(Token::RightBracket) { break; }
                self.expression()?;
                 if item_count == 255 {
                    return Err(PulseError::CompileError("Cannot have more than 255 items in a list literal.".into(), 0));
                }
                item_count += 1;
                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }
        self.consume(Token::RightBracket, "Expect ']' after list elements.")?;
        self.emit_byte(Op::BuildList as u8);
        self.emit_byte(item_count);
        Ok(())
    }

    fn map_literal(&mut self, _can_assign: bool) -> PulseResult<()> {
        let mut item_count = 0;
        if !self.check(Token::RightBrace) {
            loop {
                 if self.check(Token::RightBrace) { break; }
                // Parse key
                self.expression()?;
                self.consume(Token::Colon, "Expect ':' after map key.")?;
                // Parse value
                self.expression()?;
                
                if item_count == 255 {
                    return Err(PulseError::CompileError("Cannot have more than 255 entries in a map literal.".into(), 0));
                }
                item_count += 1;
                
                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }
        self.consume(Token::RightBrace, "Expect '}' after map entries.")?;
        self.emit_byte(Op::BuildMap as u8);
        self.emit_byte(item_count);
        Ok(())
    }

    fn subscript(&mut self, can_assign: bool) -> PulseResult<()> {
        // Called after consuming `[` (infix). Left operand is already compiled.
        self.expression()?; // Index
        self.consume(Token::RightBracket, "Expect ']' after index.")?;
        
        if can_assign && self.matches(Token::Equal)? {
            self.expression()?; // Value to set
            self.emit_byte(Op::SetIndex as u8);
        } else {
            self.emit_byte(Op::GetIndex as u8);
        }
        Ok(())
    }

    fn resolve_local(&mut self, name: &Token) -> PulseResult<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if let Token::Identifier(local_name) = &local.name {
                if let Token::Identifier(target) = name {
                    if local_name == target {
                        return Ok(i as u8);
                    }
                }
            }
        }
        Err(PulseError::CompileError("Undefined variable.".into(), 0))
    }

    fn resolve_upvalue(&mut self, name: &Token) -> Option<u8> {
        if self.enclosing.is_null() {
            return None;
        }

        let enclosing = unsafe { &mut *self.enclosing };

        // 1. Try to resolve in parent's locals
        if let Ok(local) = enclosing.resolve_local(name) {
            enclosing.locals[local as usize].is_captured = true;
            return Some(self.add_upvalue(local, true));
        }

        // 2. Try to resolve in parent's upvalues (recursive)
        if let Some(upvalue) = enclosing.resolve_upvalue(name) {
            return Some(self.add_upvalue(upvalue, false));
        }

        None
    }

    fn add_upvalue(&mut self, index: u8, is_local: bool) -> u8 {
        // Check if already captured
        for (i, upvalue) in self.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i as u8;
            }
        }

        self.upvalues.push(CompilerUpvalue { index, is_local });
        (self.upvalues.len() - 1) as u8
    }

    fn parse_variable(&mut self, msg: &str) -> PulseResult<u16> {
        self.consume_identifier(msg)?;
        self.declare_variable()?;
        if self.scope_depth > 0 {
            return Ok(0);
        }
        let name = self.parser().previous.clone();
        self.identifier_constant(&name) // Return name index for globals
    }

    fn identifier_constant(&mut self, name: &Token) -> PulseResult<u16> {
        match name {
            Token::Identifier(s) => {
                let idx = self.chunk.add_constant(Constant::String(s.clone()));
                if idx > u16::MAX as usize {
                    return Err(PulseError::CompileError("Too many constants.".into(), 0));
                }
                Ok(idx as u16)
            },
            _ => Err(PulseError::CompileError("Expected identifier.".into(), 0)),
        }
    }

    fn consume_identifier(&mut self, msg: &str) -> PulseResult<()> {
        // Helper to check if current is Identifier and advance
        match &self.parser().current {
            Token::Identifier(_) => {
                self.advance()?;
                Ok(())
            },
            _ => Err(PulseError::CompileError(msg.into(), 0)),
        }
    }

    fn consume_string(&mut self, msg: &str) -> PulseResult<()> {
        match &self.parser().current {
            Token::String(_) => {
                self.advance()?;
                Ok(())
            },
            _ => Err(PulseError::CompileError(msg.into(), 0)),
        }
    }

    fn declare_variable(&mut self) -> PulseResult<()> {
        if self.scope_depth == 0 {
            return Ok(()); // Globals not implemented yet
        }
        let name = self.parser().previous.clone();
        
        // Check for redefinition
        for local in self.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.scope_depth {
                break;
            }
            if local.name == name {
                 return Err(PulseError::CompileError("Variable with this name already declared in this scope.".into(), 0));
            }
        }

        self.add_local(name)?;
        Ok(())
    }

    fn define_variable(&mut self, global: u16) {
        if self.scope_depth > 0 {
            // Local: mark initialized
            if let Some(local) = self.locals.last_mut() {
                local.depth = self.scope_depth;
            }
        } else {
            // Global
            self.emit_byte(Op::DefineGlobal as u8);
            self.emit_u16(global);
        }
    }

    fn add_local(&mut self, name: Token) -> PulseResult<()> {
        if self.locals.len() >= 256 {
            return Err(PulseError::CompileError("Too many local variables in function.".into(), 0));
        }
        self.locals.push(Local { name, depth: -1, is_captured: false }); // -1 = uninitialized
        Ok(())
    }

    // Scoping
    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
        // Pop locals from stack
        while let Some(local) = self.locals.last() {
            if local.depth > self.scope_depth {
                if local.is_captured {
                    self.emit_byte(Op::CloseUpvalue as u8);
                } else {
                    self.emit_byte(Op::Pop as u8);
                }
                self.locals.pop();
            } else {
                break;
            }
        }
    }

    fn check(&self, token: Token) -> bool {
        unsafe { (*self.parser).current == token }
    }
    
    /// Parse a type annotation (e.g., Int, String, List<Int>, Fn<(Int) -> Bool>)
    fn parse_type(&mut self) -> PulseResult<crate::types::Type> {
        use crate::types::Type;
        
        self.parser().advance()?;
        
        match &self.parser().previous.clone() {
            Token::TypeInt => Ok(Type::Int),
            Token::TypeFloat => Ok(Type::Float),
            Token::TypeBool => Ok(Type::Bool),
            Token::TypeString => Ok(Type::String),
            Token::TypeUnit => Ok(Type::Unit),
            Token::TypePid => Ok(Type::Pid),
            Token::TypeAny => Ok(Type::Any),
            Token::TypeList => {
                // List<ElementType>
                if self.matches(Token::Less)? {
                    let elem_type = self.parse_type()?;
                    self.consume(Token::Greater, "Expect '>' after List element type.")?;
                    Ok(Type::List(Box::new(elem_type)))
                } else {
                    Ok(Type::List(Box::new(Type::Any)))
                }
            }
            Token::TypeMap => {
                // Map<KeyType, ValueType>
                if self.matches(Token::Less)? {
                    let key_type = self.parse_type()?;
                    self.consume(Token::Comma, "Expect ',' between Map key and value types.")?;
                    let val_type = self.parse_type()?;
                    self.consume(Token::Greater, "Expect '>' after Map value type.")?;
                    Ok(Type::Map(Box::new(key_type), Box::new(val_type)))
                } else {
                    Ok(Type::Map(Box::new(Type::Any), Box::new(Type::Any)))
                }
            }
            Token::TypeFn => {
                // Fn<(Param1, Param2) -> ReturnType>
                if self.matches(Token::Less)? {
                    self.consume(Token::LeftParen, "Expect '(' for Fn parameter types.")?;
                    let mut params = Vec::new();
                    if !self.check(Token::RightParen) {
                        loop {
                            params.push(self.parse_type()?);
                            if !self.matches(Token::Comma)? {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RightParen, "Expect ')' after Fn parameters.")?;
                    self.consume(Token::Arrow, "Expect '->' for Fn return type.")?;
                    let ret_type = self.parse_type()?;
                    self.consume(Token::Greater, "Expect '>' after Fn type.")?;
                    Ok(Type::Fn(params, Box::new(ret_type)))
                } else {
                    Ok(Type::Fn(vec![], Box::new(Type::Any)))
                }
            }
            Token::Identifier(name) => Ok(Type::Custom(name.clone())),
            other => Err(PulseError::CompileError(
                format!("Unexpected token in type annotation: {:?}", other),
                self.parser().line()
            )),
        }
    }

    fn binary(&mut self, _can_assign: bool) -> PulseResult<()> {
        let operator_type = self.parser().previous.clone();
        let rule = self.get_rule(&operator_type);
        
        // Parse right operand with higher precedence
        // e.g. 1 + 2 * 3 -> parse 2 * 3 first
        // Parse right operand with higher precedence
        let next_prec = rule.precedence.next();
        self.parse_precedence(next_prec)?; 
        // Pratt parsing:
        // Left associative: precedence + 1
        // Right associative: precedence

        match operator_type {
            Token::Plus => self.emit_byte(Op::Add as u8),
            Token::Minus => self.emit_byte(Op::Sub as u8),
            Token::Star => self.emit_byte(Op::Mul as u8),
            Token::Slash => self.emit_byte(Op::Div as u8),
            Token::BangEqual => self.emit_byte(Op::Neq as u8),
            Token::EqualEqual => self.emit_byte(Op::Eq as u8),
            Token::Greater => self.emit_byte(Op::Gt as u8),
            Token::GreaterEqual => {
                // a >= b  =>  not (a < b)
                self.emit_byte(Op::Lt as u8);
                self.emit_byte(Op::Not as u8);
            },
            Token::Less => self.emit_byte(Op::Lt as u8),
            Token::LessEqual => {
                // a <= b  =>  not (a > b)
                self.emit_byte(Op::Gt as u8);
                self.emit_byte(Op::Not as u8);
            },
            Token::Percent => self.emit_byte(Op::Mod as u8),
            Token::LogicalAnd => {
                // For logical operators, we need short-circuit evaluation
                // This is a simplified approach - ideally we'd implement proper short-circuiting
                // For now, emit as regular boolean AND
                self.emit_byte(Op::And as u8);  // Use existing AND operation
            },
            Token::LogicalOr => {
                // Similar for OR
                self.emit_byte(Op::Or as u8);  // Use existing OR operation
            },
            _ => return Err(PulseError::CompileError("Invalid binary operator".into(), 0)),
        }
        Ok(())
    }

    fn dot(&mut self, can_assign: bool) -> PulseResult<()> {
        self.consume_identifier("Expect property name after '.'.")?;
        let name = self.parser().previous.clone();
        let idx = self.identifier_constant(&name)?;
        
        self.emit_byte(Op::Const as u8);
        self.emit_u16(idx as u16);

        if can_assign && self.matches(Token::Equal)? {
            self.expression()?;
            self.emit_byte(Op::SetIndex as u8);
        } else {
            self.emit_byte(Op::GetIndex as u8);
        }
        Ok(())
    }

    fn and_(&mut self, _can_assign: bool) -> PulseResult<()> {
        let end_jump = self.emit_jump(Op::JumpIfFalse as u8);
        self.emit_byte(Op::Pop as u8);
        self.parse_precedence(Precedence::And)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    fn or_(&mut self, _can_assign: bool) -> PulseResult<()> {
        let else_jump = self.emit_jump(Op::JumpIfFalse as u8);
        let end_jump = self.emit_jump(Op::Jump as u8);

        self.patch_jump(else_jump)?;
        self.emit_byte(Op::Pop as u8);

        self.parse_precedence(Precedence::Or)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    fn spawn(&mut self, _can_assign: bool) -> PulseResult<()> {
        // spawn EXPRESSION
        
        // 1. Emit Spawn(0) placeholder
        let spawn_instr = self.chunk.code.len();
        self.emit_byte(Op::Spawn as u8);
        self.emit_byte(0xff); // Placeholder low
        self.emit_byte(0xff); // Placeholder high
        
        // 2. Emit Jump(0) placeholder (to jump over child code)
        let jump_over = self.emit_jump(Op::Jump as u8);
        
        // 3. Mark start of child code
        let child_start = self.chunk.code.len();
        
        // 4. Compile child expression/block
        if self.check(Token::LeftBrace) {
            self.consume(Token::LeftBrace, "Expect '{' after spawn.")?;
            self.block()?;
        } else {
            self.parse_precedence(Precedence::Assignment)?;
        }
        
        // 5. Child must HALT
        self.emit_byte(Op::Halt as u8);
        
        // 6. Patch Jump over
        self.patch_jump(jump_over)?;
        
        // 7. Patch Spawn argument (offset to child_start)
        // Note: Op::Spawn takes u16 offset from CURRENT IP? 
        // No, typically absolute or relative.
        // VM Implementation: Op::Spawn => { let offset = read_u16(); Ok(VMStatus::Spawn(offset)) }
        // Child IP set to `offset`.
        // So `offset` should be ABSOLUTE index in chunk.code?
        // Wait, `Op::Jump` usually uses relative offset.
        // Let's check `runtime.rs`:
        // child.vm.ip = offset;
        // So it sets absolute IP.
        
        // We write `child_start` (absolute index) into the Spawn instruction.
        if child_start > 0xffff {
             return Err(PulseError::CompileError("Chunk too large for spawn offset".into(), 0));
        }
        
        self.chunk.code[spawn_instr + 1] = (child_start & 0xff) as u8;
        self.chunk.code[spawn_instr + 2] = ((child_start >> 8) & 0xff) as u8;
        
        Ok(())
    }

    fn receive(&mut self, _can_assign: bool) -> PulseResult<()> {
        self.emit_byte(Op::Receive as u8);
        Ok(())
    }

    fn import_expression(&mut self, _can_assign: bool) -> PulseResult<()> {
        self.consume_string("Expect string after 'import'.")?;
        let name = self.parser().previous.clone();
        let idx = self.chunk.add_constant(Constant::String(match name {
            Token::String(s) => s,
            _ => unreachable!(),
        }));
        
        if idx > u16::MAX as usize {
            return Err(PulseError::CompileError("Too many constants.".into(), 0));
        }

        self.emit_byte(Op::Import as u8);
        self.emit_u16(idx as u16);
        Ok(())
    }

    fn anonymous_function(&mut self, _can_assign: bool) -> PulseResult<()> {
        // Parse anonymous function: fn(parameters) { body }
        let name = format!("lambda_{}", self.chunk.code.len()); // Generate unique name
        self.function(FunctionType::Function, name)?;
        Ok(())
    }

    fn super_(&mut self, _can_assign: bool) -> PulseResult<()> {
        if self.matches(Token::Dot)? {
            self.consume_identifier("Expect superclass method name.")?;
            let name = if let Token::Identifier(s) = &self.parser().previous { s.clone() } else { "".into() };
            let name_idx = self.identifier_constant(&Token::Identifier(name))?;

            // Push 'this'
            self.named_variable(Token::Identifier("this".to_string()), false)?;

            // Push 'super'
            self.named_variable(Token::Identifier("super".to_string()), false)?;

            self.emit_byte(Op::GetSuper as u8);
            self.emit_u16(name_idx as u16);
        } else {
            // Just 'super' access
             self.named_variable(Token::Identifier("super".to_string()), false)?;
        }
        Ok(())
    }

    fn register_expr(&mut self, _can_assign: bool) -> PulseResult<()> {
        // register(name, pid)
        self.consume(Token::LeftParen, "Expect '(' after 'register'.")?;
        self.expression()?; // name
        self.consume(Token::Comma, "Expect ',' after name.")?;
        self.expression()?; // pid
        self.consume(Token::RightParen, "Expect ')' after arguments.")?;
        self.emit_byte(Op::Register as u8);
        Ok(())
    }

    fn unregister_expr(&mut self, _can_assign: bool) -> PulseResult<()> {
        // unregister(name)
        self.consume(Token::LeftParen, "Expect '(' after 'unregister'.")?;
        self.expression()?; // name
        self.consume(Token::RightParen, "Expect ')' after arguments.")?;
        self.emit_byte(Op::Unregister as u8);
        Ok(())
    }

    fn whereis_expr(&mut self, _can_assign: bool) -> PulseResult<()> {
        // whereis(name)
        self.consume(Token::LeftParen, "Expect '(' after 'whereis'.")?;
        self.expression()?; // name
        self.consume(Token::RightParen, "Expect ')' after arguments.")?;
        self.emit_byte(Op::WhereIs as u8);
        Ok(())
    }

    fn link_expr(&mut self, _can_assign: bool) -> PulseResult<()> {
        // link(pid)
        self.consume(Token::LeftParen, "Expect '(' after 'link'.")?;
        self.expression()?; // pid
        self.consume(Token::RightParen, "Expect ')' after arguments.")?;
        self.emit_byte(Op::Link as u8);
        Ok(())
    }

    fn monitor_expr(&mut self, _can_assign: bool) -> PulseResult<()> {
        // monitor(pid)
        self.consume(Token::LeftParen, "Expect '(' after 'monitor'.")?;
        self.expression()?; // pid
        self.consume(Token::RightParen, "Expect ')' after arguments.")?;
        self.emit_byte(Op::Monitor as u8);
        Ok(())
    }

    fn call(&mut self, _can_assign: bool) -> PulseResult<()> {
        let mut arg_count = 0;
        if !self.check(Token::RightParen) {
            loop {
                self.expression()?;
                if arg_count == 255 {
                    return Err(PulseError::CompileError("Cannot have more than 255 arguments.".into(), 0));
                }
                arg_count += 1;
                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }
        self.consume(Token::RightParen, "Expect ')' after arguments.")?;
        
        self.emit_byte(Op::Call as u8);
        self.emit_byte(arg_count);
        Ok(())
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> PulseResult<()> {
        self.advance()?;
        
        let previous = self.parser().previous.clone();
        let prefix_rule = self.get_rule(&previous).prefix;
        if let Some(prefix_fn) = prefix_rule {
            let can_assign = precedence <= Precedence::Assignment;
            prefix_fn(self, can_assign)?;
        } else {
            println!("DEBUG: Expect expression failed on token: {:?}", previous);
            return Err(PulseError::CompileError("Expect expression.".into(), 0));
        }

        while {
            let current = self.parser().current.clone();
            precedence <= self.get_rule(&current).precedence
        } {
            self.advance()?;
            let previous = self.parser().previous.clone();
            let infix_rule = self.get_rule(&previous).infix;
            if let Some(infix_fn) = infix_rule {
                let can_assign = precedence <= Precedence::Assignment;
                infix_fn(self, can_assign)?;
            }
        }
        Ok(())
    }

    fn get_rule(&self, token: &Token) -> ParseRule<'a, 'b> {
        match token {
            Token::LeftParen => ParseRule { prefix: Some(Self::grouping), infix: Some(Self::call), precedence: Precedence::Call },
            Token::Dot => ParseRule { prefix: None, infix: Some(Self::dot), precedence: Precedence::Call },
            Token::Minus => ParseRule { prefix: Some(Self::unary), infix: Some(Self::binary), precedence: Precedence::Term },
            Token::Plus => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Term },
            Token::Slash => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Factor },
            Token::Star => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Factor },
            Token::BangEqual | Token::EqualEqual => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Equality },
            Token::Greater | Token::GreaterEqual | Token::Less | Token::LessEqual => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Comparison },
            Token::Int(_) | Token::Float(_) => ParseRule { prefix: Some(Self::number), infix: None, precedence: Precedence::None },
            Token::String(_) => ParseRule { prefix: Some(Self::string), infix: None, precedence: Precedence::None },
            Token::InterpolatedString(_) => ParseRule { prefix: Some(Self::interpolated_string), infix: None, precedence: Precedence::None },
            Token::True | Token::False | Token::Nil => ParseRule { prefix: Some(Self::literal), infix: None, precedence: Precedence::None },

            Token::LeftBrace => ParseRule { prefix: Some(Self::map_literal), infix: None, precedence: Precedence::None },
            Token::LeftBracket => ParseRule { prefix: Some(Self::list_literal), infix: Some(Self::subscript), precedence: Precedence::Call },
            Token::Identifier(_) => ParseRule { prefix: Some(Self::variable), infix: None, precedence: Precedence::None },
            Token::Spawn => ParseRule { prefix: Some(Self::spawn), infix: None, precedence: Precedence::None },
            Token::Link => ParseRule { prefix: Some(Self::link_expr), infix: None, precedence: Precedence::None },
            Token::Monitor => ParseRule { prefix: Some(Self::monitor_expr), infix: None, precedence: Precedence::None },
            Token::Receive => ParseRule { prefix: Some(Self::receive), infix: None, precedence: Precedence::None },
            Token::Register => ParseRule { prefix: Some(Self::register_expr), infix: None, precedence: Precedence::None },
            Token::Unregister => ParseRule { prefix: Some(Self::unregister_expr), infix: None, precedence: Precedence::None },
            Token::WhereIs => ParseRule { prefix: Some(Self::whereis_expr), infix: None, precedence: Precedence::None },
            Token::Match => ParseRule { prefix: Some(Self::match_expression), infix: None, precedence: Precedence::None },
            Token::Import => ParseRule { prefix: Some(Self::import_expression), infix: None, precedence: Precedence::None },
            Token::Bang => ParseRule { prefix: Some(Self::unary), infix: None, precedence: Precedence::Unary },
            Token::And => ParseRule { prefix: None, infix: Some(Self::and_), precedence: Precedence::And },
            Token::Or => ParseRule { prefix: None, infix: Some(Self::or_), precedence: Precedence::Or },
            Token::Percent => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Factor },
            Token::LogicalAnd => ParseRule { prefix: None, infix: Some(Self::and_), precedence: Precedence::And },
            Token::LogicalOr => ParseRule { prefix: None, infix: Some(Self::or_), precedence: Precedence::Or },
            Token::This => ParseRule { prefix: Some(Self::variable), infix: None, precedence: Precedence::None },
            Token::Super => ParseRule { prefix: Some(Self::super_), infix: None, precedence: Precedence::None },
            Token::Fn => ParseRule { prefix: Some(Self::anonymous_function), infix: None, precedence: Precedence::None },
            _ => ParseRule { prefix: None, infix: None, precedence: Precedence::None },
        }
    }

    // --- Helpers ---
    fn advance(&mut self) -> PulseResult<()> {
        self.parser().advance()
    }

    fn consume(&mut self, expected: Token, msg: &str) -> PulseResult<()> {
        self.parser().consume(expected, msg)
    }

    fn matches(&mut self, expected: Token) -> PulseResult<bool> {
        if self.parser().current == expected {
            self.parser().advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // --- Jump Helpers ---
    fn emit_jump(&mut self, instruction: u8) -> usize {
        self.emit_byte(instruction);
        self.emit_byte(0xff); // Placeholder
        self.emit_byte(0xff);
        self.chunk.code.len() - 2
    }

    fn patch_jump(&mut self, offset: usize) -> PulseResult<()> {
        let jump = self.chunk.code.len() - offset - 2;
        if jump > u16::MAX as usize {
             return Err(PulseError::CompileError("Too much code to jump over.".into(), 0));
        }
        self.chunk.code[offset] = (jump as u16 & 0xff) as u8;
        self.chunk.code[offset + 1] = ((jump as u16 >> 8) & 0xff) as u8;
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) -> PulseResult<()> {
        self.emit_byte(Op::Loop as u8);

        let offset = self.chunk.code.len() - loop_start + 2;
        if offset > u16::MAX as usize {
            return Err(PulseError::CompileError("Loop body too large.".into(), 0));
        }

        self.emit_byte((offset as u16 & 0xff) as u8);
        self.emit_byte(((offset as u16 >> 8) & 0xff) as u8);
        Ok(())
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.parser().previous_line;
        self.chunk.write(byte, line);
    }

    fn emit_u16(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.emit_byte(bytes[0]);
        self.emit_byte(bytes[1]);
    }

    fn emit_constant(&mut self, value: Constant) {
        let idx = self.chunk.add_constant(value);
        self.emit_byte(Op::Const as u8);
        self.emit_u16(idx as u16);
    }
}

