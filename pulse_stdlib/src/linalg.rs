//! Linear Algebra library for Pulse
//! 
//! Provides vector and matrix operations, decompositions, and linear system solvers

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;
use nalgebra::{Matrix2, Matrix3, Matrix4, DMatrix, DVector, LU, QR, SVD};

// Helper functions for extracting data
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

fn f64_vec_to_list(heap: &mut dyn HeapInterface, vec: Vec<f64>) -> Value {
    let list: Vec<Value> = vec.into_iter().map(|f| Value::Float(f)).collect();
    Value::Obj(heap.alloc_object(Object::List(list)))
}

fn i64_vec_to_list(heap: &mut dyn HeapInterface, vec: Vec<i64>) -> Value {
    let list: Vec<Value> = vec.into_iter().map(|i| Value::Int(i)).collect();
    Value::Obj(heap.alloc_object(Object::List(list)))
}

fn create_matrix_metadata(heap: &mut dyn HeapInterface, shape: Vec<i64>, data: Vec<f64>) -> Value {
    let mut map = HashMap::new();
    map.insert("shape".to_string(), i64_vec_to_list(heap, shape));
    map.insert("data".to_string(), f64_vec_to_list(heap, data));
    map.insert("type".to_string(), Value::Obj(heap.alloc_object(Object::String("matrix".to_string()))));
    Value::Obj(heap.alloc_object(Object::Map(map)))
}

fn extract_matrix_data(heap: &dyn HeapInterface, value: &Value) -> PulseResult<(Vec<i64>, Vec<f64>)> {
    match value {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let shape = match map.get("shape") {
                    Some(Value::Obj(h)) => {
                        if let Some(Object::List(list)) = heap.get_object(*h) {
                            list.iter().map(|v| extract_int(heap, v)).collect::<Result<Vec<_>, _>>()?
                        } else {
                            return Err(PulseError::RuntimeError("Shape must be a list".into()));
                        }
                    }
                    _ => return Err(PulseError::RuntimeError("Matrix missing shape".into())),
                };
                
                let data = match map.get("data") {
                    Some(Value::Obj(data_handle)) => {
                        if let Some(Object::List(list)) = heap.get_object(*data_handle) {
                            list_to_f64_vec(heap, list)?
                        } else {
                            return Err(PulseError::RuntimeError("Data must be a list".into()));
                        }
                    }
                    _ => return Err(PulseError::RuntimeError("Matrix missing data".into())),
                };
                
                Ok((shape, data))
            } else {
                Err(PulseError::RuntimeError("Expected map (matrix)".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected map (matrix)".into())),
    }
}

// ============================================================================
// VECTOR OPERATIONS
// ============================================================================

/// vector_dot(a: List, b: List) -> Float
/// Computes the dot product of two vectors
pub fn vector_dot_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("vector_dot expects 2 arguments: a, b".into()));
    }

    let a = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for vector a".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for vector a".into())),
    };
    
    let b = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for vector b".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for vector b".into())),
    };

    let a_data = list_to_f64_vec(heap, &a)?;
    let b_data = list_to_f64_vec(heap, &b)?;

    if a_data.len() != b_data.len() {
        return Err(PulseError::RuntimeError("Vectors must have same length".into()));
    }

    let result: f64 = a_data.iter().zip(b_data.iter()).map(|(x, y)| x * y).sum();
    Ok(Value::Float(result))
}

/// vector_cross(a: List, b: List) -> List
/// Computes the cross product of two 3D vectors
pub fn vector_cross_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("vector_cross expects 2 arguments: a, b".into()));
    }

    let a = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for vector a".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for vector a".into())),
    };
    
    let b = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for vector b".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for vector b".into())),
    };

    let a_data = list_to_f64_vec(heap, &a)?;
    let b_data = list_to_f64_vec(heap, &b)?;

    if a_data.len() != 3 || b_data.len() != 3 {
        return Err(PulseError::RuntimeError("Cross product requires 3D vectors".into()));
    }

    let cx = a_data[1] * b_data[2] - a_data[2] * b_data[1];
    let cy = a_data[2] * b_data[0] - a_data[0] * b_data[2];
    let cz = a_data[0] * b_data[1] - a_data[1] * b_data[0];

    Ok(f64_vec_to_list(heap, vec![cx, cy, cz]))
}

