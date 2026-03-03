//! Abstract Syntax Tree for Pulse

use crate::types::{Type, TypedParam};
use pulse_core::Constant;

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Constant),
    Variable(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Get(Box<Expr>, String),
    Set(Box<Expr>, String, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    IndexSet(Box<Expr>, Box<Expr>, Box<Expr>),
    This,
    Super(String),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Closure(String, Vec<TypedParam>, Option<Type>, Vec<Stmt>),
    Assign(String, Box<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Receive(Vec<(Pattern, Expr)>),
    Spawn(Box<Expr>),
    Send(Box<Expr>, Box<Expr>),
    ClassLiteral(String, Option<String>, Vec<Decl>),
    /// Type guard expression: `x is Int`
    TypeGuard(Box<Expr>, Type),
    /// Type cast expression: `x as Int`
    TypeCast(Box<Expr>, Type),
    /// Compound assignment: `x += expr`, `x -= expr`, etc.
    CompoundAssign(String, BinOp, Box<Expr>),
    /// Range expression: `start..end`
    Range(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    /// Union type operator: `Int | String`
    Union,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression(Expr),
    Print(Expr),
    Let(String, Option<Type>, Option<Expr>),
    /// Const declaration (immutable)
    Const(String, Option<Type>, Expr),
    /// If with optional type narrowing context
    If(
        Box<Expr>,
        Box<Stmt>,
        Option<Box<Stmt>>,
        Option<TypeNarrowing>,
    ),
    While(Expr, Box<Stmt>),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Expr>, Box<Stmt>),
    Return(Option<Expr>),
    Break,
    Continue,
    Block(Vec<Stmt>),
    Try(Box<Stmt>, String, Box<Stmt>),
    Throw(Expr),
    Send(Expr, Expr),
    Link(Expr),
    Monitor(Expr),
    Spawn(Expr),
    Import(String, Option<String>),
    Receive(Vec<(Pattern, Expr)>),
    /// Match statement for pattern matching on union types
    Match(Expr, Vec<(MatchPattern, Stmt)>),
}

/// Type narrowing information for if statements with type guards
#[derive(Debug, Clone)]
pub struct TypeNarrowing {
    /// Variable name that was type-checked
    pub var_name: String,
    /// Type that the variable is narrowed to in the true branch
    pub narrowed_type: Type,
    /// Type that the variable has in the false branch (if union)
    pub else_type: Option<Type>,
}

#[derive(Debug, Clone)]
pub enum Decl {
    Function(String, Vec<TypedParam>, Option<Type>, Vec<Stmt>),
    Class(String, Option<String>, Vec<Decl>),
    Actor(String, Vec<Stmt>),
    SharedMemory(String, Expr),
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct Script {
    pub declarations: Vec<Decl>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Variable(String),
    Literal(Expr),
}

/// Extended pattern for match expressions
#[derive(Debug, Clone)]
pub enum MatchPattern {
    /// Wildcard pattern: `_`
    Wildcard,
    /// Variable binding: `x`
    Variable(String),
    /// Literal pattern: `42`, `"hello"`
    Literal(Constant),
    /// Range pattern: `1..10`
    Range(Constant, Constant),
    /// Type pattern for union types: `x: Int`
    TypePattern(String, Type),
    /// Constructor pattern: `Some(x)`, `None`
    Constructor(String, Vec<MatchPattern>),
    /// Or pattern: `A | B`
    Or(Box<MatchPattern>, Box<MatchPattern>),
}

/// Helper methods for AST nodes
impl Expr {
    /// Get the type of a literal expression
    pub fn literal_type(&self) -> Option<Type> {
        match self {
            Expr::Literal(constant) => match constant {
                Constant::Int(_) => Some(Type::Int),
                Constant::Float(_) => Some(Type::Float),
                Constant::Bool(_) => Some(Type::Bool),
                Constant::String(_) => Some(Type::String),
                Constant::Unit => Some(Type::Unit),
                _ => None,
            },
            _ => None,
        }
    }

    /// Check if this expression is a type guard
    pub fn is_type_guard(&self) -> bool {
        matches!(self, Expr::TypeGuard(_, _))
    }

    /// Get the variable name if this is a simple variable expression
    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Expr::Variable(name) => Some(name),
            _ => None,
        }
    }
}

impl Stmt {
    /// Check if this statement contains a type guard in its condition
    pub fn has_type_guard_condition(&self) -> bool {
        match self {
            Stmt::If(cond, _, _, _) => cond.has_type_guard(),
            _ => false,
        }
    }
}

impl Expr {
    /// Check if this expression contains a type guard
    fn has_type_guard(&self) -> bool {
        match self {
            Expr::TypeGuard(_, _) => true,
            Expr::Binary(left, _, right) => left.has_type_guard() || right.has_type_guard(),
            Expr::Unary(_, expr) => expr.has_type_guard(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_literal_type() {
        let int_lit = Expr::Literal(Constant::Int(42));
        assert_eq!(int_lit.literal_type(), Some(Type::Int));

        let bool_lit = Expr::Literal(Constant::Bool(true));
        assert_eq!(bool_lit.literal_type(), Some(Type::Bool));
    }

    #[test]
    fn test_type_guard_detection() {
        let guard = Expr::TypeGuard(Box::new(Expr::Variable("x".to_string())), Type::Int);
        assert!(guard.is_type_guard());

        let var = Expr::Variable("x".to_string());
        assert!(!var.is_type_guard());
    }
}
