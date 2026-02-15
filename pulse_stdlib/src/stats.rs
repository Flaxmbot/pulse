//! Statistics library for Pulse
//! 
//! Provides descriptive statistics, probability distributions, hypothesis testing, 
//! correlation and regression

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;
use std::f64::consts::PI;
use rand::Rng;

// Helper functions
fn extract_float(heap: &dyn HeapInterface, value: &Value) -> PulseResult<f64> {
    match value {
        Value::Int(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        Value::Obj(handle) => {
            if let Some(Object::String(s)) = heap.get_object(*handle) {
                s.parse::<f64>()
                    .map_err(|_| PulseError::RuntimeError("Cannot parse string as float".into()))
            } else {
                Err(PulseError::RuntimeError("Expected numeric value".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected numeric value".into())),
    }
}

fn extract_int(_heap: &dyn HeapInterface, value: &Value) -> PulseResult<i64> {
    match value {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(*f as i64),
        _ => Err(PulseError::RuntimeError("Expected integer value".into())),
    }
}

fn list_to_f64_vec(heap: &dyn HeapInterface, list: &[Value]) -> PulseResult<Vec<f64>> {
    list.iter().map(|v| extract_float(heap, v)).collect()
}

#[allow(dead_code)]
fn f64_vec_to_list(heap: &mut dyn HeapInterface, vec: Vec<f64>) -> Value {
    let list: Vec<Value> = vec.into_iter().map(|f| Value::Float(f)).collect();
    Value::Obj(heap.alloc_object(Object::List(list)))
}

fn create_dict(heap: &mut dyn HeapInterface, map: HashMap<String, Value>) -> Value {
    Value::Obj(heap.alloc_object(Object::Map(map)))
}

// ============================================================================
// DESCRIPTIVE STATISTICS
// ============================================================================

/// mean(data: List) -> Float
/// Computes the arithmetic mean
pub fn mean_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("mean expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data)?;
    if values.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute mean of empty list".into()));
    }

    let sum: f64 = values.iter().sum();
    Ok(Value::Float(sum / values.len() as f64))
}

/// median(data: List) -> Float
/// Computes the median
pub fn median_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("median expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let mut values = list_to_f64_vec(heap, &data)?;
    if values.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute median of empty list".into()));
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let mid = values.len() / 2;
    let result = if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    };

    Ok(Value::Float(result))
}

/// mode(data: List) -> Float
/// Computes the mode (most frequent value)
pub fn mode_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("mode expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data)?;
    if values.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute mode of empty list".into()));
    }

    use std::collections::HashMap;
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for &v in &values {
        let key = (v * 1e9).round() as i64;
        *counts.entry(key).or_insert(0) += 1;
    }

    let mode = counts.iter().max_by_key(|(_, &c)| c).map(|(&k, _)| k as f64 / 1e9);
    
    match mode {
        Some(m) => Ok(Value::Float(m)),
        None => Err(PulseError::RuntimeError("Could not compute mode".into())),
    }
}

/// std(data: List) -> Float
/// Computes the standard deviation
pub fn std_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("std expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data)?;
    if values.len() < 2 {
        return Err(PulseError::RuntimeError("Need at least 2 values for std".into()));
    }

    let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
    let variance: f64 = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    
    Ok(Value::Float(variance.sqrt()))
}

/// variance(data: List) -> Float
/// Computes the variance
pub fn variance_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("variance expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data)?;
    if values.len() < 2 {
        return Err(PulseError::RuntimeError("Need at least 2 values for variance".into()));
    }

    let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
    let variance: f64 = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    
    Ok(Value::Float(variance))
}

/// min(data: List) -> Float
/// Computes the minimum value
pub fn min_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("min expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data)?;
    if values.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute min of empty list".into()));
    }

    Ok(Value::Float(values.into_iter().fold(f64::INFINITY, f64::min)))
}

/// max(data: List) -> Float
/// Computes the maximum value
pub fn max_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("max expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data)?;
    if values.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute max of empty list".into()));
    }

    Ok(Value::Float(values.into_iter().fold(f64::NEG_INFINITY, f64::max)))
}