/// vector_normalize(v: List) -> List
/// Normalizes a vector to unit length
pub fn vector_normalize_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("vector_normalize expects 1 argument: v".into()));
    }

    let v = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for vector".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for vector".into())),
    };

    let data = list_to_f64_vec(heap, &v)?;
    let magnitude: f64 = data.iter().map(|x| x * x).sum::<f64>().sqrt();

    if magnitude == 0.0 {
        return Err(PulseError::RuntimeError("Cannot normalize zero vector".into()));
    }

    let normalized: Vec<f64> = data.iter().map(|x| x / magnitude).collect();
    Ok(f64_vec_to_list(heap, normalized))
}

/// vector_magnitude(v: List) -> Float
/// Computes the magnitude (length) of a vector
pub fn vector_magnitude_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("vector_magnitude expects 1 argument: v".into()));
    }

    let v = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for vector".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for vector".into())),
    };

    let data = list_to_f64_vec(heap, &v)?;
    let magnitude: f64 = data.iter().map(|x| x * x).sum::<f64>().sqrt();
    Ok(Value::Float(magnitude))
}

// ============================================================================
// MATRIX OPERATIONS
// ============================================================================

/// matrix_multiply(a: Map, b: Map) -> Map
/// Multiplies two matrices
pub fn matrix_multiply_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("matrix_multiply expects 2 arguments: a, b".into()));
    }

    let (shape_a, data_a) = extract_matrix_data(heap, &args[0])?;
    let (shape_b, data_b) = extract_matrix_data(heap, &args[1])?;

    if shape_a.len() != 2 || shape_b.len() != 2 {
        return Err(PulseError::RuntimeError("Both arguments must be 2D matrices".into()));
    }

    if shape_a[1] != shape_b[0] {
        return Err(PulseError::RuntimeError(format!(
            "Matrix dimensions incompatible: {}x{} and {}x{}",
            shape_a[0], shape_a[1], shape_b[0], shape_b[1]
        )));
    }

    let rows_a = shape_a[0] as usize;
    let cols_a = shape_a[1] as usize;
    let cols_b = shape_b[1] as usize;

    let mut result = vec![0.0; rows_a * cols_b];

    for i in 0..rows_a {
        for j in 0..cols_b {
            for k in 0..cols_a {
                result[i * cols_b + j] += data_a[i * cols_a + k] * data_b[k * cols_b + j];
            }
        }
    }

    Ok(create_matrix_metadata(heap, vec![rows_a as i64, cols_b as i64], result))
}

/// matrix_transpose(m: Map) -> Map
/// Returns the transpose of a matrix
pub fn matrix_transpose_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("matrix_transpose expects 1 argument: m".into()));
    }

    let (shape, data) = extract_matrix_data(heap, &args[0])?;

    if shape.len() != 2 {
        return Err(PulseError::RuntimeError("transpose requires 2D matrix".into()));
    }

    let rows = shape[0] as usize;
    let cols = shape[1] as usize;

    let mut result = vec![0.0; data.len()];
    for i in 0..rows {
        for j in 0..cols {
            result[j * rows + i] = data[i * cols + j];
        }
    }

    Ok(create_matrix_metadata(heap, vec![cols as i64, rows as i64], result))
}

/// matrix_inverse(m: Map) -> Map
/// Computes the inverse of a matrix
pub fn matrix_inverse_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("matrix_inverse expects 1 argument: m".into()));
    }

    let (shape, data) = extract_matrix_data(heap, &args[0])?;

    if shape.len() != 2 || shape[0] != shape[1] {
        return Err(PulseError::RuntimeError("Matrix must be square".into()));
    }

    let n = shape[0] as usize;

    if n <= 4 {
        let result = match n {
            1 => {
                if data[0] == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular".into()));
                }
                vec![1.0 / data[0]]
            },
            2 => {
                let m = Matrix2::new(data[0], data[1], data[2], data[3]);
                if m.determinant() == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular".into()));
                }
                let inv = m.try_inverse().ok_or_else(|| PulseError::RuntimeError("Matrix is singular".into()))?;
                vec![inv[(0,0)], inv[(0,1)], inv[(1,0)], inv[(1,1)]]
            },
            3 => {
                let m = Matrix3::new(
                    data[0], data[1], data[2],
                    data[3], data[4], data[5],
                    data[6], data[7], data[8]
                );
                if m.determinant() == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular".into()));
                }
                let inv = m.try_inverse().ok_or_else(|| PulseError::RuntimeError("Matrix is singular".into()))?;
                vec![
                    inv[(0,0)], inv[(0,1)], inv[(0,2)],
                    inv[(1,0)], inv[(1,1)], inv[(1,2)],
                    inv[(2,0)], inv[(2,1)], inv[(2,2)]
                ]
            },
            4 => {
                let m = Matrix4::new(
                    data[0], data[1], data[2], data[3],
                    data[4], data[5], data[6], data[7],
                    data[8], data[9], data[10], data[11],
                    data[12], data[13], data[14], data[15]
                );
                if m.determinant() == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular".into()));
                }
                let inv = m.try_inverse().ok_or_else(|| PulseError::RuntimeError("Matrix is singular".into()))?;
                vec![
                    inv[(0,0)], inv[(0,1)], inv[(0,2)], inv[(0,3)],
                    inv[(1,0)], inv[(1,1)], inv[(1,2)], inv[(1,3)],
                    inv[(2,0)], inv[(2,1)], inv[(2,2)], inv[(2,3)],
                    inv[(3,0)], inv[(3,1)], inv[(3,2)], inv[(3,3)]
                ]
            },
            _ => return Err(PulseError::RuntimeError("Matrix too large for inversion".into())),
        };
        return Ok(create_matrix_metadata(heap, vec![n as i64, n as i64], result));
    }

    // Use nalgebra for larger matrices
    let dm = DMatrix::from_vec(shape[0] as usize, shape[1] as usize, data);
    let inv = dm.try_inverse()
        .ok_or_else(|| PulseError::RuntimeError("Matrix is singular".into()))?;
    
    let result: Vec<f64> = inv.iter().cloned().collect();
    Ok(create_matrix_metadata(heap, vec![n as i64, n as i64], result))
}

