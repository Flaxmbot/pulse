//! NumPy-like numerical computing library for Pulse
//! 
//! Provides arrays, matrices, and numerical operations

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;
use nalgebra::{Matrix2, Matrix3, Matrix4};
use std::f64::consts::PI;

/// Extract a f64 value from a Pulse Value
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

/// Extract an integer value from a Pulse Value
fn extract_int(heap: &dyn HeapInterface, value: &Value) -> PulseResult<i64> {
    match value {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(*f as i64),
        Value::Obj(handle) => {
            if let Some(Object::String(s)) = heap.get_object(*handle) {
                s.parse::<i64>()
                    .map_err(|_| PulseError::RuntimeError("Cannot parse string as integer".into()))
            } else {
                Err(PulseError::RuntimeError("Expected integer value".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected integer value".into())),
    }
}

/// Convert a Pulse List to Vec<f64>
fn list_to_f64_vec(heap: &dyn HeapInterface, list: &[Value]) -> PulseResult<Vec<f64>> {
    list.iter()
        .map(|v| extract_float(heap, v))
        .collect()
}

/// Convert Vec<f64> to a Pulse List
fn f64_vec_to_list(heap: &mut dyn HeapInterface, vec: Vec<f64>) -> Value {
    let list: Vec<Value> = vec.into_iter()
        .map(|f| Value::Float(f))
        .collect();
    Value::Obj(heap.alloc_object(Object::List(list)))
}

/// Convert Vec<i64> to a Pulse List
fn i64_vec_to_list(heap: &mut dyn HeapInterface, vec: Vec<i64>) -> Value {
    let list: Vec<Value> = vec.into_iter()
        .map(|i| Value::Int(i))
        .collect();
    Value::Obj(heap.alloc_object(Object::List(list)))
}

/// Create array metadata Map
fn create_array_metadata(heap: &mut dyn HeapInterface, shape: Vec<i64>, data: Vec<f64>) -> Value {
    let mut map = HashMap::new();
    map.insert("shape".to_string(), i64_vec_to_list(heap, shape));
    map.insert("data".to_string(), f64_vec_to_list(heap, data));
    map.insert("dtype".to_string(), Value::Obj(heap.alloc_object(Object::String("float64".to_string()))));
    Value::Obj(heap.alloc_object(Object::Map(map)))
}

/// Extract shape from a list
fn extract_shape(heap: &dyn HeapInterface, value: &Value) -> PulseResult<Vec<i64>> {
    match value {
        Value::Obj(handle) => {
            if let Some(Object::List(list)) = heap.get_object(*handle) {
                list.iter()
                    .map(|v| extract_int(heap, v))
                    .collect()
            } else {
                Err(PulseError::RuntimeError("Expected list for shape".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected list for shape".into())),
    }
}

/// Extract data from an array (map with data and shape)
fn extract_array_data(heap: &dyn HeapInterface, value: &Value) -> PulseResult<(Vec<i64>, Vec<f64>)> {
    match value {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let shape = extract_shape(heap, map.get("shape").unwrap_or(&Value::Int(0)))?;
                
                let data = match map.get("data") {
                    Some(Value::Obj(data_handle)) => {
                        if let Some(Object::List(list)) = heap.get_object(*data_handle) {
                            list_to_f64_vec(heap, list)?
                        } else {
                            return Err(PulseError::RuntimeError("Data must be a list".into()));
                        }
                    }
                    _ => return Err(PulseError::RuntimeError("Array missing data".into())),
                };
                
                Ok((shape, data))
            } else {
                Err(PulseError::RuntimeError("Expected map (array)".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected map (array)".into())),
    }
}

// ============================================================================
// ARRAY CREATION FUNCTIONS
// ============================================================================

/// array_create(shape: List, fill_value: Float) -> Map
/// Creates an array with the given shape filled with fill_value
pub fn array_create_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("array_create expects 2 arguments: shape and fill_value".into()));
    }

    let shape = extract_shape(heap, &args[0])?;

    let fill_value = extract_float(heap, &args[1])?;

    let total_size: usize = shape.iter().map(|&x| x as usize).product();
    let data = vec![fill_value; total_size];

    Ok(create_array_metadata(heap, shape, data))
}

/// array_zeros(shape: List) -> Map
/// Creates a zero-filled array
pub fn array_zeros_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("array_zeros expects 1 argument: shape".into()));
    }

    let shape = extract_shape(heap, &args[0])?;

    let total_size: usize = shape.iter().map(|&x| x as usize).product();
    let data = vec![0.0; total_size];

    Ok(create_array_metadata(heap, shape, data))
}

/// array_ones(shape: List) -> Map
/// Creates a ones-filled array
pub fn array_ones_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("array_ones expects 1 argument: shape".into()));
    }

    let shape = extract_shape(heap, &args[0])?;

    let total_size: usize = shape.iter().map(|&x| x as usize).product();
    let data = vec![1.0; total_size];

    Ok(create_array_metadata(heap, shape, data))
}

/// array_eye(n: Int) -> Map
/// Creates an identity matrix
pub fn array_eye_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("array_eye expects 1 argument: n".into()));
    }

    let n = extract_int(heap, &args[0])? as usize;
    let mut data = vec![0.0; n * n];
    
    for i in 0..n {
        data[i * n + i] = 1.0;
    }

    Ok(create_array_metadata(heap, vec![n as i64, n as i64], data))
}

