//! Random number generation library for Pulse
//! 
//! Provides random numbers, seedable RNG, and various distributions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;
use std::sync::Mutex;
use rand::{Rng, SeedableRng, distributions::Uniform};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

// Global RNG state - use Mutex for thread safety
lazy_static::lazy_static! {
    static ref GLOBAL_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_entropy());
}

// ============================================================================
// BASIC RANDOM FUNCTIONS
// ============================================================================

/// rand_int() -> Int
/// Returns a random integer
pub fn rand_int_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("rand_int expects 0 arguments".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    Ok(Value::Int(rng.gen()))
}

/// rand_int_range(min: Int, max: Int) -> Int
/// Returns a random integer in [min, max]
pub fn rand_int_range_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("rand_int_range expects 2 arguments: min, max".into()));
    }

    let min = match &args[0] {
        Value::Int(i) => *i,
        Value::Float(f) => *f as i64,
        _ => return Err(PulseError::RuntimeError("Expected integer for min".into())),
    };

    let max = match &args[1] {
        Value::Int(i) => *i,
        Value::Float(f) => *f as i64,
        _ => return Err(PulseError::RuntimeError("Expected integer for max".into())),
    };

    if min > max {
        return Err(PulseError::RuntimeError("min must be less than or equal to max".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    Ok(Value::Int(rng.gen_range(min..=max)))
}

/// rand_float() -> Float
/// Returns a random float in [0, 1)
pub fn rand_float_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("rand_float expects 0 arguments".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    Ok(Value::Float(rng.gen()))
}

/// rand_float_range(min: Float, max: Float) -> Float
/// Returns a random float in [min, max)
pub fn rand_float_range_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("rand_float_range expects 2 arguments: min, max".into()));
    }

    let min = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for min".into())),
    };

    let max = match &args[1] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for max".into())),
    };

    if min > max {
        return Err(PulseError::RuntimeError("min must be less than or equal to max".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    Ok(Value::Float(rng.gen_range(min..max)))
}

/// rand_bool() -> Bool
/// Returns a random boolean
pub fn rand_bool_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("rand_bool expects 0 arguments".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    Ok(Value::Bool(rng.gen()))
}

// ============================================================================
// SEEDABLE RNG
// ============================================================================

/// seed_rng(seed: Int) -> None
/// Seeds the global RNG
pub fn seed_rng_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("seed_rng expects 1 argument: seed".into()));
    }

    let seed = match &args[0] {
        Value::Int(i) => *i as u64,
        Value::Float(f) => *f as u64,
        _ => return Err(PulseError::RuntimeError("Expected integer for seed".into())),
    };

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    *rng = StdRng::seed_from_u64(seed);

    Ok(Value::Unit)
}

/// rng_state() -> Map
/// Returns the current RNG state
pub fn rng_state_native(heap: &mut dyn HeapInterface, _args: &[Value]) -> PulseResult<Value> {
    let mut map = HashMap::new();
    map.insert("seeded".to_string(), Value::Bool(true));
    Ok(Value::Obj(heap.alloc_object(Object::Map(map))))
}

// ============================================================================
// DISTRIBUTIONS (Simplified)
// ============================================================================

/// uniform_sample(min: Float, max: Float) -> Float
/// Sample from uniform distribution
pub fn uniform_sample_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("uniform_sample expects 2 arguments: min, max".into()));
    }

    let min = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for min".into())),
    };

    let max = match &args[1] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for max".into())),
    };

    if min > max {
        return Err(PulseError::RuntimeError("min must be less than or equal to max".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    Ok(Value::Float(rng.gen_range(min..max)))
}

/// normal_sample(mean: Float, std: Float) -> Float
/// Sample from normal (Gaussian) distribution using Box-Muller transform
pub fn normal_sample_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("normal_sample expects 2 arguments: mean, std".into()));
    }

    let mean = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for mean".into())),
    };

    let std = match &args[1] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for std".into())),
    };

    if std <= 0.0 {
        return Err(PulseError::RuntimeError("Standard deviation must be positive".into()));
    }

    // Box-Muller transform
    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let u1: f64 = rng.gen();
    let u2: f64 = rng.gen();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    Ok(Value::Float(mean + std * z))
}

/// exponential_sample(rate: Float) -> Float
/// Sample from exponential distribution
pub fn exponential_sample_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("exponential_sample expects 1 argument: rate".into()));
    }

    let rate = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for rate".into())),
    };

    if rate <= 0.0 {
        return Err(PulseError::RuntimeError("Rate must be positive".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let u: f64 = rng.gen();
    Ok(Value::Float(-u.ln() / rate))
}

/// poisson_sample(lambda: Float) -> Int
/// Sample from Poisson distribution (simplified)
pub fn poisson_sample_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("poisson_sample expects 1 argument: lambda".into()));
    }

    let lambda = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::RuntimeError("Expected number for lambda".into())),
    };

    if lambda <= 0.0 {
        return Err(PulseError::RuntimeError("Lambda must be positive".into()));
    }

    // Knuth's algorithm for Poisson sampling
    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let l = (-lambda).exp();
    let mut k = 0;
    let mut p = 1.0;
    
    while p > l {
        k += 1;
        p *= rng.gen::<f64>();
    }
    
    Ok(Value::Int(k - 1))
}