/// matrix_determinant(m: Map) -> Float
/// Computes the determinant of a matrix
pub fn matrix_determinant_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("matrix_determinant expects 1 argument: m".into()));
    }

    let (shape, data) = extract_matrix_data(heap, &args[0])?;

    if shape.len() != 2 || shape[0] != shape[1] {
        return Err(PulseError::RuntimeError("Matrix must be square".into()));
    }

    let n = shape[0] as usize;

    let det = match n {
        1 => data[0],
        2 => {
            let m = Matrix2::new(data[0], data[1], data[2], data[3]);
            m.determinant()
        },
        3 => {
            let m = Matrix3::new(
                data[0], data[1], data[2],
                data[3], data[4], data[5],
                data[6], data[7], data[8]
            );
            m.determinant()
        },
        4 => {
            let m = Matrix4::new(
                data[0], data[1], data[2], data[3],
                data[4], data[5], data[6], data[7],
                data[8], data[9], data[10], data[11],
                data[12], data[13], data[14], data[15]
            );
            m.determinant()
        },
        _ => {
            let dm = DMatrix::from_vec(n, n, data);
            dm.determinant()
        }
    };

    Ok(Value::Float(det))
}

// ============================================================================
// MATRIX DECOMPOSITION
// ============================================================================

/// matrix_lu(m: Map) -> Map
/// Computes LU decomposition returns {L, U, P}
pub fn matrix_lu_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("matrix_lu expects 1 argument: m".into()));
    }

    let (shape, data) = extract_matrix_data(heap, &args[0])?;

    if shape.len() != 2 || shape[0] != shape[1] {
        return Err(PulseError::RuntimeError("Matrix must be square for LU decomposition".into()));
    }

    let n = shape[0] as usize;
    let dm = DMatrix::from_vec(n, n, data);

    let lu = LU::new(dm);
    let l_data: Vec<f64> = lu.l().iter().cloned().collect();
    let u_data: Vec<f64> = lu.u().iter().cloned().collect();
    // Return identity matrix for permutation (simplified)
    let mut p_data: Vec<f64> = Vec::with_capacity((n * n) as usize);
    for i in 0..n {
        for j in 0..n {
            p_data.push(if i == j { 1.0 } else { 0.0 });
        }
    }

    let mut result_map = HashMap::new();
    result_map.insert("L".to_string(), create_matrix_metadata(heap, vec![n as i64, n as i64], l_data));
    result_map.insert("U".to_string(), create_matrix_metadata(heap, vec![n as i64, n as i64], u_data));
    result_map.insert("P".to_string(), create_matrix_metadata(heap, vec![n as i64, n as i64], p_data));

    Ok(Value::Obj(heap.alloc_object(Object::Map(result_map))))
}

