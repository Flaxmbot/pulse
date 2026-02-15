//! Type checker for the Pulse compiler
//! 
//! Provides type inference and type checking for Pulse programs.
//! Supports gradual typing with optional type annotations.

use crate::ast::{Expr, Stmt, Decl, BinOp, UnOp, Script};
use crate::types::{Type, TypedParam};
use std::collections::HashMap;
use pulse_core::Constant;

/// Type checking errors
#[derive(Debug, Clone)]
pub enum TypeError {
    /// Variable not found in scope
    UndefinedVariable(String),
    /// Function not found
    UndefinedFunction(String),
    /// Type mismatch
    TypeMismatch {
        expected: Type,
        actual: Type,
        location: Location,
    },
    /// Operator not supported for types
    InvalidOperator {
        operator: String,
        left_type: Type,
        right_type: Option<Type>,
        location: Location,
    },
    /// Wrong number of arguments
    WrongArgumentCount {
        expected: usize,
        actual: usize,
        function: String,
        location: Location,
    },
    /// Invalid function call
    InvalidCall {
        callee_type: Type,
        location: Location,
    },
    /// Invalid return type
    InvalidReturnType {
        expected: Type,
        actual: Type,
        location: Location,
    },
    /// Cannot infer type
    CannotInferType(Location),
    /// Invalid member access
    InvalidMemberAccess {
        type_name: String,
        member: String,
        location: Location,
    },
    /// Class not found
    UndefinedClass(String),
    /// Property already exists
    PropertyExists {
        class: String,
        property: String,
        location: Location,
    },
    /// Actor must have receive block
    ActorWithoutReceive(Location),
    /// Invalid assignment target
    InvalidAssignmentTarget(Location),
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::UndefinedVariable(name) => {
                write!(f, "Undefined variable: {}", name)
            }
            TypeError::UndefinedFunction(name) => {
                write!(f, "Undefined function: {}", name)
            }
            TypeError::TypeMismatch { expected, actual, location } => {
                write!(f, "Type mismatch at {}: expected {}, got {}", 
                    location, expected, actual)
            }
            TypeError::InvalidOperator { operator, left_type, right_type, location } => {
                if let Some(rt) = right_type {
                    write!(f, "Invalid operator '{}' at {}: {} and {}", 
                        operator, location, left_type, rt)
                } else {
                    write!(f, "Invalid operator '{}' at {}: {}", 
                        operator, location, left_type)
                }
            }
            TypeError::WrongArgumentCount { expected, actual, function, location } => {
                write!(f, "Wrong number of arguments at {}: expected {} for function '{}', got {}", 
                    location, expected, function, actual)
            }
            TypeError::InvalidCall { callee_type, location } => {
                write!(f, "Cannot call {} at {}", callee_type, location)
            }
            TypeError::InvalidReturnType { expected, actual, location } => {
                write!(f, "Invalid return type at {}: expected {}, got {}", 
                    location, expected, actual)
            }
            TypeError::CannotInferType(location) => {
                write!(f, "Cannot infer type at {}", location)
            }
            TypeError::InvalidMemberAccess { type_name, member, location } => {
                write!(f, "Invalid member access at {}: type '{}' has no member '{}'", 
                    location, type_name, member)
            }
            TypeError::UndefinedClass(name) => {
                write!(f, "Undefined class: {}", name)
            }
            TypeError::PropertyExists { class, property, location } => {
                write!(f, "Property '{}' already exists in class '{}' at {}", 
                    property, class, location)
            }
            TypeError::ActorWithoutReceive(location) => {
                write!(f, "Actor must have a receive block at {}", location)
            }
            TypeError::InvalidAssignmentTarget(location) => {
                write!(f, "Invalid assignment target at {}", location)
            }
        }
    }
}

/// Source code location for error reporting
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Type context for tracking variable and function types
pub struct TypeContext {
    /// Current scope variables: name -> type
    variables: Vec<HashMap<String, Type>>,
    /// Global functions: name -> (param_types, return_type)
    functions: HashMap<String, (Vec<Type>, Type)>,
    /// Class definitions: name -> (properties, methods)
    classes: HashMap<String, ClassInfo>,
    /// Current return type for type checking
    return_type: Option<Type>,
    /// Line counter for location tracking
    current_line: usize,
    /// Column counter for location tracking
    current_column: usize,
}

/// Information about a class
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub properties: HashMap<String, Type>,
    pub methods: HashMap<String, (Vec<Type>, Type)>,
}

