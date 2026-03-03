//! Time and DateTime native functions

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Timelike};
use futures::FutureExt;
use pulse_ast::object::{HeapInterface, Object};
use pulse_ast::{PulseError, PulseResult, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// current_timestamp() -> Float
/// Returns the current Unix timestamp in seconds (with microsecond precision)
pub fn current_timestamp_native(
    _heap: &mut dyn HeapInterface,
    args: &[Value],
) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError(
            "current_timestamp expects 0 arguments".into(),
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    Ok(Value::Float(now.as_secs_f64()))
}

/// current_timestamp_millis() -> Int
/// Returns the current Unix timestamp in milliseconds
pub fn current_timestamp_millis_native(
    _heap: &mut dyn HeapInterface,
    args: &[Value],
) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError(
            "current_timestamp_millis expects 0 arguments".into(),
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    Ok(Value::Int(now.as_millis() as i64))
}

/// current_timestamp_micros() -> Int
/// Returns the current Unix timestamp in microseconds
pub fn current_timestamp_micros_native(
    _heap: &mut dyn HeapInterface,
    args: &[Value],
) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError(
            "current_timestamp_micros expects 0 arguments".into(),
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    Ok(Value::Int(now.as_micros() as i64))
}

/// sleep(ms: Int) -> Unit
/// Sleeps for the specified number of milliseconds
pub fn sleep_native<'a>(
    _heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("sleep expects 1 argument".into()));
        }

        let ms = args[0].as_int()?;
        if ms < 0 {
            return Err(PulseError::RuntimeError(
                "sleep: duration must be non-negative".into(),
            ));
        }

        tokio::time::sleep(Duration::from_millis(ms as u64)).await;
        Ok(Value::Unit)
    }
    .boxed()
}

/// sleep_seconds(seconds: Float) -> Unit
/// Sleeps for the specified number of seconds (supports fractional seconds)
pub fn sleep_seconds_native<'a>(
    _heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "sleep_seconds expects 1 argument".into(),
            ));
        }

        let secs = match &args[0] {
            Value::Float(f) => *f,
            Value::Int(i) => *i as f64,
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "int or float".into(),
                    got: args[0].type_name(),
                })
            }
        };

        if secs < 0.0 {
            return Err(PulseError::RuntimeError(
                "sleep_seconds: duration must be non-negative".into(),
            ));
        }

        tokio::time::sleep(Duration::from_secs_f64(secs)).await;
        Ok(Value::Unit)
    }
    .boxed()
}

/// now() -> Map
/// Returns current datetime as a map with year, month, day, hour, minute, second, millisecond
pub fn now_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("now expects 0 arguments".into()));
    }

    let now = Local::now();
    let mut map: HashMap<String, Value> = HashMap::new();

    map.insert("year".to_string(), Value::Int(now.year() as i64));
    map.insert("month".to_string(), Value::Int(now.month() as i64));
    map.insert("day".to_string(), Value::Int(now.day() as i64));
    map.insert("hour".to_string(), Value::Int(now.hour() as i64));
    map.insert("minute".to_string(), Value::Int(now.minute() as i64));
    map.insert("second".to_string(), Value::Int(now.second() as i64));
    map.insert(
        "millisecond".to_string(),
        Value::Int(now.timestamp_subsec_millis() as i64),
    );

    let weekday_str = match now.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    };
    map.insert(
        "weekday".to_string(),
        Value::Obj(heap.alloc_object(Object::String(weekday_str.to_string()))),
    );

    let map_handle = heap.alloc_object(Object::Map(map));
    Ok(Value::Obj(map_handle))
}

/// format_time(timestamp: Float, format: String) -> String
/// Formats a Unix timestamp using the given format string
pub fn format_time_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "format_time expects 2 arguments".into(),
        ));
    }

    let timestamp = match &args[0] {
        Value::Float(f) => *f,
        Value::Int(i) => *i as f64,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    let format_str = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "object".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[1].type_name(),
            })
        }
    };

    let secs = timestamp as i64;
    let naive = match DateTime::from_timestamp(secs, 0) {
        Some(dt) => dt.naive_utc(),
        None => return Err(PulseError::RuntimeError("Invalid timestamp".into())),
    };
    let datetime: DateTime<Local> =
        DateTime::from_naive_utc_and_offset(naive, *Local::now().offset());

    let formatted = parse_format(&datetime, &format_str);
    let result_handle = heap.alloc_object(Object::String(formatted));
    Ok(Value::Obj(result_handle))
}

