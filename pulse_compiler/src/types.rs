//! Type system for Pulse
//! 
//! Provides optional type annotations for gradual typing.

use std::fmt;

/// Represents a type in the Pulse type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Integer type
    Int,
    /// Floating-point type
    Float,
    /// Boolean type
    Bool,
    /// String type
    String,
    /// Unit type (void)
    Unit,
    /// Actor/Process ID type
    Pid,
    /// List type with element type
    List(Box<Type>),
    /// Map type with key and value types
    Map(Box<Type>, Box<Type>),
    /// Function type with parameter types and return type
    Fn(Vec<Type>, Box<Type>),
    /// Dynamic type - matches anything (escape hatch)
    Any,
    /// Atomic integer type for concurrent access
    Atomic,
    /// User-defined type (future extension)
    Custom(String),
}

impl Type {
    /// Check if this type is compatible with another type
    pub fn is_compatible(&self, other: &Type) -> bool {
        // Any is compatible with everything (dynamic typing escape)
        if matches!(self, Type::Any) || matches!(other, Type::Any) {
            return true;
        }
        
        match (self, other) {
            (Type::Int, Type::Int) => true,
            (Type::Float, Type::Float) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::String, Type::String) => true,
            (Type::Unit, Type::Unit) => true,
            (Type::Pid, Type::Pid) => true,
            (Type::List(a), Type::List(b)) => a.is_compatible(b),
            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                k1.is_compatible(k2) && v1.is_compatible(v2)
            }
            (Type::Fn(params1, ret1), Type::Fn(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return false;
                }
                params1.iter().zip(params2.iter()).all(|(a, b)| a.is_compatible(b))
                    && ret1.is_compatible(ret2)
            }
            (Type::Custom(a), Type::Custom(b)) => a == b,
            (Type::Atomic, Type::Atomic) => true,
            // Numeric coercion: Int can be used where Float expected
            (Type::Int, Type::Float) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Unit => write!(f, "Unit"),
            Type::Pid => write!(f, "Pid"),
            Type::Any => write!(f, "Any"),
            Type::Atomic => write!(f, "Atomic"),
            Type::List(elem) => write!(f, "List<{}>", elem),
            Type::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            Type::Fn(params, ret) => {
                let params_str: Vec<String> = params.iter().map(|t| t.to_string()).collect();
                write!(f, "Fn<({}) -> {}>", params_str.join(", "), ret)
            }
            Type::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Parameter with optional type annotation
#[derive(Debug, Clone)]
pub struct TypedParam {
    pub name: String,
    pub type_annotation: Option<Type>,
}

/// Function signature for type checking
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<TypedParam>,
    pub return_type: Option<Type>,
}