/// array_linspace(start: Float, end: Float, num: Int) -> List
/// Creates evenly spaced numbers
pub fn array_linspace_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("array_linspace expects 3 arguments: start, end, num".into()));
    }

    let start = extract_float(heap, &args[0])?;
    let end = extract_float(heap, &args[1])?;
    let num = extract_int(heap, &args[2])? as usize;

    if num == 0 {
        return Ok(f64_vec_to_list(heap, vec![]));
    }

    let step = (end - start) / (num - 1) as f64;
    let data: Vec<f64> = (0..num)
        .map(|i| start + step * i as f64)
        .collect();

    Ok(f64_vec_to_list(heap, data))
}

/// array_arange(start: Float, stop: Float, step: Float) -> List
/// Creates a range of values
pub fn array_arange_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("array_arange expects 3 arguments: start, stop, step".into()));
    }

    let start = extract_float(heap, &args[0])?;
    let stop = extract_float(heap, &args[1])?;
    let step = extract_float(heap, &args[2])?;

    if step == 0.0 {
        return Err(PulseError::RuntimeError("step cannot be zero".into()));
    }

    let mut data = Vec::new();
    let mut current = start;
    
    if step > 0.0 {
        while current < stop {
            data.push(current);
            current += step;
        }
    } else {
        while current > stop {
            data.push(current);
            current += step;
        }
    }

    Ok(f64_vec_to_list(heap, data))
}

// ============================================================================
// ARRAY OPERATION FUNCTIONS
// ============================================================================

/// array_shape(arr: Map) -> List
/// Gets the shape of an array
pub fn array_shape_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("array_shape expects 1 argument: arr".into()));
    }

    let (shape, _) = extract_array_data(heap, &args[0])?;
    Ok(i64_vec_to_list(heap, shape))
}

/// array_reshape(arr: Map, shape: List) -> Map
/// Reshapes an array
pub fn array_reshape_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("array_reshape expects 2 arguments: arr, shape".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    let new_shape = extract_shape(heap, &args[1])?;

    let new_size: usize = new_shape.iter().map(|&x| x as usize).product();
    if new_size != data.len() {
        return Err(PulseError::RuntimeError(format!(
            "Cannot reshape from {} elements to {} elements",
            data.len(), new_size
        )));
    }

    Ok(create_array_metadata(heap, new_shape, data))
}