/// parse_format parses a simple format string
fn parse_format(dt: &DateTime<Local>, format: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() {
            match chars[i + 1] {
                'Y' => result.push_str(&format!("{:04}", dt.year())),
                'm' => result.push_str(&format!("{:02}", dt.month())),
                'd' => result.push_str(&format!("{:02}", dt.day())),
                'H' => result.push_str(&format!("{:02}", dt.hour())),
                'M' => result.push_str(&format!("{:02}", dt.minute())),
                'S' => result.push_str(&format!("{:02}", dt.second())),
                'f' => result.push_str(&format!("{:03}", dt.timestamp_subsec_millis())),
                'w' => result.push_str(match dt.weekday() {
                    chrono::Weekday::Mon => "Monday",
                    chrono::Weekday::Tue => "Tuesday",
                    chrono::Weekday::Wed => "Wednesday",
                    chrono::Weekday::Thu => "Thursday",
                    chrono::Weekday::Fri => "Friday",
                    chrono::Weekday::Sat => "Saturday",
                    chrono::Weekday::Sun => "Sunday",
                }),
                '%' => result.push('%'),
                _ => {
                    result.push('%');
                    result.push(chars[i + 1]);
                }
            }
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// parse_time(time_str: String, format: String) -> Float
/// Parses a time string using the given format and returns Unix timestamp
pub fn parse_time_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "parse_time expects 2 arguments".into(),
        ));
    }

    let time_str = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "object".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name(),
            })
        }
    };

    let format_str = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "object".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[1].type_name(),
            })
        }
    };

    let naive = parse_time_parsed(&time_str, &format_str)?;
    let timestamp = naive.and_utc().timestamp() as f64;
    Ok(Value::Float(timestamp))
}

/// Simple parser for common time formats
fn parse_time_parsed(time_str: &str, format: &str) -> PulseResult<NaiveDateTime> {
    if format == "%Y-%m-%d %H:%M:%S" {
        return NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| PulseError::RuntimeError(format!("Failed to parse time: {}", e)));
    }
    if format == "%Y-%m-%d" {
        let date = NaiveDate::parse_from_str(time_str, "%Y-%m-%d")
            .map_err(|e| PulseError::RuntimeError(format!("Failed to parse date: {}", e)))?;
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }
    if format == "%H:%M:%S" {
        let time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S")
            .map_err(|e| PulseError::RuntimeError(format!("Failed to parse time: {}", e)))?;
        let today = chrono::Local::now().date_naive();
        return Ok(today
            .and_hms_opt(time.hour(), time.minute(), time.second())
            .unwrap());
    }

    Err(PulseError::RuntimeError(format!(
        "Unsupported format: {}",
        format
    )))
}

/// duration_create(seconds: Float) -> Map
/// Creates a duration object from seconds
pub fn duration_create_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "duration_create expects 1 argument".into(),
        ));
    }

    let secs = match &args[0] {
        Value::Float(f) => *f,
        Value::Int(i) => *i as f64,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    let duration = Duration::from_secs_f64(secs);
    let mut map: HashMap<String, Value> = HashMap::new();
    map.insert("seconds".to_string(), Value::Float(duration.as_secs_f64()));
    map.insert(
        "milliseconds".to_string(),
        Value::Int(duration.as_millis() as i64),
    );
    map.insert(
        "microseconds".to_string(),
        Value::Int(duration.as_micros() as i64),
    );

    let map_handle = heap.alloc_object(Object::Map(map));
    Ok(Value::Obj(map_handle))
}

/// duration_add(dur1: Map, dur2: Map) -> Map
/// Adds two duration objects
pub fn duration_add_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "duration_add expects 2 arguments".into(),
        ));
    }

    let get_seconds = |arg: &Value| -> PulseResult<f64> {
        match arg {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as f64),
            Value::Obj(h) => {
                if let Some(Object::Map(m)) = heap.get_object(*h) {
                    if let Some(v) = m.get("seconds") {
                        match v {
                            Value::Float(f) => Ok(*f),
                            Value::Int(i) => Ok(*i as f64),
                            _ => Err(PulseError::RuntimeError(
                                "Duration must have seconds field".into(),
                            )),
                        }
                    } else {
                        Err(PulseError::RuntimeError(
                            "Duration must have seconds field".into(),
                        ))
                    }
                } else {
                    Err(PulseError::TypeMismatch {
                        expected: "map".into(),
                        got: "object".into(),
                    })
                }
            }
            _ => Err(PulseError::TypeMismatch {
                expected: "number or map".into(),
                got: arg.type_name(),
            }),
        }
    };

    let secs1 = get_seconds(&args[0])?;
    let secs2 = get_seconds(&args[1])?;

    let total_secs = secs1 + secs2;
    let duration = Duration::from_secs_f64(total_secs);

    let mut map: HashMap<String, Value> = HashMap::new();
    map.insert("seconds".to_string(), Value::Float(duration.as_secs_f64()));
    map.insert(
        "milliseconds".to_string(),
        Value::Int(duration.as_millis() as i64),
    );

    let map_handle = heap.alloc_object(Object::Map(map));
    Ok(Value::Obj(map_handle))
}

