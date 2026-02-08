use crate::lexer::Token;
use pulse_core::{Chunk, Op, PulseError, PulseResult, Constant}; 
use pulse_core::object::Function;
use crate::parser::Parser;
use std::rc::Rc;

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
}

struct Loop {
    start_ip: usize,
    break_jumps: Vec<usize>,
}

#[derive(PartialEq, Clone, Copy)]
pub enum FunctionType {
    Script,
    Function,
}

pub struct Compiler<'a, 'b> {
    parser: &'b mut Parser<'a>,
    chunk: Chunk,
    locals: Vec<Local>,
    scope_depth: i32,
    loops: Vec<Loop>,
    function_type: FunctionType,
}

pub fn compile(source: &str) -> PulseResult<Chunk> {
    let mut parser = Parser::new(source);
    let mut compiler = Compiler::new(&mut parser, FunctionType::Script);
    compiler.compile_script()
}

impl<'a, 'b> Compiler<'a, 'b> {
    pub fn new(parser: &'b mut Parser<'a>, function_type: FunctionType) -> Self {
        let mut locals = Vec::new();
        // Reserve slot 0
        locals.push(Local {
            name: Token::Identifier("".to_string()),
            depth: 0,
        });

        Self {
            parser,
            chunk: Chunk::new(),
            locals,
            scope_depth: 0,
            loops: Vec::new(),
            function_type,
        }
    }

    pub fn compile_script(&mut self) -> PulseResult<Chunk> {
        self.parser.advance()?;
        
        while !self.matches(Token::Eof)? {
            self.declaration()?;
        }
        
        self.emit_byte(Op::Unit as u8);
        self.emit_byte(Op::Return as u8); 
        Ok(self.chunk.clone())
    }

    // --- Declarations ---
    // --- Declarations ---
    fn declaration(&mut self) -> PulseResult<()> {
        if self.matches(Token::Fn)? {
            self.fun_declaration()
        } else if self.matches(Token::Let)? {
            self.var_declaration()
        } else {
            self.statement()
        }
    }

    fn fun_declaration(&mut self) -> PulseResult<()> {
        let global = self.parse_variable("Expect function name.")?;
        let name = if let Token::Identifier(s) = &self.parser.previous { s.clone() } else { "".into() };
        self.function(FunctionType::Function, name)?;
        self.define_variable(global); 
        Ok(())
    }