impl TypeContext {
    /// Create a new type context with built-in functions
    pub fn new() -> Self {
        let mut context = TypeContext {
            variables: vec![HashMap::new()],
            functions: HashMap::new(),
            classes: HashMap::new(),
            return_type: None,
            current_line: 1,
            current_column: 1,
        };
        
        // Add built-in functions
        context.add_builtin_functions();
        
        context
    }

    /// Add built-in functions to the context
    fn add_builtin_functions(&mut self) {
        // Print functions
        self.functions.insert("print".to_string(), (vec![Type::Any], Type::Unit));
        self.functions.insert("println".to_string(), (vec![Type::Any], Type::Unit));
        
        // String functions
        self.functions.insert("len".to_string(), (vec![Type::Any], Type::Int));
        self.functions.insert("str".to_string(), (vec![Type::Any], Type::String));
        
        // List functions
        self.functions.insert("push".to_string(), (vec![Type::List(Box::new(Type::Any)), Type::Any], Type::Unit));
        self.functions.insert("pop".to_string(), (vec![Type::List(Box::new(Type::Any))], Type::Any));
        
        // Math functions
        self.functions.insert("abs".to_string(), (vec![Type::Int], Type::Int));
        self.functions.insert("floor".to_string(), (vec![Type::Float], Type::Float));
        self.functions.insert("ceil".to_string(), (vec![Type::Float], Type::Float));
        self.functions.insert("sqrt".to_string(), (vec![Type::Float], Type::Float));
        self.functions.insert("sin".to_string(), (vec![Type::Float], Type::Float));
        self.functions.insert("cos".to_string(), (vec![Type::Float], Type::Float));
        
        // Actor functions
        self.functions.insert("spawn".to_string(), (vec![Type::Fn(vec![], Box::new(Type::Unit))], Type::Pid));
        self.functions.insert("self".to_string(), (vec![], Type::Pid));
        self.functions.insert("send".to_string(), (vec![Type::Pid, Type::Any], Type::Unit));
        self.functions.insert("link".to_string(), (vec![Type::Pid], Type::Unit));
        self.functions.insert("monitor".to_string(), (vec![Type::Pid], Type::Unit));
    }

    /// Get current location
    fn location(&self) -> Location {
        Location {
            line: self.current_line,
            column: self.current_column,
        }
    }

    /// Advance line counter
    #[allow(dead_code)]
    fn advance_line(&mut self) {
        self.current_line += 1;
        self.current_column = 1;
    }

    /// Advance column counter
    #[allow(dead_code)]
    fn advance_column(&mut self) {
        self.current_column += 1;
    }

    /// Enter a new scope
    fn enter_scope(&mut self) {
        self.variables.push(HashMap::new());
    }

    /// Exit the current scope
    fn exit_scope(&mut self) {
        self.variables.pop();
    }

    /// Define a variable in the current scope
    fn define_variable(&mut self, name: String, var_type: Type) {
        if let Some(scope) = self.variables.last_mut() {
            scope.insert(name, var_type);
        }
    }