/// array_get(arr: Map, indices: List) -> Float
/// Gets an element from an array
pub fn array_get_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("array_get expects 2 arguments: arr, indices".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;

    let indices = extract_shape(heap, &args[1])?;

    if indices.len() != shape.len() {
        return Err(PulseError::RuntimeError(format!(
            "Expected {} indices but got {}", shape.len(), indices.len()
        )));
    }

    // Validate indices
    for (i, (idx, dim)) in indices.iter().zip(shape.iter()).enumerate() {
        if *idx >= *dim {
            return Err(PulseError::RuntimeError(format!(
                "Index {} out of bounds for dimension with size {}", idx, dim
            )));
        }
    }

    // Calculate flat index
    let mut flat_index = 0;
    let mut multiplier = 1;
    for i in (0..indices.len()).rev() {
        flat_index += indices[i] as usize * multiplier;
        multiplier *= shape[i] as usize;
    }

    Ok(Value::Float(data[flat_index]))
}

/// array_set(arr: Map, indices: List, value: Float) -> Map
/// Sets an element in an array
pub fn array_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("array_set expects 3 arguments: arr, indices, value".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let indices = extract_shape(heap, &args[1])?;
    let value = extract_float(heap, &args[2])?;

    if indices.len() != shape.len() {
        return Err(PulseError::RuntimeError(format!(
            "Expected {} indices but got {}", shape.len(), indices.len()
        )));
    }

    // Validate indices
    for (i, (idx, dim)) in indices.iter().zip(shape.iter()).enumerate() {
        if *idx >= *dim {
            return Err(PulseError::RuntimeError(format!(
                "Index {} out of bounds for dimension with size {}", idx, dim
            )));
        }
    }

    // Calculate flat index
    let mut flat_index = 0;
    let mut multiplier = 1;
    for i in (0..indices.len()).rev() {
        flat_index += indices[i] as usize * multiplier;
        multiplier *= shape[i] as usize;
    }

    let mut new_data = data;
    new_data[flat_index] = value;

    Ok(create_array_metadata(heap, shape, new_data))
}

/// array_slice(arr: Map, start: Int, end: Int) -> List
/// Slices an array (1D only for now)
pub fn array_slice_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("array_slice expects 3 arguments: arr, start, end".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;

    let start = extract_int(heap, &args[1])? as usize;
    let end = extract_int(heap, &args[2])? as usize;

    if start > data.len() || end > data.len() || start > end {
        return Err(PulseError::RuntimeError("Invalid slice indices".into()));
    }

    Ok(f64_vec_to_list(heap, data[start..end].to_vec()))
}

// ============================================================================
// MATRIX OPERATION FUNCTIONS
// ============================================================================

/// matmul(a: Map, b: Map) -> Map
/// Matrix multiplication
pub fn matmul_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("matmul expects 2 arguments: a, b".into()));
    }

    let (shape_a, data_a) = extract_array_data(heap, &args[0])?;
    let (shape_b, data_b) = extract_array_data(heap, &args[1])?;

    // Check dimensions for multiplication
    if shape_a.len() != 2 || shape_b.len() != 2 {
        return Err(PulseError::RuntimeError("Both arguments must be 2D matrices".into()));
    }

    if shape_a[1] != shape_b[0] {
        return Err(PulseError::RuntimeError(format!(
            "Matrix dimensions incompatible: {}x{} and {}x{}",
            shape_a[0], shape_a[1], shape_b[0], shape_b[1]
        )));
    }

    // Perform matrix multiplication
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

    Ok(create_array_metadata(heap, vec![rows_a as i64, cols_b as i64], result))
}

/// dot(a: Map, b: Map) -> Map
/// Dot product of two vectors or matrices
pub fn dot_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("dot expects 2 arguments: a, b".into()));
    }

    let (shape_a, data_a) = extract_array_data(heap, &args[0])?;
    let (shape_b, data_b) = extract_array_data(heap, &args[1])?;

    // Handle 1D dot product
    if shape_a.len() == 1 && shape_b.len() == 1 {
        if shape_a[0] != shape_b[0] {
            return Err(PulseError::RuntimeError("Vectors must have same length".into()));
        }
        let result: f64 = data_a.iter().zip(data_b.iter()).map(|(a, b)| a * b).sum();
        return Ok(f64_vec_to_list(heap, vec![result]));
    }

    // Otherwise, use matrix multiplication
    matmul_native(heap, args)
}

