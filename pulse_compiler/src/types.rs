//! Type system for Pulse
//!
//! Provides optional type annotations for gradual typing with Hindley-Milner inference.
//! Supports generic types, union types, and effect tracking.

use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for generating unique type variable IDs
static TYPE_VAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
    /// Dynamic type - matches anything (escape hatch for gradual typing)
    Any,
    /// Atomic integer type for concurrent access
    Atomic,
    /// User-defined type (future extension)
    Custom(String),
    /// Type variable for Hindley-Milner inference
    Var(TypeVar),
    /// Union type - values can be any of the listed types
    Union(Vec<Type>),
    /// Option type - Some<T> | None
    Option(Box<Type>),
    /// Effect type for tracking side effects in actors
    Effect(EffectSet),
    /// Generic type parameter (for declarations)
    Generic(String),
}

/// Type variable for Hindley-Milner type inference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar {
    pub id: usize,
    pub name: Option<String>,
}

impl TypeVar {
    /// Create a new unique type variable
    pub fn new() -> Self {
        TypeVar {
            id: TYPE_VAR_COUNTER.fetch_add(1, Ordering::SeqCst),
            name: None,
        }
    }

    /// Create a named type variable
    pub fn named(name: impl Into<String>) -> Self {
        TypeVar {
            id: TYPE_VAR_COUNTER.fetch_add(1, Ordering::SeqCst),
            name: Some(name.into()),
        }
    }
}

impl Default for TypeVar {
    fn default() -> Self {
        Self::new()
    }
}

/// Set of effects for effect tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Actor can send messages
    Send,
    /// Actor can receive messages
    Receive,
    /// Function performs IO
    IO,
    /// Function can spawn actors
    Spawn,
    /// Function can access filesystem
    FileRead,
    /// Function can write to filesystem
    FileWrite,
    /// Function can make network requests
    Network,
}

/// Set of effects for a function or expression
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSet {
    pub effects: HashSet<Effect>,
    pub pure: bool, // If true, no effects allowed
}

impl EffectSet {
    /// Create an empty effect set (pure function)
    pub fn pure() -> Self {
        EffectSet {
            effects: HashSet::new(),
            pure: true,
        }
    }

    /// Create an effect set with specific effects
    pub fn new(effects: Vec<Effect>) -> Self {
        EffectSet {
            effects: effects.into_iter().collect(),
            pure: false,
        }
    }

    /// Add an effect to the set
    pub fn add(&mut self, effect: Effect) {
        self.effects.insert(effect);
        self.pure = false;
    }

    /// Check if this effect set includes another (subset relationship)
    pub fn includes(&self, other: &EffectSet) -> bool {
        other.effects.iter().all(|e| self.effects.contains(e))
    }

    /// Merge two effect sets
    pub fn merge(&self, other: &EffectSet) -> EffectSet {
        let mut merged = self.clone();
        for effect in &other.effects {
            merged.add(effect.clone());
        }
        merged
    }
}

impl Type {
    /// Check if this type is compatible with another type
    pub fn is_compatible(&self, other: &Type) -> bool {
        // Any is compatible with everything (dynamic typing escape)
        if matches!(self, Type::Any) || matches!(other, Type::Any) {
            return true;
        }

        // Type variables are compatible with anything (during inference)
        if matches!(self, Type::Var(_)) || matches!(other, Type::Var(_)) {
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
            (Type::Map(k1, v1), Type::Map(k2, v2)) => k1.is_compatible(k2) && v1.is_compatible(v2),
            (Type::Fn(params1, ret1), Type::Fn(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return false;
                }
                params1
                    .iter()
                    .zip(params2.iter())
                    .all(|(a, b)| a.is_compatible(b))
                    && ret1.is_compatible(ret2)
            }
            (Type::Custom(a), Type::Custom(b)) => a == b,
            (Type::Atomic, Type::Atomic) => true,
            // Union type compatibility
            (Type::Union(types), other) => types.iter().any(|t| t.is_compatible(other)),
            (other, Type::Union(types)) => types.iter().any(|t| t.is_compatible(other)),
            // Option type compatibility
            (Type::Option(inner), other) => {
                inner.is_compatible(other) || matches!(other, Type::Unit)
            }
            (other, Type::Option(inner)) => {
                inner.is_compatible(other) || matches!(other, Type::Unit)
            }
            _ => false,
        }
    }

    /// Check if this type is a subtype of another (for type narrowing)
    pub fn is_subtype(&self, other: &Type) -> bool {
        self.is_compatible(other)
    }

    /// Check if this is a concrete type (no type variables)
    pub fn is_concrete(&self) -> bool {
        match self {
            Type::Var(_) => false,
            Type::List(inner) => inner.is_concrete(),
            Type::Map(k, v) => k.is_concrete() && v.is_concrete(),
            Type::Fn(params, ret) => params.iter().all(|p| p.is_concrete()) && ret.is_concrete(),
            Type::Union(types) => types.iter().all(|t| t.is_concrete()),
            Type::Option(inner) => inner.is_concrete(),
            _ => true,
        }
    }