/// matrix_qr(m: Map) -> Map
/// Computes QR decomposition returns {Q, R}
pub fn matrix_qr_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("matrix_qr expects 1 argument: m".into()));
    }

    let (shape, data) = extract_matrix_data(heap, &args[0])?;

    if shape.len() != 2 {
        return Err(PulseError::RuntimeError("QR decomposition requires 2D matrix".into()));
    }

    let rows = shape[0] as usize;
    let cols = shape[1] as usize;
    let dm = DMatrix::from_vec(rows, cols, data);

    let qr = QR::new(dm);
    let q_data: Vec<f64> = qr.q().iter().cloned().collect();
    let r_data: Vec<f64> = qr.r().iter().cloned().collect();

    let mut result_map = HashMap::new();
    result_map.insert("Q".to_string(), create_matrix_metadata(heap, vec![rows as i64, cols as i64], q_data));
    result_map.insert("R".to_string(), create_matrix_metadata(heap, vec![cols as i64, cols as i64], r_data));

    Ok(Value::Obj(heap.alloc_object(Object::Map(result_map))))
}

/// matrix_svd(m: Map) -> Map
/// Computes SVD returns {U, S, Vt}
pub fn matrix_svd_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("matrix_svd expects 1 argument: m".into()));
    }

    let (shape, data) = extract_matrix_data(heap, &args[0])?;

    if shape.len() != 2 {
        return Err(PulseError::RuntimeError("SVD requires 2D matrix".into()));
    }

    let rows = shape[0] as usize;
    let cols = shape[1] as usize;
    let dm = DMatrix::from_vec(rows, cols, data);

    let svd = SVD::new(dm, true, true);
    
    let u_data: Vec<f64> = svd.u.unwrap().iter().cloned().collect();
    let s_data: Vec<f64> = svd.singular_values.iter().cloned().collect();
    let vt_data: Vec<f64> = svd.v_t.unwrap().iter().cloned().collect();

    let mut result_map = HashMap::new();
    result_map.insert("U".to_string(), create_matrix_metadata(heap, vec![rows as i64, rows as i64], u_data));
    result_map.insert("S".to_string(), f64_vec_to_list(heap, s_data));
    result_map.insert("Vt".to_string(), create_matrix_metadata(heap, vec![cols as i64, cols as i64], vt_data));

    Ok(Value::Obj(heap.alloc_object(Object::Map(result_map))))
}

// ============================================================================
// LINEAR SYSTEM SOLVER
// ============================================================================

/// solve_linear(a: Map, b: List) -> List
/// Solves Ax = b for x using LU decomposition
pub fn solve_linear_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("solve_linear expects 2 arguments: A, b".into()));
    }

    let (shape_a, data_a) = extract_matrix_data(heap, &args[0])?;
    
    let b = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for b".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for b".into())),
    };

    let b_data = list_to_f64_vec(heap, &b)?;

    if shape_a.len() != 2 || shape_a[0] != shape_a[1] {
        return Err(PulseError::RuntimeError("A must be a square matrix".into()));
    }

    let n = shape_a[0] as usize;
    let dm = DMatrix::from_vec(n, n, data_a);
    let bv = DVector::from_vec(b_data);

    let lu = LU::new(dm);
    let x = lu.solve(&bv).ok_or_else(|| {
        PulseError::RuntimeError("Matrix is singular or system has no solution".into())
    })?;

    Ok(f64_vec_to_list(heap, x.iter().cloned().collect()))
}

/// matrix_identity(n: Int) -> Map
/// Creates an n×n identity matrix
pub fn matrix_identity_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("matrix_identity expects 1 argument: n".into()));
    }

    let n = extract_int(heap, &args[0])? as usize;
    let mut data = vec![0.0; n * n];
    
    for i in 0..n {
        data[i * n + i] = 1.0;
    }

    Ok(create_matrix_metadata(heap, vec![n as i64, n as i64], data))
}

/// matrix_zeros(rows: Int, cols: Int) -> Map
/// Creates a zero matrix
pub fn matrix_zeros_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("matrix_zeros expects 2 arguments: rows, cols".into()));
    }

    let rows = extract_int(heap, &args[0])? as usize;
    let cols = extract_int(heap, &args[1])? as usize;
    let data = vec![0.0; rows * cols];

    Ok(create_matrix_metadata(heap, vec![rows as i64, cols as i64], data))
}

/// matrix_ones(rows: Int, cols: Int) -> Map
/// Creates a matrix filled with ones
pub fn matrix_ones_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("matrix_ones expects 2 arguments: rows, cols".into()));
    }

    let rows = extract_int(heap, &args[0])? as usize;
    let cols = extract_int(heap, &args[1])? as usize;
    let data = vec![1.0; rows * cols];

    Ok(create_matrix_metadata(heap, vec![rows as i64, cols as i64], data))
}
