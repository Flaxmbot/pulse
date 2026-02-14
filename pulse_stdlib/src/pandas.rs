//! Pandas-like DataFrame library for Pulse
//! 
//! Provides DataFrame creation, operations, and statistics

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use csv::ReaderBuilder;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn extract_string(heap: &dyn HeapInterface, value: &Value) -> PulseResult<String> {
    match value {
        Value::Obj(handle) => {
            if let Some(Object::String(s)) = heap.get_object(*handle) {
                Ok(s.clone())
            } else {
                Err(PulseError::RuntimeError("Expected string".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected string".into())),
    }
}

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

fn extract_bool(heap: &dyn HeapInterface, value: &Value) -> PulseResult<bool> {
    match value {
        Value::Bool(b) => Ok(*b),
        Value::Obj(handle) => {
            if let Some(Object::String(s)) = heap.get_object(*handle) {
                match s.as_str() {
                    "true" | "1" | "yes" => Ok(true),
                    "false" | "0" | "no" => Ok(false),
                    _ => Err(PulseError::RuntimeError("Cannot parse string as boolean".into())),
                }
            } else {
                Err(PulseError::RuntimeError("Expected boolean value".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected boolean value".into())),
    }
}

fn extract_columns(heap: &dyn HeapInterface, value: &Value) -> PulseResult<Vec<String>> {
    match value {
        Value::Obj(handle) => {
            if let Some(Object::List(list)) = heap.get_object(*handle) {
                list.iter().map(|v| extract_string(heap, v)).collect()
            } else {
                Err(PulseError::RuntimeError("Expected list of column names".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected list of column names".into())),
    }
}

fn extract_data(heap: &dyn HeapInterface, value: &Value) -> PulseResult<Vec<Vec<Value>>> {
    match value {
        Value::Obj(handle) => {
            if let Some(Object::List(rows)) = heap.get_object(*handle) {
                let mut data = Vec::new();
                for row in rows {
                    match row {
                        Value::Obj(row_handle) => {
                            if let Some(Object::List(row_values)) = heap.get_object(*row_handle) {
                                data.push(row_values.clone());
                            } else {
                                return Err(PulseError::RuntimeError("Expected list of row values".into()));
                            }
                        }
                        _ => return Err(PulseError::RuntimeError("Expected list of row values".into())),
                    }
                }
                Ok(data)
            } else {
                Err(PulseError::RuntimeError("Expected list of rows".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected list of rows".into())),
    }
}

fn extract_list_of_maps(heap: &dyn HeapInterface, value: &Value) -> PulseResult<Vec<HashMap<String, Value>>> {
    match value {
        Value::Obj(handle) => {
            if let Some(Object::List(list)) = heap.get_object(*handle) {
                let mut maps = Vec::new();
                for item in list {
                    match item {
                        Value::Obj(map_handle) => {
                            if let Some(Object::Map(map)) = heap.get_object(*map_handle) {
                                let mut string_map = HashMap::new();
                                for (k, v) in map {
                                    string_map.insert(k.clone(), v.clone());
                                }
                                maps.push(string_map);
                            } else {
                                return Err(PulseError::RuntimeError("Expected map in list".into()));
                            }
                        }
                        _ => return Err(PulseError::RuntimeError("Expected map in list".into())),
                    }
                }
                Ok(maps)
            } else {
                Err(PulseError::RuntimeError("Expected list".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected list".into())),
    }
}

fn get_df_columns(heap: &dyn HeapInterface, df: &HashMap<String, Value>) -> PulseResult<Vec<String>> {
    match df.get("columns") {
        Some(Value::Obj(handle)) => {
            if let Some(Object::List(list)) = heap.get_object(*handle) {
                list.iter().map(|v| extract_string(heap, v)).collect()
            } else {
                Err(PulseError::RuntimeError("Columns should be a list".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("DataFrame missing columns".into())),
    }
}

fn get_df_data(heap: &dyn HeapInterface, df: &HashMap<String, Value>) -> PulseResult<Vec<Vec<Value>>> {
    match df.get("data") {
        Some(Value::Obj(handle)) => extract_data(heap, &Value::Obj(*handle)),
        _ => Err(PulseError::RuntimeError("DataFrame missing data".into())),
    }
}

fn create_dataframe(heap: &mut dyn HeapInterface, columns: Vec<String>, data: Vec<Vec<Value>>) -> Value {
    let mut map = HashMap::new();
    let col_list: Vec<Value> = columns.iter()
        .map(|s| Value::Obj(heap.alloc_object(Object::String(s.clone()))))
        .collect();
    map.insert("columns".to_string(), Value::Obj(heap.alloc_object(Object::List(col_list))));
    let data_list: Vec<Value> = data.iter()
        .map(|row| Value::Obj(heap.alloc_object(Object::List(row.clone()))))
        .collect();
    map.insert("data".to_string(), Value::Obj(heap.alloc_object(Object::List(data_list))));
    let shape = vec![data.len() as i64, columns.len() as i64];
    let shape_list: Vec<Value> = shape.into_iter().map(|i| Value::Int(i)).collect();
    map.insert("shape".to_string(), Value::Obj(heap.alloc_object(Object::List(shape_list))));
    let index: Vec<Value> = (0..data.len()).map(|i| Value::Int(i as i64)).collect();
    map.insert("index".to_string(), Value::Obj(heap.alloc_object(Object::List(index))));
    Value::Obj(heap.alloc_object(Object::Map(map)))
}

// ============================================================================
// DATAFRAME CREATION FUNCTIONS
// ============================================================================

pub fn df_create_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("df_create expects 2 arguments: columns and data".into()));
    }
    let columns = extract_columns(heap, &args[0])?;
    let data = extract_data(heap, &args[1])?;
    for (i, row) in data.iter().enumerate() {
        if row.len() != columns.len() {
            return Err(PulseError::RuntimeError(format!("Row {} has {} columns, expected {}", i, row.len(), columns.len()).into()));
        }
    }
    Ok(create_dataframe(heap, columns, data))
}

pub fn df_from_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("df_from_list expects 1 argument: list of maps".into()));
    }
    let maps = extract_list_of_maps(heap, &args[0])?;
    if maps.is_empty() {
        return Ok(create_dataframe(heap, vec![], vec![]));
    }
    let mut columns: Vec<String> = maps[0].keys().cloned().collect();
    columns.sort();
    let data: Vec<Vec<Value>> = maps.iter()
        .map(|m| columns.iter().map(|col| m.get(col).cloned().unwrap_or(Value::Unit)).collect())
        .collect();
    Ok(create_dataframe(heap, columns, data))
}

pub fn df_from_csv_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("df_from_csv expects 1 argument: path".into()));
    }
    let path = extract_string(heap, &args[0])?;
    let file = File::open(&path).map_err(|e| PulseError::RuntimeError(format!("Cannot open file {}: {}", path, e).into()))?;
    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new().has_headers(true).flexible(true).from_reader(reader);
    let headers: Vec<String> = csv_reader.headers()
        .map_err(|e| PulseError::RuntimeError(format!("Cannot read headers: {}", e).into()))?
        .iter().map(|s| s.to_string()).collect();
    let mut data: Vec<Vec<Value>> = Vec::new();
    for result in csv_reader.records() {
        let record = result.map_err(|e| PulseError::RuntimeError(format!("Cannot read record: {}", e).into()))?;
        let row: Vec<Value> = record.iter().map(|s| {
            if let Ok(i) = s.parse::<i64>() { Value::Int(i) }
            else if let Ok(f) = s.parse::<f64>() { Value::Float(f) }
            else if s.is_empty() { Value::Unit }
            else { Value::Obj(heap.alloc_object(Object::String(s.to_string()))) }
        }).collect();
        data.push(row);
    }
    Ok(create_dataframe(heap, headers, data))
}

pub fn df_from_json_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("df_from_json expects 1 argument: json string".into()));
    }
    let json_str = extract_string(heap, &args[0])?;
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| PulseError::RuntimeError(format!("Invalid JSON: {}", e).into()))?;
    match json_value {
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return Ok(create_dataframe(heap, vec![], vec![]));
            }
            let mut columns: Vec<String> = Vec::new();
            for obj in &arr {
                if let serde_json::Value::Object(map) = obj {
                    for key in map.keys() {
                        if !columns.contains(key) { columns.push(key.clone()); }
                    }
                }
            }
            columns.sort();
            let mut data: Vec<Vec<Value>> = Vec::new();
            for item in arr {
                if let serde_json::Value::Object(map) = item {
                    let row: Vec<Value> = columns.iter().map(|col| {
                        match map.get(col) {
                            Some(serde_json::Value::Null) => Value::Unit,
                            Some(serde_json::Value::Bool(b)) => Value::Bool(*b),
                            Some(serde_json::Value::Number(n)) => {
                                if let Some(i) = n.as_i64() { Value::Int(i) }
                                else if let Some(f) = n.as_f64() { Value::Float(f) }
                                else { Value::Obj(heap.alloc_object(Object::String(n.to_string()))) }
                            }
                            Some(serde_json::Value::String(s)) => {
                                if let Ok(i) = s.parse::<i64>() { Value::Int(i) }
                                else if let Ok(f) = s.parse::<f64>() { Value::Float(f) }
                                else { Value::Obj(heap.alloc_object(Object::String(s.clone()))) }
                            }
                            Some(serde_json::Value::Array(arr)) => {
                                let list: Vec<Value> = arr.iter().map(|v| match v {
                                    serde_json::Value::Number(n) => {
                                        if let Some(i) = n.as_i64() { Value::Int(i) }
                                        else if let Some(f) = n.as_f64() { Value::Float(f) }
                                        else { Value::Unit }
                                    }
                                    serde_json::Value::String(s) => Value::Obj(heap.alloc_object(Object::String(s.clone()))),
                                    _ => Value::Unit,
                                }).collect();
                                Value::Obj(heap.alloc_object(Object::List(list)))
                            }
                            Some(serde_json::Value::Object(_)) => Value::Unit,
                            None => Value::Unit,
                        }
                    }).collect();
                    data.push(row);
                }
            }
            Ok(create_dataframe(heap, columns, data))
        }
        _ => Err(PulseError::RuntimeError("JSON must be an array of objects".into())),
    }
}

// ============================================================================
// DATAFRAME OPERATION FUNCTIONS
// ============================================================================

pub fn df_columns_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("df_columns expects 1 argument: dataframe".into()));
    }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?;
                let col_list: Vec<Value> = columns.iter()
                    .map(|s| Value::Obj(heap.alloc_object(Object::String(s.clone()))))
                    .collect();
                Ok(Value::Obj(heap.alloc_object(Object::List(col_list))))
            } else {
                Err(PulseError::RuntimeError("Expected DataFrame".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_shape_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("df_shape expects 1 argument: dataframe".into()));
    }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?;
                let data = get_df_data(heap, map)?;
                let shape = vec![Value::Int(data.len() as i64), Value::Int(columns.len() as i64)];
                Ok(Value::Obj(heap.alloc_object(Object::List(shape))))
            } else {
                Err(PulseError::RuntimeError("Expected DataFrame".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_head_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("df_head expects 2 arguments: dataframe and n".into()));
    }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?;
                let data = get_df_data(heap, map)?;
                let n = extract_int(heap, &args[1])? as usize;
                let head_data: Vec<Vec<Value>> = data.into_iter().take(n).collect();
                Ok(create_dataframe(heap, columns, head_data))
            } else {
                Err(PulseError::RuntimeError("Expected DataFrame".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_tail_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("df_tail expects 2 arguments: dataframe and n".into()));
    }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?;
                let data = get_df_data(heap, map)?;
                let n = extract_int(heap, &args[1])? as usize;
                let tail_data: Vec<Vec<Value>> = data.into_iter().rev().take(n).rev().collect();
                Ok(create_dataframe(heap, columns, tail_data))
            } else {
                Err(PulseError::RuntimeError("Expected DataFrame".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_select_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("df_select expects 2 arguments: dataframe and columns".into()));
    }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let all_columns = get_df_columns(heap, map)?;
                let data = get_df_data(heap, map)?;
                let selected_cols = extract_columns(heap, &args[1])?;
                let indices: Vec<usize> = selected_cols.iter()
                    .map(|col| all_columns.iter().position(|c| c == col)
                        .ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found", col).into())))
                    .collect::<Result<Vec<_>, _>>()?;
                let selected_data: Vec<Vec<Value>> = data.iter()
                    .map(|row| indices.iter().map(|&i| row[i].clone()).collect())
                    .collect();
                Ok(create_dataframe(heap, selected_cols, selected_data))
            } else {
                Err(PulseError::RuntimeError("Expected DataFrame".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_filter_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("df_filter expects 2 arguments: dataframe and condition".into()));
    }
    match (&args[0], &args[1]) {
        (Value::Obj(df_handle), Value::Obj(cond_handle)) => {
            if let (Some(Object::Map(df_map)), Some(Object::Map(cond_map))) = (heap.get_object(*df_handle), heap.get_object(*cond_handle)) {
                let columns = get_df_columns(heap, df_map)?;
                let data = get_df_data(heap, df_map)?;
                let col_name = match cond_map.get("column") {
                    Some(Value::Obj(h)) => extract_string(heap, &Value::Obj(*h))?,
                    _ => return Err(PulseError::RuntimeError("Condition missing column".into())),
                };
                let operator = match cond_map.get("operator") {
                    Some(Value::Obj(h)) => extract_string(heap, &Value::Obj(*h))?,
                    _ => return Err(PulseError::RuntimeError("Condition missing operator".into())),
                };
                let value = match cond_map.get("value") { Some(v) => v.clone(), _ => return Err(PulseError::RuntimeError("Condition missing value".into())) };
                let col_idx = columns.iter().position(|c| c == &col_name)
                    .ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found", col_name).into()))?;
                let filtered_data: Vec<Vec<Value>> = data.into_iter().filter(|row| {
                    let cell_value = &row[col_idx];
                    match operator.as_str() {
                        "==" => cell_value == &value,
                        "!=" => cell_value != &value,
                        ">" => extract_float(heap, cell_value).and_then(|a| extract_float(heap, &value).map(|b| a > b)).unwrap_or(false),
                        "<" => extract_float(heap, cell_value).and_then(|a| extract_float(heap, &value).map(|b| a < b)).unwrap_or(false),
                        ">=" => extract_float(heap, cell_value).and_then(|a| extract_float(heap, &value).map(|b| a >= b)).unwrap_or(false),
                        "<=" => extract_float(heap, cell_value).and_then(|a| extract_float(heap, &value).map(|b| a <= b)).unwrap_or(false),
                        _ => false,
                    }
                }).collect();
                Ok(create_dataframe(heap, columns, filtered_data))
            } else {
                Err(PulseError::RuntimeError("Expected DataFrame and condition".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame and condition".into())),
    }
}

pub fn df_sort_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("df_sort expects 3 arguments: dataframe, column, ascending".into()));
    }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?;
                let data = get_df_data(heap, map)?;
                let sort_col = extract_string(heap, &args[1])?;
                let ascending = extract_bool(heap, &args[2])?;
                let col_idx = columns.iter().position(|c| c == &sort_col)
                    .ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found", sort_col).into()))?;
                let mut sorted_data = data;
                sorted_data.sort_by(|a, b| {
                    let (a_val, b_val) = (&a[col_idx], &b[col_idx]);
                    if let (Ok(a_num), Ok(b_num)) = (extract_float(heap, a_val), extract_float(heap, b_val)) {
                        if ascending { a_num.partial_cmp(&b_num).unwrap_or(std::cmp::Ordering::Equal) }
                        else { b_num.partial_cmp(&a_num).unwrap_or(std::cmp::Ordering::Equal) }
                    } else {
                        let a_str = format!("{:?}", a_val);
                        let b_str = format!("{:?}", b_val);
                        if ascending { a_str.cmp(&b_str) } else { b_str.cmp(&a_str) }
                    }
                });
                Ok(create_dataframe(heap, columns, sorted_data))
            } else {
                Err(PulseError::RuntimeError("Expected DataFrame".into()))
            }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

// ============================================================================
// DATA OPERATIONS
// ============================================================================

pub fn df_group_by_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 { return Err(PulseError::RuntimeError("df_group_by expects 2 arguments".into())); }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?;
                let data = get_df_data(heap, map)?;
                let group_col = extract_string(heap, &args[1])?;
                let col_idx = columns.iter().position(|c| c == &group_col)
                    .ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found", group_col).into()))?;
                let mut groups: HashMap<String, Vec<Vec<Value>>> = HashMap::new();
                for row in data { let key = format!("{:?}", row[col_idx]); groups.entry(key).or_insert_with(Vec::new).push(row); }
                let mut result_map = HashMap::new();
                for (key, rows) in groups { let group_df = create_dataframe(heap, columns.clone(), rows); result_map.insert(key, group_df); }
                Ok(Value::Obj(heap.alloc_object(Object::Map(result_map))))
            } else { Err(PulseError::RuntimeError("Expected DataFrame".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_aggregate_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 { return Err(PulseError::RuntimeError("df_aggregate expects 3 arguments".into())); }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?;
                let data = get_df_data(heap, map)?;
                let agg_col = extract_string(heap, &args[1])?;
                let operation = extract_string(heap, &args[2])?;
                let col_idx = columns.iter().position(|c| c == &agg_col)
                    .ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found", agg_col).into()))?;
                let values: Vec<f64> = data.iter().filter_map(|row| extract_float(heap, &row[col_idx]).ok()).collect();
                if values.is_empty() { return Ok(Value::Unit); }
                let result = match operation.as_str() {
                    "sum" => Value::Float(values.iter().sum()),
                    "mean" | "avg" => Value::Float(values.iter().sum::<f64>() / values.len() as f64),
                    "count" => Value::Int(values.len() as i64),
                    "min" => Value::Float(values.iter().cloned().fold(f64::INFINITY, f64::min)),
                    "max" => Value::Float(values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
                    "std" => { let mean = values.iter().sum::<f64>() / values.len() as f64; let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64; Value::Float(variance.sqrt()) }
                    _ => return Err(PulseError::RuntimeError(format!("Unknown operation: {}", operation).into())),
                };
                Ok(result)
            } else { Err(PulseError::RuntimeError("Expected DataFrame".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_join_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 { return Err(PulseError::RuntimeError("df_join expects 3 arguments".into())); }
    match (&args[0], &args[1]) {
        (Value::Obj(h1), Value::Obj(h2)) => {
            if let (Some(Object::Map(m1)), Some(Object::Map(m2))) = (heap.get_object(*h1), heap.get_object(*h2)) {
                let cols1 = get_df_columns(heap, m1)?; let data1 = get_df_data(heap, m1)?;
                let cols2 = get_df_columns(heap, m2)?; let data2 = get_df_data(heap, m2)?;
                let join_col = extract_string(heap, &args[2])?;
                let idx1 = cols1.iter().position(|c| c == &join_col).ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found in df1", join_col).into()))?;
                let idx2 = cols2.iter().position(|c| c == &join_col).ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found in df2", join_col).into()))?;
                let mut lookup: HashMap<String, Vec<Vec<Value>>> = HashMap::new();
                for row in &data2 { let key = format!("{:?}", row[idx2]); lookup.entry(key).or_insert_with(Vec::new).push(row.clone()); }
                let mut result_data = Vec::new();
                for row1 in &data1 {
                    let key = format!("{:?}", row1[idx1]);
                    if let Some(matching_rows) = lookup.get(&key) {
                        for row2 in matching_rows {
                            let mut joined_row = row1.clone();
                            for (i, _col) in cols2.iter().enumerate() { if i != idx2 { joined_row.push(row2[i].clone()); } }
                            result_data.push(joined_row);
                        }
                    }
                }
                let mut result_cols = cols1.clone();
                for col in &cols2 { if col != &join_col { result_cols.push(col.clone()); } }
                Ok(create_dataframe(heap, result_cols, result_data))
            } else { Err(PulseError::RuntimeError("Expected DataFrames".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrames".into())),
    }
}

pub fn df_concat_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 { return Err(PulseError::RuntimeError("df_concat expects 1 argument".into())); }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(list)) = heap.get_object(*handle) {
                if list.is_empty() { return Ok(create_dataframe(heap, vec![], vec![])); }
                let first_df = match &list[0] { Value::Obj(h) => heap.get_object(*h), _ => return Err(PulseError::RuntimeError("Expected DataFrame".into())) };
                let (columns, mut all_data) = match first_df {
                    Some(Object::Map(m)) => { let cols = get_df_columns(heap, m)?; let data = get_df_data(heap, m)?; (cols, data) }
                    _ => return Err(PulseError::RuntimeError("Expected DataFrame".into())),
                };
                for item in list.iter().skip(1) {
                    if let Value::Obj(h) = item { if let Some(Object::Map(m)) = heap.get_object(*h) { let data = get_df_data(heap, m)?; all_data.extend(data); } }
                }
                Ok(create_dataframe(heap, columns, all_data))
            } else { Err(PulseError::RuntimeError("Expected list of DataFrames".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected list".into())),
    }
}

// ============================================================================
// COLUMN OPERATIONS
// ============================================================================

pub fn df_add_column_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 { return Err(PulseError::RuntimeError("df_add_column expects 3 arguments".into())); }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let mut columns = get_df_columns(heap, map)?; let mut data = get_df_data(heap, map)?;
                let new_col = extract_string(heap, &args[1])?;
                let new_values = match &args[2] { Value::Obj(h) => if let Some(Object::List(list)) = heap.get_object(*h) { list.clone() } else { return Err(PulseError::RuntimeError("Values must be a list".into())) }, _ => return Err(PulseError::RuntimeError("Values must be a list".into())) };
                if new_values.len() != data.len() { return Err(PulseError::RuntimeError("Values length mismatch".into())); }
                for (i, row) in data.iter_mut().enumerate() { row.push(new_values[i].clone()); }
                columns.push(new_col);
                Ok(create_dataframe(heap, columns, data))
            } else { Err(PulseError::RuntimeError("Expected DataFrame".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_drop_column_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 { return Err(PulseError::RuntimeError("df_drop_column expects 2 arguments".into())); }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let mut columns = get_df_columns(heap, map)?; let data = get_df_data(heap, map)?;
                let drop_col = extract_string(heap, &args[1])?;
                let col_idx = columns.iter().position(|c| c == &drop_col).ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found", drop_col).into()))?;
                columns.remove(col_idx);
                let new_data: Vec<Vec<Value>> = data.iter().map(|row| row.iter().enumerate().filter(|(i, _)| *i != col_idx).map(|(_, v)| v.clone()).collect()).collect();
                Ok(create_dataframe(heap, columns, new_data))
            } else { Err(PulseError::RuntimeError("Expected DataFrame".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_rename_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 { return Err(PulseError::RuntimeError("df_rename expects 3 arguments".into())); }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let mut columns = get_df_columns(heap, map)?; let data = get_df_data(heap, map)?;
                let old_name = extract_string(heap, &args[1])?; let new_name = extract_string(heap, &args[2])?;
                let col_idx = columns.iter().position(|c| c == &old_name).ok_or_else(|| PulseError::RuntimeError(format!("Column {} not found", old_name).into()))?;
                columns[col_idx] = new_name;
                Ok(create_dataframe(heap, columns, data))
            } else { Err(PulseError::RuntimeError("Expected DataFrame".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

// ============================================================================
// STATISTICS
// ============================================================================

pub fn df_describe_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 { return Err(PulseError::RuntimeError("df_describe expects 1 argument".into())); }
    match &args[0] {
        Value::Obj(handle) => {
            if let Some(Object::Map(map)) = heap.get_object(*handle) {
                let columns = get_df_columns(heap, map)?; let data = get_df_data(heap, map)?;
                let numeric_cols: Vec<(usize, String)> = columns.iter().enumerate()
                    .filter(|(i, _n)| data.iter().all(|row| extract_float(heap, &row[*i]).is_ok()))
                    .map(|(i, n)| (i, n.clone())).collect();
                if numeric_cols.is_empty() { 
                    let err_str = heap.alloc_object(Object::String("No numeric columns found".to_string()));
                    return Ok(create_dataframe(heap, vec!["message".to_string()], vec![vec![Value::Obj(err_str)]])); 
                }
                let summary_cols = vec!["column".to_string(), "statistic".to_string(), "value".to_string()];
                let mut summary_data: Vec<Vec<Value>> = Vec::new();
                for (col_idx, col_name) in &numeric_cols {
                    let values: Vec<f64> = data.iter().filter_map(|row| extract_float(heap, &row[*col_idx]).ok()).collect();
                    if values.is_empty() { continue; }
                    let count = values.len() as f64; let sum: f64 = values.iter().sum(); let mean = sum / count;
                    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count; let std = variance.sqrt();
                    let mut sorted = values.clone(); sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let p25 = sorted[(count * 0.25).min(count - 1.0) as usize];
                    let p50 = sorted[(count * 0.50).min(count - 1.0) as usize];
                    let p75 = sorted[(count * 0.75).min(count - 1.0) as usize];
                    let stats = vec![("count", count), ("mean", mean), ("std", std), ("min", min), ("25%", p25), ("50%", p50), ("75%", p75), ("max", max)];
                    for (stat_name, stat_value) in stats {
                        summary_data.push(vec![Value::Obj(heap.alloc_object(Object::String(col_name.to_string()))), Value::Obj(heap.alloc_object(Object::String(stat_name.to_string()))), Value::Float(stat_value)]);
                    }
                }
                Ok(create_dataframe(heap, summary_cols, summary_data))
            } else { Err(PulseError::RuntimeError("Expected DataFrame".into())) }
        }
        _ => Err(PulseError::RuntimeError("Expected DataFrame".into())),
    }
}

pub fn df_corr_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 { return Err(PulseError::RuntimeError("df_corr expects 1 argument".into())); }
    let df_map = match &args[0] { Value::Obj(handle) => heap.get_object(*handle), _ => return Err(PulseError::RuntimeError("Expected DataFrame".into())) };
    let map = match df_map { Some(Object::Map(m)) => m, _ => return Err(PulseError::RuntimeError("Expected DataFrame".into())) };
    let columns = get_df_columns(heap, map)?; let data = get_df_data(heap, map)?;
    let numeric_indices: Vec<usize> = (0..columns.len()).filter(|&i| data.iter().all(|row| extract_float(heap, &row[i]).is_ok())).collect();
    let err_msg = heap.alloc_object(Object::String("Need at least 2 numeric columns".to_string()));
    if numeric_indices.len() < 2 { return Ok(create_dataframe(heap, vec!["message".to_string()], vec![vec![Value::Obj(err_msg)]])); }
    let numeric_cols: Vec<Vec<f64>> = numeric_indices.iter().map(|&i| data.iter().filter_map(|row| extract_float(heap, &row[i]).ok()).collect()).collect();
    let n = numeric_cols[0].len(); let num_numeric = numeric_indices.len();
    let mut corr_data: Vec<Vec<Value>> = Vec::new();
    for i in 0..num_numeric {
        for j in 0..num_numeric {
            let col_i = &numeric_cols[i]; let col_j = &numeric_cols[j];
            let mean_i: f64 = col_i.iter().sum::<f64>() / n as f64; let mean_j: f64 = col_j.iter().sum::<f64>() / n as f64;
            let mut cov = 0.0; let mut var_i = 0.0; let mut var_j = 0.0;
            for k in 0..n { let diff_i = col_i[k] - mean_i; let diff_j = col_j[k] - mean_j; cov += diff_i * diff_j; var_i += diff_i * diff_i; var_j += diff_j * diff_j; }
            let corr = if var_i > 0.0 && var_j > 0.0 { cov / (var_i.sqrt() * var_j.sqrt()) } else { 0.0 };
            corr_data.push(vec![Value::Obj(heap.alloc_object(Object::String(columns[numeric_indices[i]].clone()))), Value::Obj(heap.alloc_object(Object::String(columns[numeric_indices[j]].clone()))), Value::Float(corr)]);
        }
    }
    let corr_cols = vec!["column1".to_string(), "column2".to_string(), "correlation".to_string()];
    Ok(create_dataframe(heap, corr_cols, corr_data))
}
