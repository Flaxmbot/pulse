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
    This,
    Super(String),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Closure(String, Vec<TypedParam>, Option<Type>, Vec<Stmt>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg, Not,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression(Expr),
    Print(Expr),
    Let(String, Option<Type>, Option<Expr>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
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