/// describe(data: List) -> Map
/// Returns descriptive statistics
pub fn describe_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("describe expects 1 argument: data".into()));
    }

    let data = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data)?;
    if values.is_empty() {
        return Err(PulseError::RuntimeError("Cannot describe empty list".into()));
    }

    let n = values.len() as f64;
    let sum: f64 = values.iter().sum();
    let mean = sum / n;
    
    let variance: f64 = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt();
    
    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let min = sorted.first().copied().unwrap_or(0.0);
    let max = sorted.last().copied().unwrap_or(0.0);
    
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    let mut map = HashMap::new();
    map.insert("count".to_string(), Value::Float(n));
    map.insert("mean".to_string(), Value::Float(mean));
    map.insert("std".to_string(), Value::Float(std));
    map.insert("min".to_string(), Value::Float(min));
    map.insert("max".to_string(), Value::Float(max));
    map.insert("median".to_string(), Value::Float(median));
    map.insert("variance".to_string(), Value::Float(variance));

    Ok(create_dict(heap, map))
}

// ============================================================================
// PROBABILITY DISTRIBUTIONS
// ============================================================================

/// Normal (Gaussian) PDF
fn normal_pdf(x: f64, mean: f64, std: f64) -> f64 {
    let coefficient = 1.0 / (std * (2.0 * PI).sqrt());
    let exponent = -0.5 * ((x - mean) / std).powi(2);
    coefficient * exponent.exp()
}

/// Normal distribution PDF
/// normal_pdf(x: Float, mean: Float, std: Float) -> Float
pub fn normal_pdf_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("normal_pdf expects 3 arguments: x, mean, std".into()));
    }

    let x = extract_float(heap, &args[0])?;
    let mean = extract_float(heap, &args[1])?;
    let std = extract_float(heap, &args[2])?;

    if std <= 0.0 {
        return Err(PulseError::RuntimeError("Standard deviation must be positive".into()));
    }

    Ok(Value::Float(normal_pdf(x, mean, std)))
}

/// Normal distribution CDF
pub fn normal_cdf_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("normal_cdf expects 3 arguments: x, mean, std".into()));
    }

    let x = extract_float(heap, &args[0])?;
    let mean = extract_float(heap, &args[1])?;
    let std = extract_float(heap, &args[2])?;

    if std <= 0.0 {
        return Err(PulseError::RuntimeError("Standard deviation must be positive".into()));
    }

    let z = (x - mean) / std;
    // Approximation of standard normal CDF
    let cdf = 0.5 * (1.0 + erf(z / (2.0_f64).sqrt()));
    
    Ok(Value::Float(cdf))
}

// Error function approximation
fn erf(x: f64) -> f64 {
    let a1 =  0.254829592;
    let a2 = -0.284496736;
    let a3 =  1.421413741;
    let a4 = -1.453152027;
    let a5 =  1.061405429;
    let p  =  0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}

/// Generate random sample from normal distribution
/// normal_sample(mean: Float, std: Float) -> Float
pub fn normal_sample_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("normal_sample expects 2 arguments: mean, std".into()));
    }

    let mean = extract_float(heap, &args[0])?;
    let std = extract_float(heap, &args[1])?;

    if std <= 0.0 {
        return Err(PulseError::RuntimeError("Standard deviation must be positive".into()));
    }

    let mut rng = rand::thread_rng();
    let _sample: f64 = rng.gen_range(-1.0..1.0);
    // Box-Muller transform
    let u1: f64 = rng.gen();
    let u2: f64 = rng.gen();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).sin();
    
    Ok(Value::Float(mean + std * z))
}

/// Binomial probability mass function
/// binomial_pmf(k: Int, n: Int, p: Float) -> Float
pub fn binomial_pmf_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("binomial_pmf expects 3 arguments: k, n, p".into()));
    }

    let k = extract_int(heap, &args[0])? as usize;
    let n = extract_int(heap, &args[1])? as usize;
    let p = extract_float(heap, &args[2])?;

    if p < 0.0 || p > 1.0 {
        return Err(PulseError::RuntimeError("Probability p must be between 0 and 1".into()));
    }

    if k > n {
        return Ok(Value::Float(0.0));
    }

    let binomial_coeff = factorial(n) / (factorial(k) * factorial(n - k));
    let probability = binomial_coeff as f64 * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32);

    Ok(Value::Float(probability))
}

fn factorial(n: usize) -> f64 {
    if n <= 1 { 1.0 } else { (2..=n).fold(1.0, |acc, x| acc * x as f64) }
}