/// measure_time(fn: Function) -> Map
/// Measures execution time of a function and returns duration info
pub fn measure_time_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "measure_time expects 1 argument".into(),
            ));
        }

        let start = std::time::Instant::now();

        // TODO: Actually call the function when we have proper async support
        // For now, just return the time
        let _ = args[0]; // Use the argument to avoid unused warning
        let elapsed = start.elapsed();

        let mut map: HashMap<String, Value> = HashMap::new();
        map.insert("seconds".to_string(), Value::Float(elapsed.as_secs_f64()));
        map.insert(
            "milliseconds".to_string(),
            Value::Int(elapsed.as_millis() as i64),
        );
        map.insert(
            "microseconds".to_string(),
            Value::Int(elapsed.as_micros() as i64),
        );

        let map_handle = heap.alloc_object(Object::Map(map));
        Ok(Value::Obj(map_handle))
    }
    .boxed()
}

/// unix_to_datetime(timestamp: Float) -> Map
/// Converts Unix timestamp to a datetime map
pub fn unix_to_datetime_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "unix_to_datetime expects 1 argument".into(),
        ));
    }

    let timestamp = match &args[0] {
        Value::Float(f) => *f,
        Value::Int(i) => *i as f64,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    let secs = timestamp as i64;
    let naive = match DateTime::from_timestamp(secs, 0) {
        Some(dt) => dt.naive_utc(),
        None => return Err(PulseError::RuntimeError("Invalid timestamp".into())),
    };
    let datetime: DateTime<Local> =
        DateTime::from_naive_utc_and_offset(naive, *Local::now().offset());

    let mut map: HashMap<String, Value> = HashMap::new();
    map.insert("year".to_string(), Value::Int(datetime.year() as i64));
    map.insert("month".to_string(), Value::Int(datetime.month() as i64));
    map.insert("day".to_string(), Value::Int(datetime.day() as i64));
    map.insert("hour".to_string(), Value::Int(datetime.hour() as i64));
    map.insert("minute".to_string(), Value::Int(datetime.minute() as i64));
    map.insert("second".to_string(), Value::Int(datetime.second() as i64));
    map.insert("timestamp".to_string(), Value::Float(timestamp));

    let map_handle = heap.alloc_object(Object::Map(map));
    Ok(Value::Obj(map_handle))
}

/// datetime_to_unix(dt: Map) -> Float
/// Converts a datetime map to Unix timestamp
pub fn datetime_to_unix_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "datetime_to_unix expects 1 argument".into(),
        ));
    }

    let get_field = |map: &HashMap<String, Value>, field: &str| -> PulseResult<i64> {
        if let Some(v) = map.get(field) {
            match v {
                Value::Int(i) => Ok(*i),
                Value::Float(f) => Ok(*f as i64),
                _ => Err(PulseError::RuntimeError(format!("Invalid {} field", field))),
            }
        } else {
            Err(PulseError::RuntimeError(format!("Missing {} field", field)))
        }
    };

    match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::Map(m)) = heap.get_object(*h) {
                let year = get_field(m, "year")?;
                let month = get_field(m, "month")? as u32;
                let day = get_field(m, "day")? as u32;
                let hour = get_field(m, "hour")? as u32;
                let minute = get_field(m, "minute")? as u32;
                let second = get_field(m, "second")? as u32;

                let naive = match DateTime::from_timestamp(second as i64, 0) {
                    Some(dt) => dt.naive_utc(),
                    None => return Err(PulseError::RuntimeError("Invalid datetime".into())),
                };
                let naive = naive
                    .with_year(year as i32)
                    .ok_or_else(|| PulseError::RuntimeError("Invalid year".into()))?
                    .with_month(month)
                    .ok_or_else(|| PulseError::RuntimeError("Invalid month".into()))?
                    .with_day(day)
                    .ok_or_else(|| PulseError::RuntimeError("Invalid day".into()))?
                    .with_hour(hour)
                    .ok_or_else(|| PulseError::RuntimeError("Invalid hour".into()))?
                    .with_minute(minute)
                    .ok_or_else(|| PulseError::RuntimeError("Invalid minute".into()))?;

                let timestamp = naive.and_utc().timestamp() as f64;
                Ok(Value::Float(timestamp))
            } else {
                Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "object".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "map".into(),
            got: args[0].type_name(),
        }),
    }
}