/// transpose(arr: Map) -> Map
/// Transposes a matrix
pub fn transpose_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("transpose expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;

    if shape.len() != 2 {
        return Err(PulseError::RuntimeError("transpose requires 2D matrix".into()));
    }

    let rows = shape[0] as usize;
    let cols = shape[1] as usize;

    // Transpose
    let mut result = vec![0.0; data.len()];
    for i in 0..rows {
        for j in 0..cols {
            result[j * rows + i] = data[i * cols + j];
        }
    }

    Ok(create_array_metadata(heap, vec![cols as i64, rows as i64], result))
}

/// inverse(matrix: Map) -> Map
/// Computes the matrix inverse
pub fn inverse_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("inverse expects 1 argument: matrix".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;

    if shape.len() != 2 || shape[0] != shape[1] {
        return Err(PulseError::RuntimeError("Matrix must be square".into()));
    }

    let n = shape[0] as usize;

    if n <= 4 {
        // Use nalgebra for small matrices
        let result = match n {
            1 => {
                // 1x1 matrix inverse
                if data[0] == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular (determinant = 0)".into()));
                }
                vec![1.0 / data[0]]
            },
            2 => {
                let m = Matrix2::new(
                    data[0], data[1],
                    data[2], data[3]
                );
                if m.determinant() == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular (determinant = 0)".into()));
                }
                m.try_inverse().unwrap().as_slice().to_vec()
            },
            3 => {
                let m = Matrix3::new(
                    data[0], data[1], data[2],
                    data[3], data[4], data[5],
                    data[6], data[7], data[8]
                );
                if m.determinant() == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular (determinant = 0)".into()));
                }
                m.try_inverse().unwrap().as_slice().to_vec()
            },
            4 => {
                let m = Matrix4::new(
                    data[0], data[1], data[2], data[3],
                    data[4], data[5], data[6], data[7],
                    data[8], data[9], data[10], data[11],
                    data[12], data[13], data[14], data[15]
                );
                if m.determinant() == 0.0 {
                    return Err(PulseError::RuntimeError("Matrix is singular (determinant = 0)".into()));
                }
                m.try_inverse().unwrap().as_slice().to_vec()
            },
            _ => unreachable!(),
        };
        
        return Ok(create_array_metadata(heap, vec![n as i64, n as i64], result));
    }

    // For larger matrices, use Gaussian elimination
    let mut augmented: Vec<Vec<f64>> = Vec::new();
    for i in 0..n {
        let mut row = data[i * n..(i + 1) * n].to_vec();
        // Add identity matrix
        for j in 0..n {
            row.push(if j == i { 1.0 } else { 0.0 });
        }
        augmented.push(row);
    }

    // Gaussian elimination
    for col in 0..n {
        // Find pivot
        let mut max_row = col;
        for row in (col + 1)..n {
            if augmented[row][col].abs() > augmented[max_row][col].abs() {
                max_row = row;
            }
        }
        augmented.swap(col, max_row);

        if augmented[col][col].abs() < 1e-10 {
            return Err(PulseError::RuntimeError("Matrix is singular".into()));
        }

        // Scale pivot row
        let pivot = augmented[col][col];
        for j in 0..(2 * n) {
            augmented[col][j] /= pivot;
        }

        // Eliminate column
        for row in 0..n {
            if row != col {
                let factor = augmented[row][col];
                for j in 0..(2 * n) {
                    augmented[row][j] -= factor * augmented[col][j];
                }
            }
        }
    }

    // Extract inverse
    let mut result = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in n..(2 * n) {
            result.push(augmented[i][j]);
        }
    }

    Ok(create_array_metadata(heap, vec![n as i64, n as i64], result))
}

