//! SQLite database native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

// Use Rc<RefCell<>> for thread-unsafe storage within a single thread
type DbConnection = Rc<RefCell<rusqlite::Connection>>;

// Database registry - uses thread-local storage
thread_local! {
    static DATABASES: RefCell<HashMap<String, DbConnection>> = RefCell::new(HashMap::new());
}

/// db_open(path: String) -> String
/// Opens or creates a SQLite database, returns a database handle
pub fn db_open_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("db_open expects 1 argument".into()));
    }

    let path = extract_string(heap, &args[0])?;
    let db_id = uuid::Uuid::new_v4().to_string();

    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| PulseError::RuntimeError(format!("Failed to open database: {}", e)))?;

    DATABASES.with(|dbs| {
        dbs.borrow_mut().insert(db_id.clone(), Rc::new(RefCell::new(conn)));
    });

    Ok(Value::Obj(heap.alloc_object(Object::String(db_id))))
}

/// db_open_memory() -> String
/// Opens an in-memory SQLite database
pub fn db_open_memory_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    let _ = args; // Unused
    let db_id = uuid::Uuid::new_v4().to_string();

    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|e| PulseError::RuntimeError(format!("Failed to create in-memory database: {}", e)))?;

    DATABASES.with(|dbs| {
        dbs.borrow_mut().insert(db_id.clone(), Rc::new(RefCell::new(conn)));
    });

    Ok(Value::Obj(heap.alloc_object(Object::String(db_id))))
}

/// db_execute(db_id: String, sql: String) -> Int
/// Executes SQL without returning rows, returns number of affected rows
pub fn db_execute_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("db_execute expects 2 arguments".into()));
    }

    let db_id = extract_string(heap, &args[0])?;
    let sql = extract_string(heap, &args[1])?;

    let conn = DATABASES.with(|dbs| {
        dbs.borrow().get(&db_id).cloned()
    });

    let conn = conn.ok_or_else(|| PulseError::RuntimeError("Invalid database handle".into()))?;

    let affected = conn.borrow_mut().execute(&sql, [])
        .map_err(|e| PulseError::RuntimeError(format!("SQL execution failed: {}", e)))?;

    Ok(Value::Int(affected as i64))
}

/// db_query(db_id: String, sql: String) -> List
/// Executes SQL query and returns results as a list of maps
pub fn db_query_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("db_query expects 2 arguments".into()));
    }

    let db_id = extract_string(heap, &args[0])?;
    let sql = extract_string(heap, &args[1])?;

    let conn = DATABASES.with(|dbs| {
        dbs.borrow().get(&db_id).cloned()
    });

    let conn = conn.ok_or_else(|| PulseError::RuntimeError("Invalid database handle".into()))?;

    let binding = conn.borrow_mut();
    let mut stmt = binding.prepare(&sql)
        .map_err(|e| PulseError::RuntimeError(format!("Failed to prepare statement: {}", e)))?;

    let column_count = stmt.column_count();
    let mut column_names = Vec::new();
    for i in 0..column_count {
        column_names.push(stmt.column_name(i).unwrap_or("?").to_string());
    }

    let mut results = Vec::new();
    let mut rows = stmt.query([])
        .map_err(|e| PulseError::RuntimeError(format!("Query failed: {}", e)))?;

    while let Some(row) = rows.next().map_err(|e| PulseError::RuntimeError(format!("Row iteration failed: {}", e)))? {
        let mut row_map = HashMap::new();
        for (i, name) in column_names.iter().enumerate() {
            let value: Value = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => Value::Unit,
                Ok(rusqlite::types::ValueRef::Integer(i)) => Value::Int(i),
                Ok(rusqlite::types::ValueRef::Real(f)) => {
                    Value::Obj(heap.alloc_object(Object::String(format!("{}", f))))
                }
                Ok(rusqlite::types::ValueRef::Text(s)) => {
                    Value::Obj(heap.alloc_object(Object::String(String::from_utf8_lossy(s).to_string())))
                }
                Ok(rusqlite::types::ValueRef::Blob(b)) => {
                    Value::Obj(heap.alloc_object(Object::String(format!("<blob: {} bytes>", b.len()))))
                }
                Err(_) => Value::Unit,
            };
            row_map.insert(name.clone(), value);
        }
        results.push(Value::Obj(heap.alloc_object(Object::Map(row_map))));
    }

    let handle = heap.alloc_object(Object::List(results));
    Ok(Value::Obj(handle))
}