// ============================================================================
// SAMPLING
// ============================================================================

/// choice(list: List) -> Value
/// Randomly choose an element from a list
pub fn choice_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("choice expects 1 argument: list".into()));
    }

    let list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list".into())),
    };

    if list.is_empty() {
        return Err(PulseError::RuntimeError("Cannot choose from empty list".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    if let Some(item) = list.choose(&mut *rng) {
        Ok(item.clone())
    } else {
        Err(PulseError::RuntimeError("Failed to choose element".into()))
    }
}

// ============================================================================
// SHUFFLE AND SAMPLE
// ============================================================================

/// shuffle(list: List) -> List
/// Shuffles a list in place (returns new list)
pub fn shuffle_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("shuffle expects 1 argument: list".into()));
    }

    let mut list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list".into())),
    };

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    list.shuffle(&mut *rng);

    Ok(Value::Obj(heap.alloc_object(Object::List(list))))
}

/// sample(list: List, n: Int) -> List
/// Randomly samples n elements from list
pub fn sample_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("sample expects 2 arguments: list, n".into()));
    }

    let list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list".into())),
    };

    let n = match &args[1] {
        Value::Int(i) => *i as usize,
        Value::Float(f) => *f as usize,
        _ => return Err(PulseError::RuntimeError("Expected integer for n".into())),
    };

    if list.is_empty() {
        return Err(PulseError::RuntimeError("Cannot sample from empty list".into()));
    }

    let n = n.min(list.len());

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let mut indices: Vec<usize> = (0..list.len()).collect();
    indices.shuffle(&mut *rng);
    indices.truncate(n);
    
    let mut result = Vec::new();
    for &i in &indices {
        result.push(list[i].clone());
    }
    
    Ok(Value::Obj(heap.alloc_object(Object::List(result))))
}

/// choices(list: List, n: Int) -> List
/// Samples with replacement
pub fn choices_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("choices expects 2 arguments: list, n".into()));
    }

    let list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list".into())),
    };

    let n = match &args[1] {
        Value::Int(i) => *i as usize,
        Value::Float(f) => *f as usize,
        _ => return Err(PulseError::RuntimeError("Expected integer for n".into())),
    };

    if list.is_empty() {
        return Err(PulseError::RuntimeError("Cannot sample from empty list".into()));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let dist = Uniform::new(0, list.len());
    let mut result = Vec::new();
    for _ in 0..n {
        let idx = rng.sample(dist);
        result.push(list[idx].clone());
    }
    
    Ok(Value::Obj(heap.alloc_object(Object::List(result))))
}

// ============================================================================
// RANDOM UTILITIES
// ============================================================================

/// random_bytes(n: Int) -> List
/// Generate n random bytes
pub fn random_bytes_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("random_bytes expects 1 argument: n".into()));
    }

    let n = match &args[0] {
        Value::Int(i) => *i as usize,
        Value::Float(f) => *f as usize,
        _ => return Err(PulseError::RuntimeError("Expected integer for n".into())),
    };

    if n == 0 {
        return Ok(Value::Obj(heap.alloc_object(Object::List(vec![]))));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let bytes: Vec<u8> = (0..n).map(|_| rng.gen()).collect();
    let values: Vec<Value> = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
    Ok(Value::Obj(heap.alloc_object(Object::List(values))))
}

/// random_hex(n: Int) -> String
/// Generate n random hex characters
pub fn random_hex_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("random_hex expects 1 argument: n".into()));
    }

    let n = match &args[0] {
        Value::Int(i) => *i as usize,
        Value::Float(f) => *f as usize,
        _ => return Err(PulseError::RuntimeError("Expected integer for n".into())),
    };

    if n == 0 {
        return Ok(Value::Obj(heap.alloc_object(Object::String("".to_string()))));
    }

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let chars: String = (0..n)
        .map(|_| {
            let idx = rng.gen_range(0..16);
            "0123456789abcdef".chars().nth(idx).unwrap()
        })
        .collect();
    
    Ok(Value::Obj(heap.alloc_object(Object::String(chars))))
}

/// random_string(n: Int) -> String
/// Generate a random alphanumeric string
pub fn random_string_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("random_string expects 1 argument: n".into()));
    }

    let n = match &args[0] {
        Value::Int(i) => *i as usize,
        Value::Float(f) => *f as usize,
        _ => return Err(PulseError::RuntimeError("Expected integer for n".into())),
    };

    if n == 0 {
        return Ok(Value::Obj(heap.alloc_object(Object::String("".to_string()))));
    }

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let mut rng = GLOBAL_RNG.lock().map_err(|e| PulseError::RuntimeError(e.to_string()))?;
    let s: String = (0..n)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    
    Ok(Value::Obj(heap.alloc_object(Object::String(s))))
}