/// determinant(matrix: Map) -> Float
/// Computes the matrix determinant
pub fn determinant_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("determinant expects 1 argument: matrix".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;

    if shape.len() != 2 || shape[0] != shape[1] {
        return Err(PulseError::RuntimeError("Matrix must be square".into()));
    }

    let n = shape[0] as usize;

    // Use nalgebra for small matrices
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
            // For larger matrices, use LU decomposition (simplified)
            let mut lu = data.clone();
            let mut sign = 1.0;
            
            for i in 0..n {
                for j in i..n {
                    let mut sum = lu[i * n + j];
                    for k in 0..i {
                        sum -= lu[i * n + k] * lu[k * n + j];
                    }
                    lu[i * n + j] = sum;
                }
                for j in (i + 1)..n {
                    let mut sum = lu[j * n + i];
                    for k in 0..i {
                        sum -= lu[j * n + k] * lu[k * n + i];
                    }
                    lu[j * n + i] = sum / lu[i * n + i];
                }
                if lu[i * n + i] == 0.0 {
                    return Ok(Value::Float(0.0));
                }
                if i > 0 && lu[i * n + i] == 0.0 {
                    sign = -sign;
                }
            }
            
            let mut result = sign;
            for i in 0..n {
                result *= lu[i * n + i];
            }
            result
        }
    };

    Ok(Value::Float(det))
}

// ============================================================================
// ELEMENT-WISE OPERATIONS
// ============================================================================

/// add(a: Map, b: Map) -> Map
/// Element-wise addition
pub fn add_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("add expects 2 arguments: a, b".into()));
    }

    let (shape, data_a) = extract_array_data(heap, &args[0])?;
    let (_, data_b) = extract_array_data(heap, &args[1])?;

    if data_a.len() != data_b.len() {
        return Err(PulseError::RuntimeError("Arrays must have same size".into()));
    }

    let result: Vec<f64> = data_a.iter().zip(data_b.iter()).map(|(a, b)| a + b).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// sub(a: Map, b: Map) -> Map
/// Element-wise subtraction
pub fn sub_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("sub expects 2 arguments: a, b".into()));
    }

    let (shape, data_a) = extract_array_data(heap, &args[0])?;
    let (_, data_b) = extract_array_data(heap, &args[1])?;

    if data_a.len() != data_b.len() {
        return Err(PulseError::RuntimeError("Arrays must have same size".into()));
    }

    let result: Vec<f64> = data_a.iter().zip(data_b.iter()).map(|(a, b)| a - b).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// mul(a: Map, b: Map) -> Map
/// Element-wise multiplication
pub fn mul_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("mul expects 2 arguments: a, b".into()));
    }

    let (shape, data_a) = extract_array_data(heap, &args[0])?;
    let (_, data_b) = extract_array_data(heap, &args[1])?;

    if data_a.len() != data_b.len() {
        return Err(PulseError::RuntimeError("Arrays must have same size".into()));
    }

    let result: Vec<f64> = data_a.iter().zip(data_b.iter()).map(|(a, b)| a * b).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// div(a: Map, b: Map) -> Map
/// Element-wise division
pub fn div_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("div expects 2 arguments: a, b".into()));
    }

    let (shape, data_a) = extract_array_data(heap, &args[0])?;
    let (_, data_b) = extract_array_data(heap, &args[1])?;

    if data_a.len() != data_b.len() {
        return Err(PulseError::RuntimeError("Arrays must have same size".into()));
    }

    let result: Vec<f64> = data_a.iter().zip(data_b.iter()).map(|(a, b)| a / b).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// sqrt(arr: Map) -> Map
