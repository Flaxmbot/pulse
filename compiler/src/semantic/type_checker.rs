#![allow(clippy::result_large_err)]
//! Type checker for the Pulse compiler
//!
//! Provides Hindley-Milner type inference with gradual typing support.
//! Supports generic types, union types, type guards, and effect tracking.

use crate::ast::{BinOp, Decl, Expr, MatchPattern, Script, Stmt, UnOp};
use crate::types::{Effect, EffectSet, Substitution, Type, TypeVar, TypedParam};
use pulse_ast::Constant;
use std::collections::HashMap;

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
    /// Type unification failed
    UnificationFailure {
        type1: Type,
        type2: Type,
        location: Location,
    },
    /// Infinite type (occurs check failed)
    InfiniteType {
        var: TypeVar,
        ty: Type,
        location: Location,
    },
    /// Effect mismatch - function has effects not in annotation
    EffectMismatch {
        expected: EffectSet,
        actual: EffectSet,
        location: Location,
    },
    /// Invalid effect in pure function
    InvalidEffect { effect: Effect, location: Location },
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
            TypeError::TypeMismatch {
                expected,
                actual,
                location,
            } => {
                write!(
                    f,
                    "Type mismatch at {}: expected {}, got {}",
                    location, expected, actual
                )
            }
            TypeError::InvalidOperator {
                operator,
                left_type,
                right_type,
                location,
            } => {
                if let Some(rt) = right_type {
                    write!(
                        f,
                        "Invalid operator '{}' at {}: {} and {}",
                        operator, location, left_type, rt
                    )
                } else {
                    write!(
                        f,
                        "Invalid operator '{}' at {}: {}",
                        operator, location, left_type
                    )
                }
            }
            TypeError::WrongArgumentCount {
                expected,
                actual,
                function,
                location,
            } => {
                write!(
                    f,
                    "Wrong number of arguments at {}: expected {} for function '{}', got {}",
                    location, expected, function, actual
                )
            }
            TypeError::InvalidCall {
                callee_type,
                location,
            } => {
                write!(f, "Cannot call {} at {}", callee_type, location)
            }
            TypeError::InvalidReturnType {
                expected,
                actual,
                location,
            } => {
                write!(
                    f,
                    "Invalid return type at {}: expected {}, got {}",
                    location, expected, actual
                )
            }
            TypeError::CannotInferType(location) => {
                write!(f, "Cannot infer type at {}", location)
            }
            TypeError::InvalidMemberAccess {
                type_name,
                member,
                location,
            } => {
                write!(
                    f,
                    "Invalid member access at {}: type '{}' has no member '{}'",
                    location, type_name, member
                )
            }
            TypeError::UndefinedClass(name) => {
                write!(f, "Undefined class: {}", name)
            }
            TypeError::PropertyExists {
                class,
                property,
                location,
            } => {
                write!(
                    f,
                    "Property '{}' already exists in class '{}' at {}",
                    property, class, location
                )
            }
            TypeError::ActorWithoutReceive(location) => {
                write!(f, "Actor must have a receive block at {}", location)
            }
            TypeError::InvalidAssignmentTarget(location) => {
                write!(f, "Invalid assignment target at {}", location)
            }
            TypeError::UnificationFailure {
                type1,
                type2,
                location,
            } => {
                write!(
                    f,
                    "Cannot unify types at {}: {} and {}",
                    location, type1, type2
                )
            }
            TypeError::InfiniteType { var, ty, location } => {
                write!(
                    f,
                    "Infinite type at {}: t{} occurs in {}",
                    location, var.id, ty
                )
            }
            TypeError::EffectMismatch {
                expected,
                actual,
                location,
            } => {
                write!(
                    f,
                    "Effect mismatch at {}: expected {:?}, got {:?}",
                    location, expected, actual
                )
            }
            TypeError::InvalidEffect { effect, location } => {
                write!(
                    f,
                    "Invalid effect at {}: {:?} not allowed in this context",
                    location, effect
                )
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

/// Information about a class
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub properties: HashMap<String, Type>,
    pub methods: HashMap<String, (Vec<Type>, Type)>,
}

/// Type context for tracking variable and function types
pub struct TypeContext {
    /// Current scope variables: name -> type
    variables: Vec<HashMap<String, Type>>,
    /// Global functions: name -> (param_types, return_type, effects)
    functions: HashMap<String, (Vec<Type>, Type, EffectSet)>,
    /// Class definitions: name -> ClassInfo
    classes: HashMap<String, ClassInfo>,
    /// Current return type for type checking
    return_type: Option<Type>,
    /// Current effect context (what effects are allowed)
    allowed_effects: EffectSet,
    /// Line counter for location tracking
    current_line: usize,
    /// Column counter for location tracking
    current_column: usize,
    /// Are we inside an actor context?
    in_actor_context: bool,
}

impl TypeContext {
    /// Create a new type context with built-in functions
    pub fn new() -> Self {
        let mut context = TypeContext {
            variables: vec![HashMap::new()],
            functions: HashMap::new(),
            classes: HashMap::new(),
            return_type: None,
            allowed_effects: EffectSet::pure(),
            current_line: 1,
            current_column: 1,
            in_actor_context: false,
        };

        // Add built-in functions
        context.add_builtin_functions();

        context
    }