    /// Get the free type variables in this type
    pub fn free_vars(&self) -> HashSet<usize> {
        let mut vars = HashSet::new();
        self.collect_free_vars(&mut vars);
        vars
    }

    fn collect_free_vars(&self, vars: &mut HashSet<usize>) {
        match self {
            Type::Var(tv) => {
                vars.insert(tv.id);
            }
            Type::List(inner) => inner.collect_free_vars(vars),
            Type::Map(k, v) => {
                k.collect_free_vars(vars);
                v.collect_free_vars(vars);
            }
            Type::Fn(params, ret) => {
                for p in params {
                    p.collect_free_vars(vars);
                }
                ret.collect_free_vars(vars);
            }
            Type::Union(types) => {
                for t in types {
                    t.collect_free_vars(vars);
                }
            }
            Type::Option(inner) => inner.collect_free_vars(vars),
            _ => {}
        }
    }

    /// Apply a substitution to this type
    pub fn apply_subst(&self, subst: &Substitution) -> Type {
        match self {
            Type::Var(tv) => {
                if let Some(t) = subst.get(tv.id) {
                    t.clone()
                } else {
                    self.clone()
                }
            }
            Type::List(inner) => Type::List(Box::new(inner.apply_subst(subst))),
            Type::Map(k, v) => Type::Map(
                Box::new(k.apply_subst(subst)),
                Box::new(v.apply_subst(subst)),
            ),
            Type::Fn(params, ret) => Type::Fn(
                params.iter().map(|p| p.apply_subst(subst)).collect(),
                Box::new(ret.apply_subst(subst)),
            ),
            Type::Union(types) => Type::Union(types.iter().map(|t| t.apply_subst(subst)).collect()),
            Type::Option(inner) => Type::Option(Box::new(inner.apply_subst(subst))),
            _ => self.clone(),
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
            Type::Var(tv) => {
                if let Some(name) = &tv.name {
                    write!(f, "'{}(t{})", name, tv.id)
                } else {
                    write!(f, "t{}", tv.id)
                }
            }
            Type::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", type_strs.join(" | "))
            }
            Type::Option(inner) => write!(f, "Option<{}>", inner),
            Type::Effect(eff) => {
                if eff.pure {
                    write!(f, "Pure")
                } else {
                    let effs: Vec<String> =
                        eff.effects.iter().map(|e| format!("{:?}", e)).collect();
                    write!(f, "Effect<{}>", effs.join(", "))
                }
            }
            Type::Generic(name) => write!(f, "{}", name),
        }
    }
}

/// Type substitution for Hindley-Milner inference
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    mappings: std::collections::HashMap<usize, Type>,
}

impl Substitution {
    /// Create an empty substitution
    pub fn new() -> Self {
        Substitution {
            mappings: std::collections::HashMap::new(),
        }
    }

    /// Add a mapping from type variable to type
    pub fn insert(&mut self, var_id: usize, ty: Type) {
        self.mappings.insert(var_id, ty);
    }

    /// Get the type for a variable
    pub fn get(&self, var_id: usize) -> Option<&Type> {
        self.mappings.get(&var_id)
    }

    /// Compose two substitutions: self after other
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = other.clone();
        for (id, ty) in &self.mappings {
            result.insert(*id, ty.apply_subst(other));
        }
        result
    }

    /// Apply this substitution to another substitution
    pub fn apply_to_subst(&self, subst: &Substitution) -> Substitution {
        let mut result = Substitution::new();
        for (id, ty) in &subst.mappings {
            result.insert(*id, ty.apply_subst(self));
        }
        result
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
    pub effects: EffectSet,
}

/// Type constraint for Hindley-Milner inference
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Two types must be equal
    Equal(Type, Type),
    /// First type must be a subtype of second
    Subtype(Type, Type),
    /// Type must have a specific effect
    HasEffect(Type, Effect),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_var_generation() {
        let tv1 = TypeVar::new();
        let tv2 = TypeVar::new();
        assert_ne!(tv1.id, tv2.id);
    }

    #[test]
    fn test_union_type_compatibility() {
        let int_or_string = Type::Union(vec![Type::Int, Type::String]);
        assert!(int_or_string.is_compatible(&Type::Int));
        assert!(int_or_string.is_compatible(&Type::String));
        assert!(!int_or_string.is_compatible(&Type::Float));
    }

    #[test]
    fn test_option_type() {
        let opt_int = Type::Option(Box::new(Type::Int));
        assert!(opt_int.is_compatible(&Type::Int));
        assert!(opt_int.is_compatible(&Type::Unit));
    }

    #[test]
    fn test_effect_set() {
        let mut effects = EffectSet::pure();
        assert!(effects.pure);

        effects.add(Effect::Send);
        assert!(!effects.pure);
        assert!(effects.effects.contains(&Effect::Send));
    }

    #[test]
    fn test_substitution() {
        let mut subst = Substitution::new();
        let tv = TypeVar::new();
        subst.insert(tv.id, Type::Int);

        let ty = Type::List(Box::new(Type::Var(tv.clone())));
        let result = ty.apply_subst(&subst);

        assert_eq!(result, Type::List(Box::new(Type::Int)));
    }
}