/// Poisson probability mass function
/// poisson_pmf(k: Int, lambda: Float) -> Float
pub fn poisson_pmf_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("poisson_pmf expects 2 arguments: k, lambda".into()));
    }

    let k = extract_int(heap, &args[0])? as usize;
    let lambda = extract_float(heap, &args[1])?;

    if lambda <= 0.0 {
        return Err(PulseError::RuntimeError("Lambda must be positive".into()));
    }

    let probability = lambda.powi(k as i32) * (-lambda).exp() / factorial(k) as f64;

    Ok(Value::Float(probability))
}

// ============================================================================
// HYPOTHESIS TESTING
// ============================================================================

/// T-test for comparing two means
/// ttest(sample1: List, sample2: List) -> Map
pub fn ttest_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("ttest expects 2 arguments: sample1, sample2".into()));
    }

    let s1 = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for sample1".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for sample1".into())),
    };

    let s2 = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for sample2".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for sample2".into())),
    };

    let v1 = list_to_f64_vec(heap, &s1)?;
    let v2 = list_to_f64_vec(heap, &s2)?;

    if v1.len() < 2 || v2.len() < 2 {
        return Err(PulseError::RuntimeError("Each sample must have at least 2 values".into()));
    }

    let n1 = v1.len() as f64;
    let n2 = v2.len() as f64;
    
    let m1: f64 = v1.iter().sum::<f64>() / n1;
    let m2: f64 = v2.iter().sum::<f64>() / n2;
    
    let var1: f64 = v1.iter().map(|x| (x - m1).powi(2)).sum::<f64>() / (n1 - 1.0);
    let var2: f64 = v2.iter().map(|x| (x - m2).powi(2)).sum::<f64>() / (n2 - 1.0);

    // Welch's t-test
    let se = (var1 / n1 + var2 / n2).sqrt();
    let t_stat = if se == 0.0 { 0.0 } else { (m1 - m2) / se };
    
    // Degrees of freedom (Welch-Satterthwaite)
    let df = ((var1 / n1 + var2 / n2).powi(2) / 
              ((var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0))).sqrt();

    // Two-tailed p-value approximation
    let p_value = 2.0 * (1.0 - student_t_cdf(t_stat.abs(), df));

    let mut map = HashMap::new();
    map.insert("t_statistic".to_string(), Value::Float(t_stat));
    map.insert("p_value".to_string(), Value::Float(p_value));
    map.insert("df".to_string(), Value::Float(df));
    map.insert("mean1".to_string(), Value::Float(m1));
    map.insert("mean2".to_string(), Value::Float(m2));

    Ok(create_dict(heap, map))
}

fn student_t_cdf(t: f64, df: f64) -> f64 {
    let x = df / (df + t * t);
    let incomplete_beta = 0.5 * beta_inc(df / 2.0, 0.5, x);
    if t > 0.0 { 1.0 - incomplete_beta } else { incomplete_beta }
}

fn beta_inc(a: f64, b: f64, x: f64) -> f64 {
    // Simplified incomplete beta approximation
    let bt = if x == 0.0 || x == 1.0 { 0.0 } else {
        // Use approximation for log gamma
        let ln_gamma_approx = |z: f64| -> f64 {
            let c = [76.18009172947146, -86.50532032941677, 24.01409824083091,
                     -1.231739572450155, 0.1208650973866179e-2, -0.5395239384953e-5];
            let mut y = z;
            let mut tmp = z + 5.5;
            tmp -= (z + 0.5) * tmp.ln();
            let mut ser = 1.000000000190015;
            for &coef in &c {
                y += 1.0;
                ser += coef / y;
            }
            -tmp + (z - 0.5) * z.ln() + (2.5066282746310005 * ser) / z
        };
        (a + b).ln() - a.ln() - b.ln() + a * x.ln() + (1.0 - x).ln() + ln_gamma_approx(a + b) - ln_gamma_approx(a) - ln_gamma_approx(b)
    };
    if x < (a + 1.0) / (a + b + 2.0) {
        bt / a
    } else {
        1.0 - bt / b
    }
}