    /// Look up a variable in all scopes
    fn lookup_variable(&self, name: &str) -> Option<Type> {
        for scope in self.variables.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    /// Define a function
    fn define_function(&mut self, name: String, params: Vec<Type>, return_type: Type) {
        self.functions.insert(name, (params, return_type));
    }

    /// Look up a function
    fn lookup_function(&self, name: &str) -> Option<(Vec<Type>, Type)> {
        self.functions.get(name).cloned()
    }

    /// Define a class
    fn define_class(&mut self, name: String, info: ClassInfo) {
        self.classes.insert(name, info);
    }

    /// Look up a class
    fn lookup_class(&self, name: &str) -> Option<ClassInfo> {
        self.classes.get(name).cloned()
    }

    /// Set the current return type
    fn set_return_type(&mut self, return_type: Option<Type>) {
        self.return_type = return_type;
    }

    /// Get the current return type
    #[allow(dead_code)]
    fn get_return_type(&self) -> Option<Type> {
        self.return_type.clone()
    }
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker result
pub type TypeCheckResult<T> = Result<T, TypeError>;

/// Type checker for Pulse programs
pub struct TypeChecker {
    context: TypeContext,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        TypeChecker {
            context: TypeContext::new(),
            errors: Vec::new(),
        }
    }

    /// Check a script and return errors
    pub fn check_script(&mut self, script: &Script) -> Vec<TypeError> {
        // First pass: collect all declarations
        self.collect_declarations(&script.declarations);
        
        // Second pass: check types
        for decl in &script.declarations {
            if let Err(e) = self.check_decl(decl) {
                self.errors.push(e);
            }
        }
        
        std::mem::take(&mut self.errors)
    }

    /// Collect all declarations for type checking
    fn collect_declarations(&mut self, decls: &[Decl]) {
        for decl in decls {
            match decl {
                Decl::Function(name, params, return_type, _) => {
                    let param_types: Vec<Type> = params.iter()
                        .map(|p| p.type_annotation.clone().unwrap_or(Type::Any))
                        .collect();
                    let ret_type = return_type.clone().unwrap_or(Type::Any);
                    self.context.define_function(name.clone(), param_types, ret_type);
                }
                Decl::Class(name, _, class_decls) => {
                    let mut info = ClassInfo {
                        name: name.clone(),
                        properties: HashMap::new(),
                        methods: HashMap::new(),
                    };
                    
                    for class_decl in class_decls {
                        if let Decl::Function(method_name, params, return_type, _) = class_decl {
                            let param_types: Vec<Type> = params.iter()
                                .map(|p| p.type_annotation.clone().unwrap_or(Type::Any))
                                .collect();
                            let ret_type = return_type.clone().unwrap_or(Type::Any);
                            info.methods.insert(method_name.clone(), (param_types, ret_type));
                        }
                    }
                    
                    self.context.define_class(name.clone(), info);
                }
                _ => {}
            }
        }
    }

    /// Check a declaration
    fn check_decl(&mut self, decl: &Decl) -> TypeCheckResult<()> {
        match decl {
            Decl::Function(name, params, return_type, body) => {
                self.check_function(name, params, return_type, body)
            }
            Decl::Class(name, parent, class_decls) => {
                self.check_class(name, parent, class_decls)
            }
            Decl::Actor(name, body) => {
                self.check_actor(name, body)
            }
            Decl::Stmt(stmt) => {
                self.check_stmt(stmt)
            }
            _ => Ok(()) // Skip other declarations for now
        }
    }

    /// Check a function definition
    fn check_function(
        &mut self,
        _name: &str,
        params: &[TypedParam],
        return_type: &Option<Type>,
        body: &[Stmt],
    ) -> TypeCheckResult<()> {
        // Set return type for the function
        let ret_type = return_type.clone().unwrap_or(Type::Any);
        self.context.set_return_type(Some(ret_type.clone()));
        
        // Enter new scope for function parameters
        self.context.enter_scope();
        
        // Define parameters
        for param in params {
            let param_type = param.type_annotation.clone().unwrap_or(Type::Any);
            self.context.define_variable(param.name.clone(), param_type);
        }
        
        // Check function body
        for stmt in body {
            self.check_stmt(stmt)?;
        }
        
        // Exit function scope
        self.context.exit_scope();
        self.context.set_return_type(None);
        
        Ok(())
    }

    /// Check a class definition
    fn check_class(
        &mut self,
        _name: &str,
        parent: &Option<String>,
        class_decls: &[Decl],
    ) -> TypeCheckResult<()> {
        // Check parent exists if specified
        if let Some(parent_name) = parent {
            self.context.lookup_class(parent_name)
                .ok_or_else(|| TypeError::UndefinedClass(parent_name.clone()))?;
        }
        
        // Check all methods
        for class_decl in class_decls {
            if let Decl::Function(method_name, params, return_type, body) = class_decl {
                self.check_function(method_name, params, return_type, body)?;
            }
        }
        
        Ok(())
    }

    /// Check an actor definition
    fn check_actor(&mut self, _name: &str, body: &[Stmt]) -> TypeCheckResult<()> {
        // Enter new scope for actor
        self.context.enter_scope();
        
        // Check body
        for stmt in body {
            self.check_stmt(stmt)?;
        }
        
        // Exit actor scope
        self.context.exit_scope();
        
        Ok(())
    }

    /// Check a statement
    fn check_stmt(&mut self, stmt: &Stmt) -> TypeCheckResult<()> {
        match stmt {
            Stmt::Expression(expr) => {
                self.check_expr(expr)?;
            }
            Stmt::Print(expr) => {
                self.check_expr(expr)?;
            }
            Stmt::Let(name, type_annotation, init_expr) => {
                self.check_let(name, type_annotation.as_ref(), init_expr.as_ref())?;
            }
            Stmt::If(cond, then_branch, else_branch) => {
                self.check_if(cond, then_branch, else_branch.as_ref().map(|b| &**b))?;
            }
            Stmt::While(cond, body) => {
                self.check_while(cond, body)?;
            }
            Stmt::For(init, cond, update, body) => {
                self.check_for(
                    init.as_ref().map(|s| s.as_ref()), 
                    cond.as_ref(), 
                    update.as_ref(), 
                    body,
                )?;
            }
            Stmt::Return(expr) => {
                self.check_return(expr.as_ref())?;
            }
            Stmt::Break | Stmt::Continue => {
                // Control flow - no type checking needed
            }
            Stmt::Block(stmts) => {
                self.check_block(stmts)?;
            }
            Stmt::Try(body, catch_var, catch_body) => {
                self.check_try(body, catch_var, catch_body)?;
            }
            Stmt::Throw(expr) => {
                self.check_expr(expr)?;
            }
            Stmt::Send(target, message) => {
                let target_type = self.check_expr(target)?;
                if !target_type.is_compatible(&Type::Pid) {
                    return Err(TypeError::TypeMismatch {
                        expected: Type::Pid,
                        actual: target_type,
                        location: self.context.location(),
                    });
                }
                self.check_expr(message)?;
            }
            Stmt::Link(expr) => {
                let expr_type = self.check_expr(expr)?;
                if !expr_type.is_compatible(&Type::Pid) {
                    return Err(TypeError::TypeMismatch {
                        expected: Type::Pid,
                        actual: expr_type,
                        location: self.context.location(),
                    });
                }
            }
            Stmt::Monitor(expr) => {
                let expr_type = self.check_expr(expr)?;
                if !expr_type.is_compatible(&Type::Pid) {
                    return Err(TypeError::TypeMismatch {
                        expected: Type::Pid,
                        actual: expr_type,
                        location: self.context.location(),
                    });
                }
            }
            Stmt::Spawn(expr) => {
                self.check_expr(expr)?;
            }
        }
        
        Ok(())
    }

    /// Check a let statement
    fn check_let(
        &mut self,
        name: &str,
        type_annotation: Option<&Type>,
        init_expr: Option<&Expr>,
    ) -> TypeCheckResult<()> {
        // Get the type from initialization or annotation
        let inferred_type = if let Some(expr) = init_expr {
            self.check_expr(expr)?
        } else {
            type_annotation.cloned().ok_or_else(|| 
                TypeError::CannotInferType(self.context.location()))?
        };
        
        // If there's a type annotation, check compatibility
        if let Some(ann_type) = type_annotation {
            if !inferred_type.is_compatible(ann_type) {
                return Err(TypeError::TypeMismatch {
                    expected: ann_type.clone(),
                    actual: inferred_type,
                    location: self.context.location(),
                });
            }
            self.context.define_variable(name.to_string(), ann_type.clone());
        } else {
            self.context.define_variable(name.to_string(), inferred_type);
        }
        
        Ok(())
    }

    /// Check an if statement
    fn check_if(
        &mut self,
        cond: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) -> TypeCheckResult<()> {
        // Check condition is boolean
        let cond_type = self.check_expr(cond)?;
        if !cond_type.is_compatible(&Type::Bool) {
            return Err(TypeError::TypeMismatch {
                expected: Type::Bool,
                actual: cond_type,
                location: self.context.location(),
            });
        }
        
        // Check branches
        self.check_stmt(then_branch)?;
        if let Some(else_br) = else_branch {
            self.check_stmt(else_br)?;
        }
        
        Ok(())
    }

    /// Check a while statement
    fn check_while(&mut self, cond: &Expr, body: &Stmt) -> TypeCheckResult<()> {
        // Check condition is boolean
        let cond_type = self.check_expr(cond)?;
        if !cond_type.is_compatible(&Type::Bool) {
            return Err(TypeError::TypeMismatch {
                expected: Type::Bool,
                actual: cond_type,
                location: self.context.location(),
            });
        }
        
        self.check_stmt(body)
    }

    /// Check a for statement
    fn check_for(
        &mut self,
        init: Option<&Stmt>,
        cond: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
    ) -> TypeCheckResult<()> {
        if let Some(i) = init {
            self.check_stmt(i)?;
        }
        
        if let Some(c) = cond {
            let cond_type = self.check_expr(c)?;
            if !cond_type.is_compatible(&Type::Bool) {
                return Err(TypeError::TypeMismatch {
                    expected: Type::Bool,
                    actual: cond_type,
                    location: self.context.location(),
                });
            }
        }
        
        if let Some(u) = update {
            self.check_expr(u)?;
        }
        
        self.check_stmt(body)
    }

    /// Check a return statement
    fn check_return(&mut self, expr: Option<&Expr>) -> TypeCheckResult<()> {
        let actual_type = if let Some(e) = expr {
            self.check_expr(e)?
        } else {
            Type::Unit
        };
        
        if let Some(expected) = &self.context.return_type {
            if !actual_type.is_compatible(expected) {
                return Err(TypeError::InvalidReturnType {
                    expected: expected.clone(),
                    actual: actual_type,
                    location: self.context.location(),
                });
            }
        }
        
        Ok(())
    }

    /// Check a block statement
    fn check_block(&mut self, stmts: &[Stmt]) -> TypeCheckResult<()> {
        self.context.enter_scope();
        
        for stmt in stmts {
            self.check_stmt(stmt)?;
        }
        
        self.context.exit_scope();
        
        Ok(())
    }

    /// Check a try-catch statement
    fn check_try(
        &mut self,
        body: &Stmt,
        catch_var: &str,
        catch_body: &Stmt,
    ) -> TypeCheckResult<()> {
        self.check_stmt(body)?;
        
        self.context.enter_scope();
        self.context.define_variable(catch_var.to_string(), Type::String);
        self.check_stmt(catch_body)?;
        self.context.exit_scope();
        
        Ok(())
    }

    /// Check an expression and return its type
    fn check_expr(&mut self, expr: &Expr) -> TypeCheckResult<Type> {
        match expr {
            Expr::Literal(constant) => Ok(self.check_literal(constant)),
            Expr::Variable(name) => self.check_variable(name),
            Expr::Binary(left, op, right) => self.check_binary(left, op, right),
            Expr::Unary(op, operand) => self.check_unary(op, operand),
            Expr::Call(callee, args) => self.check_call(callee, args),
            Expr::Get(object, member) => self.check_get(object, member),
            Expr::Set(object, _member, value) => self.check_set(object, value),
            Expr::Index(object, index) => self.check_index(object, index),
            Expr::This => Ok(Type::Any), // Could be more specific in class context
            Expr::Super(_method) => Ok(Type::Any),
            Expr::List(elements) => self.check_list(elements),
            Expr::Map(entries) => self.check_map(entries),
            Expr::Closure(name, params, return_type, body) => {
                self.check_closure(name, params, return_type, body)
            }
        }
    }

    /// Check a literal and return its type
    fn check_literal(&self, constant: &Constant) -> Type {
        match constant {
            Constant::Bool(_) => Type::Bool,
            Constant::Int(_) => Type::Int,
            Constant::Float(_) => Type::Float,
            Constant::String(_) => Type::String,
            Constant::Unit => Type::Unit,
            Constant::Function(_) => Type::Fn(vec![], Box::new(Type::Any)),
            Constant::SharedMemory(_) => Type::Any,
            Constant::Socket(_) => Type::Custom("Socket".to_string()),
            Constant::Listener(_) => Type::Custom("Listener".to_string()),
        }
    }

    /// Check a variable reference
    fn check_variable(&self, name: &str) -> TypeCheckResult<Type> {
        self.context.lookup_variable(name)
            .ok_or_else(|| TypeError::UndefinedVariable(name.to_string()))
    }

    /// Check a binary operation
    fn check_binary(&mut self, left: &Expr, op: &BinOp, right: &Expr) -> TypeCheckResult<Type> {
        let left_type = self.check_expr(left)?;
        let right_type = self.check_expr(right)?;
        
        match op {
            // Arithmetic operations
            BinOp::Add => self.check_arithmetic_op(&left_type, &right_type),
            BinOp::Sub => self.check_arithmetic_op(&left_type, &right_type),
            BinOp::Mul => self.check_arithmetic_op(&left_type, &right_type),
            BinOp::Div => self.check_arithmetic_op(&left_type, &right_type),
            
            // Comparison operations
            BinOp::Eq | BinOp::Ne => {
                // Equality works for most types
                Ok(Type::Bool)
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.check_comparison_op(&left_type, &right_type)
            }
            
            // Logical operations
            BinOp::And | BinOp::Or => {
                if !left_type.is_compatible(&Type::Bool) {
                    return Err(TypeError::TypeMismatch {
                        expected: Type::Bool,
                        actual: left_type,
                        location: self.context.location(),
                    });
                }
                if !right_type.is_compatible(&Type::Bool) {
                    return Err(TypeError::TypeMismatch {
                        expected: Type::Bool,
                        actual: right_type,
                        location: self.context.location(),
                    });
                }
                Ok(Type::Bool)
            }
        }
    }

    /// Check an arithmetic operation
    fn check_arithmetic_op(&self, left: &Type, right: &Type) -> TypeCheckResult<Type> {
        match (left, right) {
            // Int + Int -> Int
            (Type::Int, Type::Int) => Ok(Type::Int),
            // Float + Float -> Float
            (Type::Float, Type::Float) => Ok(Type::Float),
            // Int + Float -> Float
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(Type::Float),
            // String + String -> String
            (Type::String, Type::String) => Ok(Type::String),
            // List + List -> List
            (Type::List(a), Type::List(b)) if a == b => Ok(Type::List(Box::new(*a.clone()))),
            // Any allows anything
            (Type::Any, _) | (_, Type::Any) => Ok(Type::Any),
            // Error for other combinations
            _ => Err(TypeError::InvalidOperator {
                operator: "+".to_string(),
                left_type: left.clone(),
                right_type: Some(right.clone()),
                location: self.context.location(),
            }),
        }
    }

    /// Check a comparison operation
    fn check_comparison_op(&self, left: &Type, right: &Type) -> TypeCheckResult<Type> {
        match (left, right) {
            (Type::Int, Type::Int) => Ok(Type::Bool),
            (Type::Float, Type::Float) => Ok(Type::Bool),
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(Type::Bool),
            (Type::String, Type::String) => Ok(Type::Bool),
            (Type::Bool, Type::Bool) => Ok(Type::Bool),
            (Type::Any, _) | (_, Type::Any) => Ok(Type::Bool),
            _ => Err(TypeError::InvalidOperator {
                operator: "<".to_string(),
                left_type: left.clone(),
                right_type: Some(right.clone()),
                location: self.context.location(),
            }),
        }
    }

    /// Check a unary operation
    fn check_unary(&mut self, op: &UnOp, operand: &Expr) -> TypeCheckResult<Type> {
        let operand_type = self.check_expr(operand)?;
        
        match op {
            UnOp::Neg => {
                match operand_type {
                    Type::Int => Ok(Type::Int),
                    Type::Float => Ok(Type::Float),
                    Type::Any => Ok(Type::Any),
                    _ => Err(TypeError::InvalidOperator {
                        operator: "-".to_string(),
                        left_type: operand_type,
                        right_type: None,
                        location: self.context.location(),
                    }),
                }
            }
            UnOp::Not => {
                // ! works on booleans and returns boolean
                if operand_type.is_compatible(&Type::Bool) {
                    Ok(Type::Bool)
                } else if matches!(operand_type, Type::Any) {
                    Ok(Type::Bool)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: Type::Bool,
                        actual: operand_type,
                        location: self.context.location(),
                    })
                }
            }
        }
    }

    /// Check a function call
    fn check_call(&mut self, callee: &Expr, args: &[Expr]) -> TypeCheckResult<Type> {
        let callee_type = self.check_callable(callee)?;
        
        match callee_type {
            Type::Fn(params, return_type) => {
                // Check argument count
                if params.len() != args.len() {
                    return Err(TypeError::WrongArgumentCount {
                        expected: params.len(),
                        actual: args.len(),
                        function: "function".to_string(),
                        location: self.context.location(),
                    });
                }
                
                // Check argument types
                for (param_type, arg) in params.iter().zip(args.iter()) {
                    let arg_type = self.check_expr(arg)?;
                    if !arg_type.is_compatible(param_type) {
                        return Err(TypeError::TypeMismatch {
                            expected: param_type.clone(),
                            actual: arg_type,
                            location: self.context.location(),
                        });
                    }
                }
                
                Ok(*return_type)
            }
            Type::Any => Ok(Type::Any),
            _ => Err(TypeError::InvalidCall {
                callee_type: callee_type,
                location: self.context.location(),
            }),
        }
    }

    /// Check if an expression can be used as a callable (function or variable reference)
    fn check_callable(&mut self, expr: &Expr) -> TypeCheckResult<Type> {
        match expr {
            Expr::Variable(name) => {
                // First check if it's a function
                if let Some((params, return_type)) = self.context.lookup_function(name) {
                    return Ok(Type::Fn(params, Box::new(return_type)));
                }
                // Then check if it's a variable
                self.context.lookup_variable(name)
                    .ok_or_else(|| TypeError::UndefinedVariable(name.clone()))
            }
            _ => self.check_expr(expr)
        }
    }

    /// Check property access (e.g., obj.property)
    fn check_get(&mut self, object: &Expr, member: &str) -> TypeCheckResult<Type> {
        let object_type = self.check_expr(object)?;
        
        match &object_type {
            Type::Custom(class_name) => {
                if let Some(class_info) = self.context.lookup_class(class_name) {
                    if let Some(method_type) = class_info.methods.get(member) {
                        return Ok(Type::Fn(method_type.0.clone(), Box::new(method_type.1.clone())));
                    }
                    if let Some(prop_type) = class_info.properties.get(member) {
                        return Ok(prop_type.clone());
                    }
                }
                Err(TypeError::InvalidMemberAccess {
                    type_name: class_name.clone(),
                    member: member.to_string(),
                    location: self.context.location(),
                })
            }
            Type::Map(_, _) => {
                // Accessing a map - currently not supported
                Err(TypeError::InvalidMemberAccess {
                    type_name: "Map".to_string(),
                    member: member.to_string(),
                    location: self.context.location(),
                })
            }
            Type::Any => Ok(Type::Any),
            _ => Err(TypeError::InvalidMemberAccess {
                type_name: object_type.to_string(),
                member: member.to_string(),
                location: self.context.location(),
            }),
        }
    }

    /// Check property assignment (e.g., obj.property = value)
    fn check_set(&mut self, object: &Expr, value: &Expr) -> TypeCheckResult<Type> {
        let _object_type = self.check_expr(object)?;
        let value_type = self.check_expr(value)?;
        
        // For now, just return the value type
        Ok(value_type)
    }

    /// Check index access (e.g., arr[0])
    fn check_index(&mut self, object: &Expr, index: &Expr) -> TypeCheckResult<Type> {
        let object_type = self.check_expr(object)?;
        let _index_type = self.check_expr(index)?;
        
        match object_type {
            Type::List(elem_type) => Ok(*elem_type),
            Type::Map(_, value_type) => Ok(*value_type),
            Type::String => Ok(Type::String),
            Type::Any => Ok(Type::Any),
            _ => Err(TypeError::InvalidOperator {
                operator: "[]".to_string(),
                left_type: object_type,
                right_type: None,
                location: self.context.location(),
            }),
        }
    }

    /// Check a list literal
    fn check_list(&self, elements: &[Expr]) -> TypeCheckResult<Type> {
        if elements.is_empty() {
            return Ok(Type::List(Box::new(Type::Any)));
        }
        
        // For now, just return a list of Any
        Ok(Type::List(Box::new(Type::Any)))
    }

    /// Check a map literal
    fn check_map(&self, entries: &[(Expr, Expr)]) -> TypeCheckResult<Type> {
        if entries.is_empty() {
            return Ok(Type::Map(Box::new(Type::Any), Box::new(Type::Any)));
        }
        
        // For now, just return a map of Any to Any
        Ok(Type::Map(Box::new(Type::Any), Box::new(Type::Any)))
    }

    /// Check a closure
    fn check_closure(
        &mut self,
        name: &str,
        params: &[TypedParam],
        return_type: &Option<Type>,
        body: &[Stmt],
    ) -> TypeCheckResult<Type> {
        self.check_function(name, params, return_type, body)?;
        
        let param_types: Vec<Type> = params.iter()
            .map(|p| p.type_annotation.clone().unwrap_or(Type::Any))
            .collect();
        let ret_type = return_type.clone().unwrap_or(Type::Any);
        
        Ok(Type::Fn(param_types, Box::new(ret_type)))
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Main type checking function
pub fn check_types(script: &Script) -> Vec<TypeError> {
    let mut checker = TypeChecker::new();
    checker.check_script(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TypedParam;
    use pulse_core::Constant;

    /// Helper to create a simple script from declarations
    fn make_script(decls: Vec<Decl>) -> Script {
        Script { declarations: decls }
    }

    /// Helper to create a let statement
    fn let_stmt(name: &str, ann: Option<Type>, init: Option<Expr>) -> Stmt {
        Stmt::Let(name.to_string(), ann, init)
    }

    /// Helper to create a variable expression
    fn var(name: &str) -> Expr {
        Expr::Variable(name.to_string())
    }

    /// Helper to create a literal expression
    fn lit(val: i64) -> Expr {
        Expr::Literal(Constant::Int(val))
    }

    /// Helper to create a function declaration
    fn fn_decl(name: &str, params: Vec<TypedParam>, ret_type: Option<Type>, body: Vec<Stmt>) -> Decl {
        Decl::Function(name.to_string(), params, ret_type, body)
    }

    #[test]
    fn test_undefined_variable() {
        let script = make_script(vec![
            Decl::Stmt(let_stmt("x", None, Some(lit(1)))),
            Decl::Stmt(Stmt::Expression(var("y"))),
        ]);
        
        let errors = check_types(&script);
        assert!(!errors.is_empty());
        assert!(matches!(&errors[0], TypeError::UndefinedVariable(v) if v == "y"));
    }

    #[test]
    fn test_type_mismatch() {
        // This tests the case where we try to use a variable before it's defined
        // with a type annotation that doesn't match
        let script = make_script(vec![
            Decl::Stmt(let_stmt("x", Some(Type::Int), Some(Expr::Literal(Constant::Bool(true))))),
        ]);
        
        let errors = check_types(&script);
        assert!(!errors.is_empty());
        assert!(matches!(&errors[0], TypeError::TypeMismatch { expected: Type::Int, actual: Type::Bool, .. }));
    }

    #[test]
    fn test_wrong_argument_count() {
        let script = make_script(vec![
            fn_decl("foo", vec![TypedParam { name: "a".to_string(), type_annotation: Some(Type::Int) }], Some(Type::Int), vec![]),
            Decl::Stmt(Stmt::Expression(Expr::Call(Box::new(var("foo")), vec![]))),
        ]);
        
        let errors = check_types(&script);
        assert!(!errors.is_empty());
        assert!(matches!(&errors[0], TypeError::WrongArgumentCount { expected: 1, actual: 0, .. }));
    }

    #[test]
    fn test_invalid_operator() {
        // Int + String is not allowed
        let script = make_script(vec![
            Decl::Stmt(let_stmt("a", Some(Type::Int), Some(lit(1)))),
            Decl::Stmt(Stmt::Expression(Expr::Binary(Box::new(var("a")), BinOp::Add, Box::new(Expr::Literal(Constant::String("hello".to_string())))))),
        ]);
        
        let errors = check_types(&script);
        assert!(!errors.is_empty());
        assert!(matches!(&errors[0], TypeError::InvalidOperator { operator: op, .. } if op == "+"));
    }

    #[test]
    fn test_if_condition_must_be_bool() {
        let script = make_script(vec![
            Decl::Stmt(let_stmt("x", Some(Type::Int), Some(lit(1)))),
            Decl::Stmt(Stmt::If(var("x"), Box::new(Stmt::Block(vec![])), None)),
        ]);
        
        let errors = check_types(&script);
        assert!(!errors.is_empty());
        assert!(matches!(&errors[0], TypeError::TypeMismatch { expected: Type::Bool, actual: Type::Int, .. }));
    }

    #[test]
    fn test_valid_arithmetic() {
        let script = make_script(vec![
            Decl::Stmt(let_stmt("a", Some(Type::Int), Some(lit(1)))),
            Decl::Stmt(let_stmt("b", Some(Type::Int), Some(lit(2)))),
            Decl::Stmt(Stmt::Expression(Expr::Binary(Box::new(var("a")), BinOp::Add, Box::new(var("b"))))),
        ]);
        
        let errors = check_types(&script);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_valid_function_call() {
        let script = make_script(vec![
            fn_decl("foo", vec![TypedParam { name: "a".to_string(), type_annotation: Some(Type::Int) }], Some(Type::Int), vec![]),
            Decl::Stmt(Stmt::Expression(Expr::Call(Box::new(var("foo")), vec![lit(1)]))),
        ]);
        
        let errors = check_types(&script);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_builtin_functions() {
        // println is a built-in function that takes Any and returns Unit
        let script = make_script(vec![
            Decl::Stmt(Stmt::Print(Expr::Literal(Constant::String("hello".to_string())))),
        ]);
        
        let errors = check_types(&script);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }
}