    /// Add built-in functions to the context
    fn add_builtin_functions(&mut self) {
        // Print functions (pure - they don't affect program logic)
        self.functions.insert(
            "print".to_string(),
            (vec![Type::Any], Type::Unit, EffectSet::pure()),
        );
        self.functions.insert(
            "println".to_string(),
            (vec![Type::Any], Type::Unit, EffectSet::pure()),
        );

        // String functions
        self.functions.insert(
            "len".to_string(),
            (vec![Type::Any], Type::Int, EffectSet::pure()),
        );
        self.functions.insert(
            "str".to_string(),
            (vec![Type::Any], Type::String, EffectSet::pure()),
        );

        // Crypto & Networking
        self.functions.insert(
            "tcp_connect".to_string(),
            (vec![Type::String], Type::Any, EffectSet::pure()),
        );
        self.functions.insert(
            "tcp_write".to_string(),
            (vec![Type::Any, Type::String], Type::Bool, EffectSet::pure()),
        );
        self.functions.insert(
            "tcp_read".to_string(),
            (vec![Type::Any], Type::String, EffectSet::pure()),
        );
        self.functions.insert(
            "websocket_connect".to_string(),
            (vec![Type::String], Type::Any, EffectSet::pure()),
        );
        self.functions.insert(
            "websocket_send".to_string(),
            (vec![Type::Any, Type::String], Type::Bool, EffectSet::pure()),
        );
        self.functions.insert(
            "websocket_recv".to_string(),
            (vec![Type::Any], Type::Any, EffectSet::pure()),
        );
        self.functions.insert(
            "bincode_serialize".to_string(),
            (vec![Type::Any], Type::List(Box::new(Type::Int)), EffectSet::pure()),
        );
        self.functions.insert(
            "bincode_deserialize".to_string(),
            (vec![Type::List(Box::new(Type::Int))], Type::Any, EffectSet::pure()),
        );
        self.functions.insert(
            "sha256".to_string(),
            (vec![Type::String], Type::String, EffectSet::pure()),
        );
        self.functions.insert(
            "to_string".to_string(),
            (vec![Type::Any], Type::String, EffectSet::pure()),
        );

        // List functions
        self.functions.insert(
            "push".to_string(),
            (
                vec![Type::List(Box::new(Type::Any)), Type::Any],
                Type::Unit,
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "pop".to_string(),
            (
                vec![Type::List(Box::new(Type::Any))],
                Type::Any,
                EffectSet::pure(),
            ),
        );

        // Math functions
        self.functions.insert(
            "abs".to_string(),
            (vec![Type::Int], Type::Int, EffectSet::pure()),
        );
        self.functions.insert(
            "floor".to_string(),
            (vec![Type::Float], Type::Float, EffectSet::pure()),
        );
        self.functions.insert(
            "ceil".to_string(),
            (vec![Type::Float], Type::Float, EffectSet::pure()),
        );
        self.functions.insert(
            "sqrt".to_string(),
            (vec![Type::Float], Type::Float, EffectSet::pure()),
        );
        self.functions.insert(
            "sin".to_string(),
            (vec![Type::Float], Type::Float, EffectSet::pure()),
        );
        self.functions.insert(
            "cos".to_string(),
            (vec![Type::Float], Type::Float, EffectSet::pure()),
        );

        // Actor functions (require actor context and have Send/Spawn effects)
        let actor_effects = EffectSet::new(vec![Effect::Send, Effect::Spawn]);
        self.functions.insert(
            "spawn".to_string(),
            (
                vec![Type::Fn(vec![], Box::new(Type::Unit))],
                Type::Pid,
                actor_effects.clone(),
            ),
        );
        self.functions
            .insert("self".to_string(), (vec![], Type::Pid, EffectSet::pure()));
        self.functions.insert(
            "send".to_string(),
            (
                vec![Type::Pid, Type::Any],
                Type::Unit,
                actor_effects.clone(),
            ),
        );
        self.functions.insert(
            "link".to_string(),
            (vec![Type::Pid], Type::Unit, actor_effects.clone()),
        );
        self.functions.insert(
            "monitor".to_string(),
            (vec![Type::Pid], Type::Unit, actor_effects.clone()),
        );

        // String/collection/runtime helpers used by stdlib-style Pulse programs
        self.functions.insert(
            "split_string".to_string(),
            (
                vec![Type::String, Type::String],
                Type::List(Box::new(Type::String)),
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "join_strings".to_string(),
            (
                vec![Type::List(Box::new(Type::Any)), Type::String],
                Type::String,
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "string_replace".to_string(),
            (
                vec![Type::String, Type::String, Type::String],
                Type::String,
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "string_uppercase".to_string(),
            (vec![Type::String], Type::String, EffectSet::pure()),
        );
        self.functions.insert(
            "string_lowercase".to_string(),
            (vec![Type::String], Type::String, EffectSet::pure()),
        );
        self.functions.insert(
            "string_contains".to_string(),
            (
                vec![Type::String, Type::String],
                Type::Bool,
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "substring".to_string(),
            (
                vec![Type::String, Type::Int, Type::Int],
                Type::String,
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "sleep".to_string(),
            (vec![Type::Int], Type::Unit, EffectSet::pure()),
        );
        self.functions.insert(
            "create_set".to_string(),
            (vec![], Type::List(Box::new(Type::Any)), EffectSet::pure()),
        );
        self.functions.insert(
            "add_to_set".to_string(),
            (vec![Type::Any, Type::Any], Type::Unit, EffectSet::pure()),
        );
        self.functions.insert(
            "contains_in_set".to_string(),
            (vec![Type::Any, Type::Any], Type::Bool, EffectSet::pure()),
        );
        self.functions.insert(
            "create_queue".to_string(),
            (vec![], Type::List(Box::new(Type::Any)), EffectSet::pure()),
        );
        self.functions.insert(
            "enqueue".to_string(),
            (vec![Type::Any, Type::Any], Type::Unit, EffectSet::pure()),
        );
        self.functions.insert(
            "dequeue".to_string(),
            (vec![Type::Any], Type::Any, EffectSet::pure()),
        );
        self.functions.insert(
            "peek_queue".to_string(),
            (vec![Type::Any], Type::Any, EffectSet::pure()),
        );
        self.functions.insert(
            "range".to_string(),
            (
                vec![Type::Int, Type::Int, Type::Int],
                Type::List(Box::new(Type::Int)),
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "list_concat".to_string(),
            (
                vec![
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ],
                Type::List(Box::new(Type::Any)),
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "map_has_key".to_string(),
            (
                vec![
                    Type::Map(Box::new(Type::Any), Box::new(Type::Any)),
                    Type::Any,
                ],
                Type::Bool,
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "map_keys".to_string(),
            (
                vec![Type::Map(Box::new(Type::Any), Box::new(Type::Any))],
                Type::List(Box::new(Type::Any)),
                EffectSet::pure(),
            ),
        );
        self.functions.insert(
            "type_of".to_string(),
            (vec![Type::Any], Type::String, EffectSet::pure()),
        );
        self.functions.insert(
            "assert_eq".to_string(),
            (vec![Type::Any, Type::Any], Type::Unit, EffectSet::pure()),
        );
        self.functions.insert(
            "assert_ne".to_string(),
            (vec![Type::Any, Type::Any], Type::Unit, EffectSet::pure()),
        );
        self.functions.insert(
            "input_prompt".to_string(),
            (vec![Type::String], Type::String, EffectSet::pure()),
        );
    }

    /// Get current location
    fn location(&self) -> Location {
        Location {
            line: self.current_line,
            column: self.current_column,
        }
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
    fn define_function(
        &mut self,
        name: String,
        params: Vec<Type>,
        return_type: Type,
        effects: EffectSet,
    ) {
        self.functions.insert(name, (params, return_type, effects));
    }

    /// Look up a function
    #[allow(dead_code)]
    fn lookup_function(&self, name: &str) -> Option<(Vec<Type>, Type, EffectSet)> {
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
    fn get_return_type(&self) -> Option<Type> {
        self.return_type.clone()
    }

    /// Set actor context
    fn set_actor_context(&mut self, in_actor: bool) {
        self.in_actor_context = in_actor;
    }

    /// Check if we're in an actor context
    fn in_actor_context(&self) -> bool {
        self.in_actor_context
    }

    /// Set allowed effects
    fn set_allowed_effects(&mut self, effects: EffectSet) {
        self.allowed_effects = effects;
    }

    /// Get allowed effects
    #[allow(dead_code)]
    fn get_allowed_effects(&self) -> &EffectSet {
        &self.allowed_effects
    }

    /// Check if an effect is allowed
    #[allow(dead_code)]
    fn is_effect_allowed(&self, effect: &Effect) -> bool {
        self.allowed_effects.effects.contains(effect)
    }
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker result
pub type TypeCheckResult<T> = Result<T, TypeError>;

/// Hindley-Milner type inference using Algorithm W
pub struct TypeChecker {
    context: TypeContext,
    errors: Vec<TypeError>,
    /// Current substitution for type variables
    substitution: Substitution,
    /// Next type variable ID
    _next_var_id: usize,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        TypeChecker {
            context: TypeContext::new(),
            errors: Vec::new(),
            substitution: Substitution::new(),
            _next_var_id: 0,
        }
    }

    /// Generate a fresh type variable
    fn fresh_var(&mut self) -> Type {
        let var = TypeVar::new();
        Type::Var(var)
    }

    /// Generate a fresh type variable with a name hint
    fn fresh_var_named(&mut self, name: &str) -> Type {
        let var = TypeVar::named(name);
        Type::Var(var)
    }

    /// Apply current substitution to a type
    fn apply_subst(&self, ty: &Type) -> Type {
        ty.apply_subst(&self.substitution)
    }

    /// Unify two types, updating the substitution
    fn unify(&mut self, t1: &Type, t2: &Type, location: Location) -> TypeCheckResult<()> {
        let t1 = self.apply_subst(t1);
        let t2 = self.apply_subst(t2);

        match (&t1, &t2) {
            // Same concrete types
            (Type::Int, Type::Int) => Ok(()),
            (Type::Float, Type::Float) => Ok(()),
            (Type::Bool, Type::Bool) => Ok(()),
            (Type::String, Type::String) => Ok(()),
            (Type::Unit, Type::Unit) => Ok(()),
            (Type::Pid, Type::Pid) => Ok(()),
            (Type::Atomic, Type::Atomic) => Ok(()),
            (Type::Any, _) | (_, Type::Any) => Ok(()), // Any unifies with anything

            // Type variables
            (Type::Var(v), t) | (t, Type::Var(v)) => self.var_bind(v, t, location),

            // Recursive types
            (Type::List(a), Type::List(b)) => self.unify(a, b, location),
            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                self.unify(k1, k2, location)?;
                self.unify(v1, v2, location)
            }
            (Type::Option(a), Type::Option(b)) => self.unify(a, b, location),
            (Type::Fn(params1, ret1), Type::Fn(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Err(TypeError::UnificationFailure {
                        type1: t1.clone(),
                        type2: t2.clone(),
                        location,
                    });
                }
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    self.unify(p1, p2, location)?;
                }
                self.unify(ret1, ret2, location)
            }
            (Type::Union(types1), Type::Union(types2)) => {
                // Union unification: try to unify pairwise
                // This is a simplified version - full union unification is complex
                if types1.len() == types2.len() {
                    for (ty1, ty2) in types1.iter().zip(types2.iter()) {
                        self.unify(ty1, ty2, location)?;
                    }
                    Ok(())
                } else {
                    Err(TypeError::UnificationFailure {
                        type1: t1.clone(),
                        type2: t2.clone(),
                        location,
                    })
                }
            }
            // Union with concrete type
            (Type::Union(types), t) => {
                // Check if any variant unifies
                for ty in types {
                    if let Ok(()) = self.unify(ty, t, location) {
                        return Ok(());
                    }
                }
                Err(TypeError::UnificationFailure {
                    type1: t1.clone(),
                    type2: t2.clone(),
                    location,
                })
            }
            (t, Type::Union(types)) => {
                for ty in types {
                    if let Ok(()) = self.unify(t, ty, location) {
                        return Ok(());
                    }
                }
                Err(TypeError::UnificationFailure {
                    type1: t1.clone(),
                    type2: t2.clone(),
                    location,
                })
            }
            // Custom types
            (Type::Custom(a), Type::Custom(b)) if a == b => Ok(()),

            // Failure
            _ => Err(TypeError::UnificationFailure {
                type1: t1.clone(),
                type2: t2.clone(),
                location,
            }),
        }
    }

    /// Bind a type variable to a type (occurs check)
    fn var_bind(&mut self, var: &TypeVar, ty: &Type, location: Location) -> TypeCheckResult<()> {
        // Check for infinite type (occurs check)
        if self.occurs_in(var, ty) {
            return Err(TypeError::InfiniteType {
                var: var.clone(),
                ty: ty.clone(),
                location,
            });
        }

        // Add to substitution
        self.substitution.insert(var.id, ty.clone());
        Ok(())
    }

    /// Check if a type variable occurs in a type (occurs check)
    fn occurs_in(&self, var: &TypeVar, ty: &Type) -> bool {
        match ty {
            Type::Var(v) => v.id == var.id,
            Type::List(inner) => self.occurs_in(var, inner),
            Type::Map(k, v) => self.occurs_in(var, k) || self.occurs_in(var, v),
            Type::Option(inner) => self.occurs_in(var, inner),
            Type::Fn(params, ret) => {
                params.iter().any(|p| self.occurs_in(var, p)) || self.occurs_in(var, ret)
            }
            Type::Union(types) => types.iter().any(|t| self.occurs_in(var, t)),
            _ => false,
        }
    }

    /// Check a script and return errors
    pub fn check_script(&mut self, script: &Script) -> Vec<TypeError> {
        // First pass: collect all declarations
        self.collect_declarations(&script.declarations);

        // Second pass: infer and check types
        for decl in &script.declarations {
            if let Err(e) = self.infer_decl(decl) {
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
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.type_annotation
                                .clone()
                                .unwrap_or_else(|| self.fresh_var())
                        })
                        .collect();
                    let ret_type = return_type.clone().unwrap_or_else(|| self.fresh_var());
                    self.context.define_function(
                        name.clone(),
                        param_types,
                        ret_type,
                        EffectSet::pure(),
                    );
                }
                Decl::Class(name, _, class_decls) => {
                    let mut info = ClassInfo {
                        name: name.clone(),
                        properties: HashMap::new(),
                        methods: HashMap::new(),
                    };

                    for class_decl in class_decls {
                        if let Decl::Function(method_name, params, return_type, _) = class_decl {
                            let param_types: Vec<Type> = params
                                .iter()
                                .map(|p| {
                                    p.type_annotation
                                        .clone()
                                        .unwrap_or_else(|| self.fresh_var())
                                })
                                .collect();
                            let ret_type = return_type.clone().unwrap_or_else(|| self.fresh_var());
                            info.methods
                                .insert(method_name.clone(), (param_types, ret_type));
                        }
                    }

                    self.context.define_class(name.clone(), info);
                }
                _ => {}
            }
        }
    }

    /// Infer type of a declaration
    fn infer_decl(&mut self, decl: &Decl) -> TypeCheckResult<Type> {
        match decl {
            Decl::Function(name, params, return_type, body) => {
                self.infer_function(name, params, return_type, body)
            }
            Decl::Class(name, parent, class_decls) => self.infer_class(name, parent, class_decls),
            Decl::Actor(name, body) => self.infer_actor(name, body),
            Decl::Stmt(stmt) => self.infer_stmt(stmt),
            _ => Ok(Type::Unit), // Skip other declarations for now
        }
    }

    /// Infer function type
    fn infer_function(
        &mut self,
        _name: &str,
        params: &[TypedParam],
        return_type: &Option<Type>,
        body: &[Stmt],
    ) -> TypeCheckResult<Type> {
        // Create fresh type variables for parameters without annotations
        let mut param_types = Vec::new();

        self.context.enter_scope();

        // Define parameters in scope
        for param in params.iter() {
            let param_ty = if let Some(ann) = &param.type_annotation {
                ann.clone()
            } else {
                self.fresh_var_named(&param.name)
            };
            param_types.push(param_ty.clone());
            self.context.define_variable(param.name.clone(), param_ty);
        }

        // Set return type
        let ret_ty = if let Some(ann) = return_type {
            ann.clone()
        } else {
            self.fresh_var()
        };
        self.context.set_return_type(Some(ret_ty.clone()));

        // Infer body types
        let mut body_ty = Type::Unit;
        for stmt in body {
            body_ty = self.infer_stmt(stmt)?;
        }

        // Unify inferred body type with expected return type
        if let Some(ann) = return_type {
            self.unify(&body_ty, ann, self.context.location())?;
        }

        self.context.exit_scope();
        self.context.set_return_type(None);

        let func_type = Type::Fn(param_types, Box::new(ret_ty));
        Ok(func_type)
    }

    /// Infer class type
    fn infer_class(
        &mut self,
        _name: &str,
        parent: &Option<String>,
        class_decls: &[Decl],
    ) -> TypeCheckResult<Type> {
        // Check parent exists if specified
        if let Some(parent_name) = parent {
            self.context
                .lookup_class(parent_name)
                .ok_or_else(|| TypeError::UndefinedClass(parent_name.clone()))?;
        }

        // Infer all methods
        for class_decl in class_decls {
            if let Decl::Function(method_name, params, return_type, body) = class_decl {
                self.infer_function(method_name, params, return_type, body)?;
            }
        }

        Ok(Type::Unit)
    }

    /// Infer actor type
    fn infer_actor(&mut self, _name: &str, body: &[Stmt]) -> TypeCheckResult<Type> {
        self.context.enter_scope();
        self.context.set_actor_context(true);

        // Allow Send and Spawn effects in actor context
        let actor_effects = EffectSet::new(vec![Effect::Send, Effect::Receive, Effect::Spawn]);
        self.context.set_allowed_effects(actor_effects);

        // Infer body
        for stmt in body {
            self.infer_stmt(stmt)?;
        }

        self.context.set_allowed_effects(EffectSet::pure());
        self.context.set_actor_context(false);
        self.context.exit_scope();

        Ok(Type::Unit)
    }

    /// Infer statement type
    fn infer_stmt(&mut self, stmt: &Stmt) -> TypeCheckResult<Type> {
        match stmt {
            Stmt::Expression(expr) => self.infer_expr(expr),
            Stmt::Print(expr) => {
                self.infer_expr(expr)?;
                Ok(Type::Unit)
            }
            Stmt::Let(name, type_annotation, init_expr) => {
                let ty = if let Some(expr) = init_expr {
                    let inferred = self.infer_expr(expr)?;
                    if let Some(ann) = type_annotation {
                        self.unify(&inferred, ann, self.context.location())?;
                        ann.clone()
                    } else {
                        inferred
                    }
                } else {
                    type_annotation.clone().unwrap_or_else(|| self.fresh_var())
                };

                self.context.define_variable(name.clone(), ty.clone());
                Ok(Type::Unit)
            }
            Stmt::If(cond, then_branch, else_branch, narrowing) => {
                let cond_ty = self.infer_expr(cond)?;
                self.unify(&cond_ty, &Type::Bool, self.context.location())?;

                // Apply type narrowing if present
                if let Some(narrow) = narrowing {
                    self.context.enter_scope();
                    self.context
                        .define_variable(narrow.var_name.clone(), narrow.narrowed_type.clone());
                    self.infer_stmt(then_branch)?;
                    self.context.exit_scope();
                } else {
                    self.infer_stmt(then_branch)?;
                }

                if let Some(else_br) = else_branch {
                    self.infer_stmt(else_br)?;
                }

                Ok(Type::Unit)
            }
            Stmt::While(cond, body) => {
                let cond_ty = self.infer_expr(cond)?;
                self.unify(&cond_ty, &Type::Bool, self.context.location())?;
                self.infer_stmt(body)
            }
            Stmt::For(init, cond, update, body) => {
                self.context.enter_scope();

                if let Some(init_stmt) = init {
                    if let Stmt::Let(name, ann, expr) = init_stmt.as_ref() {
                        let ty = if let Some(e) = expr {
                            self.infer_expr(e)?
                        } else {
                            ann.clone().unwrap_or_else(|| self.fresh_var())
                        };
                        self.context.define_variable(name.clone(), ty);
                    }
                }

                if let Some(c) = cond {
                    let cond_ty = self.infer_expr(c)?;
                    self.unify(&cond_ty, &Type::Bool, self.context.location())?;
                }

                if let Some(u) = update {
                    self.infer_expr(u)?;
                }

                self.infer_stmt(body)?;
                self.context.exit_scope();
                Ok(Type::Unit)
            }
            Stmt::Return(expr) => {
                let ret_ty = if let Some(e) = expr {
                    self.infer_expr(e)?
                } else {
                    Type::Unit
                };

                if let Some(expected) = self.context.get_return_type() {
                    self.unify(&ret_ty, &expected, self.context.location())?;
                }

                Ok(ret_ty)
            }
            Stmt::Break | Stmt::Continue => Ok(Type::Unit),
            Stmt::Block(stmts) => {
                self.context.enter_scope();
                let mut last_ty = Type::Unit;
                for stmt in stmts {
                    last_ty = self.infer_stmt(stmt)?;
                }
                self.context.exit_scope();
                Ok(last_ty)
            }
            Stmt::Try(body, catch_var, catch_body) => {
                let try_ty = self.infer_stmt(body)?;
                self.context.enter_scope();
                self.context.define_variable(catch_var.clone(), Type::Any);
                let catch_ty = self.infer_stmt(catch_body)?;
                self.context.exit_scope();
                self.unify(&try_ty, &catch_ty, self.context.location())?;
                Ok(try_ty)
            }
            Stmt::Throw(expr) => {
                self.infer_expr(expr)?;
                // Throw has the type of the containing function's return
                Ok(self.context.get_return_type().unwrap_or(Type::Any))
            }
            Stmt::Send(target, message) => {
                // Check we're in an actor context
                if !self.context.in_actor_context() {
                    return Err(TypeError::InvalidEffect {
                        effect: Effect::Send,
                        location: self.context.location(),
                    });
                }

                let target_ty = self.infer_expr(target)?;
                self.unify(&target_ty, &Type::Pid, self.context.location())?;
                self.infer_expr(message)?;
                Ok(Type::Unit)
            }
            Stmt::Link(expr) => {
                let expr_ty = self.infer_expr(expr)?;
                self.unify(&expr_ty, &Type::Pid, self.context.location())?;
                Ok(Type::Unit)
            }
            Stmt::Monitor(expr) => {
                let expr_ty = self.infer_expr(expr)?;
                self.unify(&expr_ty, &Type::Pid, self.context.location())?;
                Ok(Type::Unit)
            }
            Stmt::Spawn(expr) => {
                self.infer_expr(expr)?;
                Ok(Type::Pid)
            }
            Stmt::Import(_path, _alias) => Ok(Type::Unit),
            Stmt::Receive(arms) => {
                for (_pattern, expr) in arms {
                    self.infer_expr(expr)?;
                }
                Ok(Type::Unit)
            }
            Stmt::Match(expr, arms) => {
                let match_ty = self.infer_expr(expr)?;
                let result_ty = self.fresh_var();

                for (pattern, stmt) in arms {
                    self.context.enter_scope();

                    // Bind pattern variables
                    self.bind_pattern(&match_ty, pattern)?;

                    let arm_ty = self.infer_stmt(stmt)?;
                    self.unify(&result_ty, &arm_ty, self.context.location())?;

                    self.context.exit_scope();
                }

                Ok(result_ty)
            }
            Stmt::Const(name, type_annotation, init_expr) => {
                let inferred = self.infer_expr(init_expr)?;
                let ty = if let Some(ann) = type_annotation {
                    self.unify(&inferred, ann, self.context.location())?;
                    ann.clone()
                } else {
                    inferred
                };
                self.context.define_variable(name.clone(), ty);
                Ok(Type::Unit)
            }
        }
    }

    /// Bind pattern variables to types
    fn bind_pattern(&mut self, match_ty: &Type, pattern: &MatchPattern) -> TypeCheckResult<()> {
        match pattern {
            MatchPattern::Wildcard => Ok(()),
            MatchPattern::Variable(name) => {
                self.context.define_variable(name.clone(), match_ty.clone());
                Ok(())
            }
            MatchPattern::Literal(_) => {
                // Literals don't bind variables
                Ok(())
            }
            MatchPattern::Range(_, _) => {
                // Range patterns don't bind variables
                Ok(())
            }
            MatchPattern::TypePattern(name, ty) => {
                // Narrow the type
                self.unify(match_ty, ty, self.context.location())?;
                self.context.define_variable(name.clone(), ty.clone());
                Ok(())
            }
            MatchPattern::Constructor(_, patterns) => {
                // Constructor patterns - simplified
                for p in patterns {
                    self.bind_pattern(match_ty, p)?;
                }
                Ok(())
            }
            MatchPattern::Or(left, right) => {
                self.bind_pattern(match_ty, left)?;
                self.bind_pattern(match_ty, right)
            }
        }
    }

    /// Infer expression type
    fn infer_expr(&mut self, expr: &Expr) -> TypeCheckResult<Type> {
        match expr {
            Expr::Literal(constant) => Ok(match constant {
                Constant::Int(_) => Type::Int,
                Constant::Float(_) => Type::Float,
                Constant::Bool(_) => Type::Bool,
                Constant::String(_) => Type::String,
                Constant::Unit => Type::Unit,
                _ => Type::Any,
            }),
            Expr::Variable(name) => self
                .context
                .lookup_variable(name)
                .or_else(|| {
                    self.context
                        .lookup_function(name)
                        .map(|(params, ret, _)| Type::Fn(params, Box::new(ret)))
                })
                .or_else(|| {
                    self.context
                        .lookup_class(name)
                        .map(|_| Type::Custom(name.clone()))
                })
                .ok_or_else(|| TypeError::UndefinedVariable(name.clone())),
            Expr::Binary(left, op, right) => {
                let left_ty = self.infer_expr(left)?;
                let right_ty = self.infer_expr(right)?;

                match op {
                    BinOp::Add => {
                        // String concatenation is permitted in dynamic contexts.
                        if left_ty == Type::String || right_ty == Type::String {
                            return Ok(Type::String);
                        }
                        self.unify(&left_ty, &Type::Int, self.context.location())
                            .or_else(|_| {
                                self.unify(&left_ty, &Type::Float, self.context.location())
                            })?;
                        self.unify(&right_ty, &Type::Int, self.context.location())
                            .or_else(|_| {
                                self.unify(&right_ty, &Type::Float, self.context.location())
                            })?;

                        if left_ty == Type::Float || right_ty == Type::Float {
                            Ok(Type::Float)
                        } else {
                            Ok(Type::Int)
                        }
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                        // Arithmetic: both operands must be numeric, result is Float if either is Float
                        self.unify(&left_ty, &Type::Int, self.context.location())
                            .or_else(|_| {
                                self.unify(&left_ty, &Type::Float, self.context.location())
                            })?;
                        self.unify(&right_ty, &Type::Int, self.context.location())
                            .or_else(|_| {
                                self.unify(&right_ty, &Type::Float, self.context.location())
                            })?;

                        if left_ty == Type::Float || right_ty == Type::Float {
                            Ok(Type::Float)
                        } else {
                            Ok(Type::Int)
                        }
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        // Comparison: operands must be compatible
                        self.unify(&left_ty, &right_ty, self.context.location())?;
                        Ok(Type::Bool)
                    }
                    BinOp::And | BinOp::Or => {
                        self.unify(&left_ty, &Type::Bool, self.context.location())?;
                        self.unify(&right_ty, &Type::Bool, self.context.location())?;
                        Ok(Type::Bool)
                    }
                    BinOp::Union => {
                        // Create union type
                        Ok(Type::Union(vec![left_ty, right_ty]))
                    }
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        // Bitwise operators: both operands must be Int, result is Int
                        self.unify(&left_ty, &Type::Int, self.context.location())?;
                        self.unify(&right_ty, &Type::Int, self.context.location())?;
                        Ok(Type::Int)
                    }
                }
            }
            Expr::Unary(op, expr) => {
                let ty = self.infer_expr(expr)?;
                match op {
                    UnOp::Neg => {
                        self.unify(&ty, &Type::Int, self.context.location())
                            .or_else(|_| self.unify(&ty, &Type::Float, self.context.location()))?;
                        Ok(ty)
                    }
                    UnOp::Not => {
                        self.unify(&ty, &Type::Bool, self.context.location())?;
                        Ok(Type::Bool)
                    }
                    UnOp::BitNot => {
                        self.unify(&ty, &Type::Int, self.context.location())?;
                        Ok(Type::Int)
                    }
                }
            }
            Expr::Call(callee, args) => {
                let callee_ty = self.infer_expr(callee)?;
                let arg_types: Vec<Type> = args
                    .iter()
                    .map(|a| self.infer_expr(a))
                    .collect::<Result<Vec<_>, _>>()?;

                match callee_ty {
                    Type::Fn(params, ret) => {
                        if params.len() != arg_types.len() {
                            return Err(TypeError::WrongArgumentCount {
                                expected: params.len(),
                                actual: arg_types.len(),
                                function: "anonymous".to_string(),
                                location: self.context.location(),
                            });
                        }
                        for (p, a) in params.iter().zip(arg_types.iter()) {
                            self.unify(p, a, self.context.location())?;
                        }
                        Ok(*ret)
                    }
                    Type::Var(_) => {
                        // Fresh function type - infer from args
                        let ret_ty = self.fresh_var();
                        let func_ty = Type::Fn(arg_types, Box::new(ret_ty.clone()));
                        self.unify(&callee_ty, &func_ty, self.context.location())?;
                        Ok(ret_ty)
                    }
                    Type::Custom(name) => {
                        let _ = arg_types;
                        Ok(Type::Custom(name))
                    }
                    _ => Err(TypeError::InvalidCall {
                        callee_type: callee_ty,
                        location: self.context.location(),
                    }),
                }
            }
            Expr::Get(obj, _name) => {
                let obj_ty = self.infer_expr(obj)?;
                // For now, return a fresh variable for any property access
                // In a full implementation, we'd look up the property type
                let _ = obj_ty;
                Ok(self.fresh_var())
            }
            Expr::Set(obj, _name, value) => {
                let obj_ty = self.infer_expr(obj)?;
                let val_ty = self.infer_expr(value)?;
                let _ = obj_ty;
                Ok(val_ty)
            }
            Expr::Index(obj, index) => {
                let obj_ty = self.infer_expr(obj)?;
                let index_ty = self.infer_expr(index)?;

                match obj_ty {
                    Type::List(elem_ty) => {
                        self.unify(&index_ty, &Type::Int, self.context.location())?;
                        Ok(*elem_ty)
                    }
                    Type::Map(key_ty, val_ty) => {
                        self.unify(&index_ty, &key_ty, self.context.location())?;
                        Ok(*val_ty)
                    }
                    _ => {
                        // Unknown - return fresh var
                        Ok(self.fresh_var())
                    }
                }
            }
            Expr::IndexSet(obj, index, value) => {
                let obj_ty = self.infer_expr(obj)?;
                let index_ty = self.infer_expr(index)?;
                let val_ty = self.infer_expr(value)?;

                match obj_ty {
                    Type::List(elem_ty) => {
                        self.unify(&index_ty, &Type::Int, self.context.location())?;
                        self.unify(&elem_ty, &val_ty, self.context.location())?;
                    }
                    Type::Map(key_ty, val_ty_map) => {
                        self.unify(&index_ty, &key_ty, self.context.location())?;
                        self.unify(&val_ty_map, &val_ty, self.context.location())?;
                    }
                    _ => {}
                }

                Ok(val_ty)
            }
            Expr::This => Ok(self.fresh_var()),
            Expr::Super(_) => Ok(self.fresh_var()),
            Expr::List(elements) => {
                let elem_ty = if elements.is_empty() {
                    self.fresh_var()
                } else {
                    let first_ty = self.infer_expr(&elements[0])?;
                    for elem in &elements[1..] {
                        let ty = self.infer_expr(elem)?;
                        self.unify(&first_ty, &ty, self.context.location())?;
                    }
                    first_ty
                };
                Ok(Type::List(Box::new(elem_ty)))
            }
            Expr::Map(entries) => {
                if entries.is_empty() {
                    Ok(Type::Map(
                        Box::new(self.fresh_var()),
                        Box::new(self.fresh_var()),
                    ))
                } else {
                    let (first_key, first_val) = &entries[0];
                    let mut key_ty = self.infer_expr(first_key)?;
                    let mut val_ty = self.infer_expr(first_val)?;

                    for (k, v) in &entries[1..] {
                        let k_ty = self.infer_expr(k)?;
                        let v_ty = self.infer_expr(v)?;
                        if self.unify(&key_ty, &k_ty, self.context.location()).is_err() {
                            key_ty = Type::Any;
                        }
                        if self.unify(&val_ty, &v_ty, self.context.location()).is_err() {
                            val_ty = Type::Any;
                        }
                    }

                    Ok(Type::Map(Box::new(key_ty), Box::new(val_ty)))
                }
            }
            Expr::Closure(_name, params, return_type, body) => {
                self.context.enter_scope();

                let mut param_types = Vec::new();
                for param in params {
                    let ty = param
                        .type_annotation
                        .clone()
                        .unwrap_or_else(|| self.fresh_var());
                    param_types.push(ty.clone());
                    self.context.define_variable(param.name.clone(), ty);
                }

                let ret_ty = if let Some(ann) = return_type {
                    ann.clone()
                } else {
                    self.fresh_var()
                };
                self.context.set_return_type(Some(ret_ty.clone()));

                let mut last_ty = Type::Unit;
                for stmt in body {
                    last_ty = self.infer_stmt(stmt)?;
                }

                self.unify(&ret_ty, &last_ty, self.context.location())?;

                self.context.exit_scope();
                self.context.set_return_type(None);

                Ok(Type::Fn(param_types, Box::new(ret_ty)))
            }
            Expr::Assign(name, value) => {
                let val_ty = self.infer_expr(value)?;

                if let Some(var_ty) = self.context.lookup_variable(name) {
                    self.unify(&var_ty, &val_ty, self.context.location())?;
                    Ok(val_ty)
                } else {
                    // Define new variable
                    self.context.define_variable(name.clone(), val_ty.clone());
                    Ok(val_ty)
                }
            }
            Expr::MethodCall(obj, _method, args) => {
                let obj_ty = self.infer_expr(obj)?;
                for arg in args {
                    self.infer_expr(arg)?;
                }
                let _ = obj_ty;
                Ok(self.fresh_var())
            }
            Expr::Receive(_) => Ok(self.fresh_var()),
            Expr::Spawn(closure) => {
                self.infer_expr(closure)?;
                Ok(Type::Pid)
            }
            Expr::Send(target, msg) => {
                let target_ty = self.infer_expr(target)?;
                self.unify(&target_ty, &Type::Pid, self.context.location())?;
                self.infer_expr(msg)?;
                Ok(Type::Unit)
            }
            Expr::ClassLiteral(_, _, _) => Ok(Type::Unit),
            Expr::TypeGuard(var, _ty) => {
                let _var_ty = self.infer_expr(var)?;
                // Type guard returns Bool
                // The type narrowing happens at the statement level
                Ok(Type::Bool)
            }
            Expr::TypeCast(expr, ty) => {
                self.infer_expr(expr)?;
                Ok(ty.clone())
            }
            Expr::CompoundAssign(name, op, rhs) => {
                // Look up the variable
                let var_ty = self
                    .context
                    .lookup_variable(name)
                    .ok_or_else(|| TypeError::UndefinedVariable(name.clone()))?;
                let rhs_ty = self.infer_expr(rhs)?;

                // Apply the binary op type rules
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                        self.unify(&var_ty, &Type::Int, self.context.location())
                            .or_else(|_| {
                                self.unify(&var_ty, &Type::Float, self.context.location())
                            })?;
                        self.unify(&rhs_ty, &Type::Int, self.context.location())
                            .or_else(|_| {
                                self.unify(&rhs_ty, &Type::Float, self.context.location())
                            })?;
                    }
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        self.unify(&var_ty, &Type::Int, self.context.location())?;
                        self.unify(&rhs_ty, &Type::Int, self.context.location())?;
                    }
                    _ => {}
                }
                Ok(var_ty)
            }
            Expr::Range(start, end) => {
                let start_ty = self.infer_expr(start)?;
                let end_ty = self.infer_expr(end)?;
                self.unify(&start_ty, &Type::Int, self.context.location())?;
                self.unify(&end_ty, &Type::Int, self.context.location())?;
                Ok(Type::List(Box::new(Type::Int)))
            }
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_inference_literal() {
        let checker = TypeChecker::new();
        // Test would go here with actual parsing
        assert_eq!(checker._next_var_id, 0);
    }

    #[test]
    fn test_unification_int() {
        let mut checker = TypeChecker::new();
        assert!(checker
            .unify(&Type::Int, &Type::Int, Location { line: 1, column: 1 })
            .is_ok());
    }

    #[test]
    fn test_unification_var() {
        let mut checker = TypeChecker::new();
        let var = checker.fresh_var();
        assert!(checker
            .unify(&var, &Type::Int, Location { line: 1, column: 1 })
            .is_ok());

        // After unification, applying subst should give Int
        let result = checker.apply_subst(&var);
        assert_eq!(result, Type::Int);
    }

    #[test]
    fn test_unification_list() {
        let mut checker = TypeChecker::new();
        let var = checker.fresh_var();
        let list_var = Type::List(Box::new(var.clone()));
        let list_int = Type::List(Box::new(Type::Int));

        assert!(checker
            .unify(&list_var, &list_int, Location { line: 1, column: 1 })
            .is_ok());

        // The inner type should now be Int
        let result = checker.apply_subst(&var);
        assert_eq!(result, Type::Int);
    }

    #[test]
    fn test_occurs_check() {
        let mut checker = TypeChecker::new();
        let var = if let Type::Var(v) = checker.fresh_var() {
            v
        } else {
            unreachable!()
        };
        let list_var = Type::List(Box::new(Type::Var(var.clone())));

        // This should fail the occurs check
        let result = checker.unify(
            &Type::Var(var.clone()),
            &list_var,
            Location { line: 1, column: 1 },
        );
        assert!(result.is_err());
    }
}