/// Chi-square test for independence
/// chisquare(observed: List, expected: List) -> Map
pub fn chisquare_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("chisquare expects 2 arguments: observed, expected".into()));
    }

    let obs = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for observed".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for observed".into())),
    };

    let exp = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for expected".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for expected".into())),
    };

    let obs_vals = list_to_f64_vec(heap, &obs)?;
    let exp_vals = list_to_f64_vec(heap, &exp)?;

    if obs_vals.len() != exp_vals.len() {
        return Err(PulseError::RuntimeError("Observed and expected must have same length".into()));
    }

    let chi_square: f64 = obs_vals.iter()
        .zip(exp_vals.iter())
        .map(|(o, e)| if *e != 0.0 { (o - e).powi(2) / e } else { 0.0 })
        .sum();

    let df = (obs_vals.len() - 1) as f64;
    let p_value = 1.0 - chi_square_cdf(chi_square, df);

    let mut map = HashMap::new();
    map.insert("chi_square".to_string(), Value::Float(chi_square));
    map.insert("p_value".to_string(), Value::Float(p_value));
    map.insert("df".to_string(), Value::Float(df));

    Ok(create_dict(heap, map))
}

fn chi_square_cdf(x: f64, df: f64) -> f64 {
    // Simplified chi-square CDF using gamma function
    if x <= 0.0 { 0.0 } else { gammainc(df / 2.0, x / 2.0) }
}

fn gammainc(a: f64, x: f64) -> f64 {
    // Simplified incomplete gamma approximation
    if x <= 0.0 { 0.0 } else if x < a + 1.0 {
        let mut sum = 0.0;
        let mut term = 1.0 / a;
        sum += term;
        for n in 1..100 {
            term *= x / (a + n as f64);
            sum += term;
            if term.abs() < 1e-10 { break; }
        }
        sum * x.powf(a) * (-x).exp()
    } else {
        1.0 - gammainc(a + 1.0, x - 1.0)
    }
}

// ============================================================================
// CORRELATION AND REGRESSION
// ============================================================================

/// Pearson correlation coefficient
/// correlation(x: List, y: List) -> Float
pub fn correlation_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("correlation expects 2 arguments: x, y".into()));
    }

    let x_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for x".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for x".into())),
    };

    let y_list = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for y".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for y".into())),
    };

    let x = list_to_f64_vec(heap, &x_list)?;
    let y = list_to_f64_vec(heap, &y_list)?;

    if x.len() != y.len() || x.len() < 2 {
        return Err(PulseError::RuntimeError("Lists must have same length and at least 2 elements".into()));
    }

    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = x.iter().map(|a| a * a).sum();
    let sum_y2: f64 = y.iter().map(|a| a * a).sum();

    let numerator = n * sum_xy - sum_x * sum_y;
    let denominator = ((n * sum_x2 - sum_x.powi(2)) * (n * sum_y2 - sum_y.powi(2))).sqrt();

    if denominator == 0.0 {
        return Ok(Value::Float(0.0));
    }

    Ok(Value::Float(numerator / denominator))
}

/// Linear regression
/// linear_regression(x: List, y: List) -> Map
pub fn linear_regression_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("linear_regression expects 2 arguments: x, y".into()));
    }

    let x_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for x".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for x".into())),
    };

    let y_list = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for y".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for y".into())),
    };

    let x = list_to_f64_vec(heap, &x_list)?;
    let y = list_to_f64_vec(heap, &y_list)?;

    if x.len() != y.len() || x.len() < 2 {
        return Err(PulseError::RuntimeError("Lists must have same length and at least 2 elements".into()));
    }

    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = x.iter().map(|a| a * a).sum();

    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
    let intercept = (sum_y - slope * sum_x) / n;

    let mut map = HashMap::new();
    map.insert("slope".to_string(), Value::Float(slope));
    map.insert("intercept".to_string(), Value::Float(intercept));
    map.insert("equation".to_string(), Value::Obj(heap.alloc_object(Object::String(
        format!("y = {}x + {}", slope, intercept)
    ))));

    Ok(create_dict(heap, map))
}

/// Compute covariance
/// covariance(x: List, y: List) -> Float
pub fn covariance_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("covariance expects 2 arguments: x, y".into()));
    }

    let x_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for x".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for x".into())),
    };

    let y_list = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for y".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for y".into())),
    };

    let x = list_to_f64_vec(heap, &x_list)?;
    let y = list_to_f64_vec(heap, &y_list)?;

    if x.len() != y.len() || x.len() < 2 {
        return Err(PulseError::RuntimeError("Lists must have same length and at least 2 elements".into()));
    }

    let n = x.len() as f64;
    let mean_x: f64 = x.iter().sum::<f64>() / n;
    let mean_y: f64 = y.iter().sum::<f64>() / n;

    let cov: f64 = x.iter().zip(y.iter())
        .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
        .sum::<f64>() / n;

    Ok(Value::Float(cov))
}