    fn function(&mut self, function_type: FunctionType, name: String) -> PulseResult<()> {
        // Create new compiler for body
        let mut compiler = Compiler::new(self.parser, function_type);
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

                if !compiler.matches(Token::Comma)? {
                    break;
                }
            }
        }
        compiler.consume(Token::RightParen, "Expect ')' after parameters.")?;
        
        compiler.consume(Token::LeftBrace, "Expect '{' before function body.")?;
        compiler.block()?;
        
        // Emit return nil in case user didn't
        compiler.emit_return();
        
        let chunk = compiler.chunk.clone();
        let function = Function {
            arity,
            chunk: Rc::new(chunk),
            name,
        };
        
        // Add function to Parent (self) constants
        let idx = self.chunk.add_constant(Constant::Function(Box::new(function)));
        self.emit_byte(Op::Closure as u8);
        self.emit_byte(idx as u8);
        
        Ok(())
    }
    
    // Helper to emit return
    fn emit_return(&mut self) {
        self.emit_byte(Op::Unit as u8); // Default return
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
        } else if self.matches(Token::Return)? {
            self.return_statement()?;
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

    fn print_statement(&mut self) -> PulseResult<()> {
        self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after value.")?;
        self.emit_byte(Op::Print as u8);
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
        // for (init; cond; incr) body
        self.begin_scope();

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
        // Incr
        // let mut loop_vars_start = loop_start; // unused
        // Actually, continue should jump to increment start if present, else loop_start.
        // My implementation: 
        // Loops push `start_ip: loop_start`.
        // If we have increment, we change `loop_start` to `increment_start` BEFORE pushing loop context?
        // NO.
        // `for (init; cond; incr) { body }`
        // Structure:
        // [Init]
        // LoopStart:
        // [Cond] -> JumpIfFalse(Exit)
        // BodyJump -> Jump(Body)
        // IncrementStart:
        // [Incr]
        // Loop(LoopStart)
        // Body:
        // [Body]
        // Loop(IncrementStart)  <-- Continue should go here?
        // Patch BodyJump
        
        // My current implementation:
        // [Init]
        // LoopStart:
        // [Cond] -> JumpIfFalse(Exit)
        // BodyJump -> Jump(Body)
        // IncrementStart:
        // [Incr]
        // Loop(LoopStart)
        // Body:   <-- `loop_start` variable updated to `increment_start` in code
        // [Body]
        // Loop(IncrementStart)
        
        // When I push Loop context:
        // self.loops.push(Loop { start_ip: loop_start, ... });
        // `loop_start` IS `increment_start` if increment exists.
        // So `continue` jumps to `increment_start`. Correct.
        
        // `loop_vars_start` was unused.
        
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
            start_ip: loop_start, // Continue goes to increment (if existing) or start
            break_jumps: Vec::new(), // Breaks go to end
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
            self.expression()?;
            self.consume(Token::Semicolon, "Expect ';' after return value.")?;
            self.emit_byte(Op::Return as u8);
        }
        Ok(())
    }

    fn continue_statement(&mut self) -> PulseResult<()> {
        self.consume(Token::Semicolon, "Expect ';' after 'continue'.")?;
        
        let start_ip = if let Some(loop_ctx) = self.loops.last() {
            Some(loop_ctx.start_ip)
        } else {
            None
        };

        if let Some(ip) = start_ip {
             self.emit_loop(ip)?;
        } else {
            return Err(PulseError::CompileError("Cannot use 'continue' outside of a loop.".into(), 0));
        }
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
        match &self.parser.previous {
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
        let operator_type = self.parser.previous.clone();

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

    fn string(&mut self, _can_assign: bool) -> PulseResult<()> {
        let s = match &self.parser.previous {
            Token::String(s) => s.clone(),
            _ => return Err(PulseError::CompileError("Expected string".into(), 0)),
        };
        self.emit_constant(Constant::String(s));
        Ok(())
    }

    fn literal(&mut self, _can_assign: bool) -> PulseResult<()> {
        match self.parser.previous {
            Token::True => self.emit_constant(Constant::Bool(true)),
            Token::False => self.emit_constant(Constant::Bool(false)),
            Token::Nil => self.emit_byte(Op::Unit as u8),
            _ => return Err(PulseError::CompileError("Expected literal".into(), 0)),
        }
        Ok(())
    }

    fn variable(&mut self, can_assign: bool) -> PulseResult<()> {
        self.named_variable(self.parser.previous.clone(), can_assign)
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) -> PulseResult<()> {
        let arg = self.resolve_local(&name);
        
        if let Ok(local_idx) = arg {
            if can_assign && self.matches(Token::Equal)? {
                self.expression()?;
                self.emit_byte(Op::SetLocal as u8);
                self.emit_byte(local_idx);
            } else {
                self.emit_byte(Op::GetLocal as u8);
                self.emit_byte(local_idx);
            }
        } else {
            // Global
            let global_idx = self.identifier_constant(&name)?;
            if can_assign && self.matches(Token::Equal)? {
                self.expression()?;
                self.emit_byte(Op::SetGlobal as u8);
                self.emit_byte(global_idx);
            } else {
                self.emit_byte(Op::GetGlobal as u8);
                self.emit_byte(global_idx);
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

    fn parse_variable(&mut self, msg: &str) -> PulseResult<u8> {
        self.consume_identifier(msg)?;
        self.declare_variable()?;
        if self.scope_depth > 0 {
            return Ok(0);
        }
        let name = self.parser.previous.clone();
        self.identifier_constant(&name) // Return name index for globals
    }
    
    fn identifier_constant(&mut self, name: &Token) -> PulseResult<u8> {
        match name {
            Token::Identifier(s) => {
                let idx = self.chunk.add_constant(Constant::String(s.clone()));
                if idx > u8::MAX as usize {
                    return Err(PulseError::CompileError("Too many constants.".into(), 0));
                }
                Ok(idx as u8)
            },
            _ => Err(PulseError::CompileError("Expected identifier.".into(), 0)),
        }
    }

    fn consume_identifier(&mut self, msg: &str) -> PulseResult<()> {
        // Helper to check if current is Identifier and advance
        match &self.parser.current {
            Token::Identifier(_) => {
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
        let name = self.parser.previous.clone();
        
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

    fn define_variable(&mut self, global: u8) {
        if self.scope_depth > 0 {
            // Local: mark initialized
            if let Some(local) = self.locals.last_mut() {
                local.depth = self.scope_depth;
            }
        } else {
            // Global
            self.emit_byte(Op::DefineGlobal as u8);
            self.emit_byte(global);
        }
    }

    fn add_local(&mut self, name: Token) -> PulseResult<()> {
        if self.locals.len() >= 256 {
            return Err(PulseError::CompileError("Too many local variables in function.".into(), 0));
        }
        self.locals.push(Local { name, depth: -1 }); // -1 = uninitialized
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
                self.emit_byte(Op::Pop as u8);
                self.locals.pop();
            } else {
                break;
            }
        }
    }

    fn check(&self, token: Token) -> bool {
        self.parser.current == token
    }

    fn binary(&mut self, _can_assign: bool) -> PulseResult<()> {
        let operator_type = self.parser.previous.clone();
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
            _ => return Err(PulseError::CompileError("Invalid binary operator".into(), 0)),
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
        self.parse_precedence(Precedence::Assignment)?;
        
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
        
        let prefix_rule = self.get_rule(&self.parser.previous).prefix;
        if let Some(prefix_fn) = prefix_rule {
            let can_assign = precedence <= Precedence::Assignment;
            prefix_fn(self, can_assign)?;
        } else {
            return Err(PulseError::CompileError("Expect expression.".into(), 0));
        }

        while precedence <= self.get_rule(&self.parser.current).precedence {
            self.advance()?;
            let infix_rule = self.get_rule(&self.parser.previous).infix;
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
            Token::Minus => ParseRule { prefix: Some(Self::unary), infix: Some(Self::binary), precedence: Precedence::Term },
            Token::Plus => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Term },
            Token::Slash => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Factor },
            Token::Star => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Factor },
            Token::BangEqual | Token::EqualEqual => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Equality },
            Token::Greater | Token::GreaterEqual | Token::Less | Token::LessEqual => ParseRule { prefix: None, infix: Some(Self::binary), precedence: Precedence::Comparison },
            Token::Int(_) | Token::Float(_) => ParseRule { prefix: Some(Self::number), infix: None, precedence: Precedence::None },
            Token::String(_) => ParseRule { prefix: Some(Self::string), infix: None, precedence: Precedence::None },
            Token::True | Token::False | Token::Nil => ParseRule { prefix: Some(Self::literal), infix: None, precedence: Precedence::None },

            Token::LeftBrace => ParseRule { prefix: Some(Self::map_literal), infix: None, precedence: Precedence::None },
            Token::LeftBracket => ParseRule { prefix: Some(Self::list_literal), infix: Some(Self::subscript), precedence: Precedence::Call },
            Token::Identifier(_) => ParseRule { prefix: Some(Self::variable), infix: None, precedence: Precedence::None },
            Token::Spawn => ParseRule { prefix: Some(Self::spawn), infix: None, precedence: Precedence::None },
            Token::Receive => ParseRule { prefix: Some(Self::receive), infix: None, precedence: Precedence::None },
            Token::And => ParseRule { prefix: None, infix: Some(Self::and_), precedence: Precedence::And },
            Token::Or => ParseRule { prefix: None, infix: Some(Self::or_), precedence: Precedence::Or },
            _ => ParseRule { prefix: None, infix: None, precedence: Precedence::None },
        }
    }

    // --- Helpers ---
    fn advance(&mut self) -> PulseResult<()> {
        self.parser.advance()
    }

    fn consume(&mut self, expected: Token, msg: &str) -> PulseResult<()> {
        self.parser.consume(expected, msg)
    }

    fn matches(&mut self, expected: Token) -> PulseResult<bool> {
        if self.parser.current == expected {
            self.parser.advance()?;
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
        self.chunk.write(byte, self.parser.previous_line);
    }
    
    fn emit_constant(&mut self, value: Constant) {
        let idx = self.chunk.add_constant(value);
        self.emit_byte(Op::Const as u8);
        self.emit_byte(idx as u8);
    }
}