/// db_close(db_id: String) -> Bool
/// Closes a database connection
pub fn db_close_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("db_close expects 1 argument".into()));
    }

    let db_id = extract_string(heap, &args[0])?;

    DATABASES.with(|dbs| {
        dbs.borrow_mut().remove(&db_id);
    });
    
    Ok(Value::Bool(true))
}

/// db_begin(db_id: String) -> Bool
/// Begins a transaction
pub fn db_begin_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("db_begin expects 1 argument".into()));
    }

    let db_id = extract_string(heap, &args[0])?;

    let conn = DATABASES.with(|dbs| {
        dbs.borrow().get(&db_id).cloned()
    });

    let conn = conn.ok_or_else(|| PulseError::RuntimeError("Invalid database handle".into()))?;

    conn.borrow_mut().execute("BEGIN", [])
        .map_err(|e| PulseError::RuntimeError(format!("Begin transaction failed: {}", e)))?;

    Ok(Value::Bool(true))
}

/// db_commit(db_id: String) -> Bool
/// Commits a transaction
pub fn db_commit_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("db_commit expects 1 argument".into()));
    }

    let db_id = extract_string(heap, &args[0])?;

    let conn = DATABASES.with(|dbs| {
        dbs.borrow().get(&db_id).cloned()
    });

    let conn = conn.ok_or_else(|| PulseError::RuntimeError("Invalid database handle".into()))?;

    conn.borrow_mut().execute("COMMIT", [])
        .map_err(|e| PulseError::RuntimeError(format!("Commit transaction failed: {}", e)))?;

    Ok(Value::Bool(true))
}

/// db_rollback(db_id: String) -> Bool
/// Rolls back a transaction
pub fn db_rollback_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("db_rollback expects 1 argument".into()));
    }

    let db_id = extract_string(heap, &args[0])?;

    let conn = DATABASES.with(|dbs| {
        dbs.borrow().get(&db_id).cloned()
    });

    let conn = conn.ok_or_else(|| PulseError::RuntimeError("Invalid database handle".into()))?;

    conn.borrow_mut().execute("ROLLBACK", [])
        .map_err(|e| PulseError::RuntimeError(format!("Rollback transaction failed: {}", e)))?;

    Ok(Value::Bool(true))
}

/// db_tables(db_id: String) -> List
/// Returns list of tables in the database
pub fn db_tables_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("db_tables expects 1 argument".into()));
    }

    let db_id = extract_string(heap, &args[0])?;

    let conn = DATABASES.with(|dbs| {
        dbs.borrow().get(&db_id).cloned()
    });

    let conn = conn.ok_or_else(|| PulseError::RuntimeError("Invalid database handle".into()))?;

    let binding = conn.borrow_mut();
    let mut stmt = binding
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .map_err(|e| PulseError::RuntimeError(format!("Failed to query tables: {}", e)))?;

    let mut results = Vec::new();
    let mut rows = stmt.query([]).map_err(|e| PulseError::RuntimeError(format!("Query failed: {}", e)))?;

    while let Some(row) = rows.next().map_err(|e| PulseError::RuntimeError(format!("Row iteration failed: {}", e)))? {
        if let Ok(name) = row.get::<_, String>(0) {
            results.push(Value::Obj(heap.alloc_object(Object::String(name))));
        }
    }

    Ok(Value::Obj(heap.alloc_object(Object::List(results))))
}

// Helper functions
fn extract_string(heap: &dyn HeapInterface, value: &Value) -> Result<String, PulseError> {
    match value {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                Ok(s.clone())
            } else {
                Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() })
            }
        }
        _ => Err(PulseError::TypeMismatch { expected: "string".into(), got: value.type_name() }),
    }
}