/// Element-wise square root
pub fn sqrt_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("sqrt expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.sqrt()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// abs(arr: Map) -> Map
/// Element-wise absolute value
pub fn abs_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("abs expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.abs()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// pow(arr: Map, n: Float) -> Map
/// Element-wise power
pub fn pow_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("pow expects 2 arguments: arr, n".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let n = extract_float(heap, &args[1])?;
    let result: Vec<f64> = data.iter().map(|x| x.powf(n)).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// sin(arr: Map) -> Map
/// Element-wise sine
pub fn sin_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("sin expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.sin()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// cos(arr: Map) -> Map
/// Element-wise cosine
pub fn cos_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("cos expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.cos()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// tan(arr: Map) -> Map
/// Element-wise tangent
pub fn tan_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("tan expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.tan()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// exp(arr: Map) -> Map
/// Element-wise exponential
pub fn exp_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("exp expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.exp()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// log(arr: Map) -> Map
/// Element-wise natural logarithm
pub fn numpy_log_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("log expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.ln()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// log10(arr: Map) -> Map
/// Element-wise base-10 logarithm
pub fn log10_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("log10 expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.log10()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// floor(arr: Map) -> Map
/// Element-wise floor
pub fn floor_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("floor expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.floor()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// ceil(arr: Map) -> Map
/// Element-wise ceiling
pub fn ceil_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("ceil expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.ceil()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// round(arr: Map) -> Map
/// Element-wise rounding
pub fn round_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("round expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| x.round()).collect();
    Ok(create_array_metadata(heap, shape, result))
}

/// negate(arr: Map) -> Map
/// Element-wise negation
pub fn negate_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("negate expects 1 argument: arr".into()));
    }

    let (shape, data) = extract_array_data(heap, &args[0])?;
    let result: Vec<f64> = data.iter().map(|x| -x).collect();
    Ok(create_array_metadata(heap, shape, result))
}

// ============================================================================
// AGGREGATION FUNCTIONS
// ============================================================================

/// sum(arr: Map) -> Float
/// Sum of all elements
pub fn sum_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("sum expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    let result: f64 = data.iter().sum();
    Ok(Value::Float(result))
}

/// mean(arr: Map) -> Float
/// Mean of all elements
pub fn mean_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("mean expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    if data.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute mean of empty array".into()));
    }
    let result: f64 = data.iter().sum::<f64>() / data.len() as f64;
    Ok(Value::Float(result))
}

/// std(arr: Map) -> Float
/// Standard deviation of all elements
pub fn std_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("std expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    if data.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute std of empty array".into()));
    }

    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;
    let result = variance.sqrt();

    Ok(Value::Float(result))
}

/// var(arr: Map) -> Float
/// Variance of all elements
pub fn var_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("var expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    if data.is_empty() {
        return Err(PulseError::RuntimeError("Cannot compute variance of empty array".into()));
    }

    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64;

    Ok(Value::Float(variance))
}

/// min(arr: Map) -> Float
/// Minimum value
pub fn min_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("min expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    if data.is_empty() {
        return Err(PulseError::RuntimeError("Cannot find min of empty array".into()));
    }

    let result = data.iter().cloned().fold(f64::INFINITY, f64::min);
    Ok(Value::Float(result))
}

/// max(arr: Map) -> Float
/// Maximum value
pub fn max_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("max expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    if data.is_empty() {
        return Err(PulseError::RuntimeError("Cannot find max of empty array".into()));
    }

    let result = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Ok(Value::Float(result))
}

/// argmin(arr: Map) -> Int
/// Index of minimum value
pub fn argmin_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("argmin expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    if data.is_empty() {
        return Err(PulseError::RuntimeError("Cannot find argmin of empty array".into()));
    }

    let min_idx = data.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    Ok(Value::Int(min_idx as i64))
}

/// argmax(arr: Map) -> Int
/// Index of maximum value
pub fn argmax_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("argmax expects 1 argument: arr".into()));
    }

    let (_, data) = extract_array_data(heap, &args[0])?;
    if data.is_empty() {
        return Err(PulseError::RuntimeError("Cannot find argmax of empty array".into()));
    }

    let max_idx = data.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    Ok(Value::Int(max_idx as i64))
}

// ============================================================================
// CONSTANTS AND UTILITIES
// ============================================================================

/// pi() -> Float
/// Returns the value of pi
pub fn pi_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("pi takes no arguments".into()));
    }
    Ok(Value::Float(PI))
}

/// e() -> Float
/// Returns the value of e
pub fn e_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("e takes no arguments".into()));
    }
    Ok(Value::Float(std::f64::consts::E))
}
