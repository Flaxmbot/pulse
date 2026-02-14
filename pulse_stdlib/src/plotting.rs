//! Plotting library for Pulse
//! 
//! Provides ASCII/text-based charts for terminal output

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;

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

fn extract_int(heap: &dyn HeapInterface, value: &Value) -> PulseResult<i64> {
    match value {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(*f as i64),
        _ => Err(PulseError::RuntimeError("Expected integer value".into())),
    }
}

fn list_to_f64_vec(heap: &dyn HeapInterface, list: &[Value]) -> PulseResult<Vec<f64>> {
    list.iter().map(|v| extract_float(heap, v)).collect()
}

fn list_to_string_vec(heap: &dyn HeapInterface, list: &[Value]) -> PulseResult<Vec<String>> {
    list.iter().map(|v| {
        match v {
            Value::Obj(handle) => {
                if let Some(Object::String(s)) = heap.get_object(*handle) {
                    Ok(s.clone())
                } else {
                    Err(PulseError::RuntimeError("Expected string in list".into()))
                }
            }
            Value::Int(i) => Ok(i.to_string()),
            Value::Float(f) => Ok(format!("{}", f)),
            _ => Err(PulseError::RuntimeError("Expected string in list".into())),
        }
    }).collect()
}

// ============================================================================
// ASCII BAR CHART
// ============================================================================

/// bar_chart(data: List, labels: List) -> String
/// Creates an ASCII bar chart
pub fn bar_chart_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 1 || args.len() > 2 {
        return Err(PulseError::RuntimeError("bar_chart expects 1-2 arguments: data, labels (optional)".into()));
    }

    let data_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data_list)?;
    
    let labels = if args.len() >= 2 {
        let label_list = match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
                else { return Err(PulseError::RuntimeError("Expected list for labels".into())); }
            }
            _ => return Err(PulseError::RuntimeError("Expected list for labels".into())),
        };
        list_to_string_vec(heap, &label_list)?
    } else {
        values.iter().enumerate().map(|(i, _)| format!("{}", i + 1)).collect()
    };

    if values.is_empty() {
        return Ok(Value::Obj(heap.alloc_object(Object::String("No data to plot".to_string()))));
    }

    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let range = if max_val - min_val == 0.0 { 1.0 } else { max_val - min_val };
    
    let chart_width = 40;
    let bar_max_width = 20;
    
    let mut output = String::new();
    output.push_str("┌────────────────────────────────────────┐\n");
    output.push_str("│           BAR CHART                    │\n");
    output.push_str("├────────────────────────────────────────┤\n");
    
    for (i, &v) in values.iter().enumerate() {
        let label = labels.get(i).cloned().unwrap_or_default();
        let normalized = (v - min_val) / range;
        let bar_len = (normalized * bar_max_width as f64) as usize;
        let bar: String = "█".repeat(bar_len);
        
        output.push_str(&format!("│ {:12} │ {:20} │ {:8.2} │\n", 
            if label.len() > 12 { &label[..12] } else { &label }, 
            bar, v));
    }
    
    output.push_str("├────────────────────────────────────────┤\n");
    output.push_str(&format!("│ Min: {:6.2}  Max: {:6.2}  Range: {:6.2}  │\n", 
        min_val, max_val, range));
    output.push_str("└────────────────────────────────────────┘\n");

    Ok(Value::Obj(heap.alloc_object(Object::String(output))))
}

// ============================================================================
// ASCII LINE CHART
// ============================================================================

/// line_chart(data: List) -> String
/// Creates an ASCII line chart
pub fn line_chart_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("line_chart expects 1 argument: data".into()));
    }

    let data_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data_list)?;

    if values.is_empty() {
        return Ok(Value::Obj(heap.alloc_object(Object::String("No data to plot".to_string()))));
    }

    let height = 10;
    let width = 40;
    
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let range = if max_val - min_val == 0.0 { 1.0 } else { max_val - min_val };
    
    let mut grid: Vec<Vec<char>> = vec![vec![' '; width]; height];
    
    // Fill grid with line
    for i in 0..(values.len() - 1).min(width - 1) {
        let x1 = i;
        let x2 = (i + 1).min(values.len() - 1);
        
        let y1 = ((values[x1] - min_val) / range * (height - 1) as f64) as usize;
        let y2 = ((values[x2] - min_val) / range * (height - 1) as f64) as usize;
        
        let y_min = y1.min(y2).min(height - 1);
        let y_max = y1.max(y2).min(height - 1);
        
        for y in y_min..=y_max {
            grid[y][x1] = '─';
        }
        
        if y1 != y2 {
            grid[height - 1 - y2][x2] = if y2 > y1 { '└' } else { '┌' };
        } else {
            grid[height - 1 - y2][x2] = '●';
        }
    }
    
    // Mark endpoints
    if !values.is_empty() {
        let first_y = ((values[0] - min_val) / range * (height - 1) as f64) as usize;
        grid[height - 1 - first_y.min(height - 1)][0] = '●';
        
        let last_idx = (values.len() - 1).min(width - 1);
        let last_y = ((values[values.len() - 1] - min_val) / range * (height - 1) as f64) as usize;
        grid[height - 1 - last_y.min(height - 1)][last_idx] = '●';
    }
    
    let mut output = String::new();
    output.push_str("┌────────────────────────────────────────┐\n");
    output.push_str("│           LINE CHART                   │\n");
    output.push_str("├────────────────────────────────────────┤\n");
    
    for row in grid.iter() {
        output.push_str("│");
        for c in row.iter() {
            output.push(*c);
        }
        output.push_str(" │\n");
    }
    
    output.push_str("├────────────────────────────────────────┤\n");
    output.push_str(&format!("│ {:>10} {:>22} │\n", 
        format!("Min: {:.2}", min_val),
        format!("Max: {:.2}", max_val)));
    output.push_str("└────────────────────────────────────────┘\n");

    Ok(Value::Obj(heap.alloc_object(Object::String(output))))
}

// ============================================================================
// ASCII HISTOGRAM
// ============================================================================

/// histogram(data: List, bins: Int) -> String
/// Creates an ASCII histogram
pub fn histogram_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 1 || args.len() > 2 {
        return Err(PulseError::RuntimeError("histogram expects 1-2 arguments: data, bins (optional)".into()));
    }

    let data_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data_list)?;
    let num_bins = if args.len() >= 2 {
        extract_int(heap, &args[1])? as usize
    } else {
        10
    };

    if values.is_empty() {
        return Ok(Value::Obj(heap.alloc_object(Object::String("No data to plot".to_string()))));
    }

    let num_bins = num_bins.max(1).min(20);
    let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = if max_val - min_val == 0.0 { 1.0 } else { max_val - min_val };
    
    let mut bins: Vec<usize> = vec![0; num_bins];
    
    for &v in &values {
        let bin_idx = ((v - min_val) / range * num_bins as f64) as usize;
        let bin_idx = bin_idx.min(num_bins - 1);
        bins[bin_idx] += 1;
    }
    
    let max_count = bins.iter().cloned().fold(0usize, usize::max);
    let bar_max_width = 20;
    
    let mut output = String::new();
    output.push_str("┌────────────────────────────────────────┐\n");
    output.push_str("│          HISTOGRAM                     │\n");
    output.push_str("├────────────────────────────────────────┤\n");
    
    let bin_width = range / num_bins as f64;
    
    for (i, &count) in bins.iter().enumerate() {
        let bin_start = min_val + (i as f64) * bin_width;
        let bin_end = min_val + ((i + 1) as f64) * bin_width;
        
        let normalized = if max_count > 0 { count as f64 / max_count as f64 } else { 0.0 };
        let bar_len = (normalized * bar_max_width as f64) as usize;
        let bar: String = "█".repeat(bar_len);
        
        output.push_str(&format!("│ [{:5.1},{:5.1}) │ {:20} │ {:4} │\n", 
            bin_start, bin_end, bar, count));
    }
    
    output.push_str("├────────────────────────────────────────┤\n");
    output.push_str(&format!("│ Total: {:3} bins: {:2}  Range: {:.2}    │\n", 
        values.len(), num_bins, range));
    output.push_str("└────────────────────────────────────────┘\n");

    Ok(Value::Obj(heap.alloc_object(Object::String(output))))
}

// ============================================================================
// ASCII BOX PLOT
// ============================================================================

/// box_plot(data: List, label: String) -> String
/// Creates an ASCII box plot
pub fn box_plot_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 1 || args.len() > 2 {
        return Err(PulseError::RuntimeError("box_plot expects 1-2 arguments: data, label (optional)".into()));
    }

    let data_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data_list)?;
    let label = if args.len() >= 2 {
        match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) { s.clone() } 
                else { "Data".to_string() }
            }
            _ => "Data".to_string(),
        }
    } else {
        "Data".to_string()
    };

    if values.is_empty() {
        return Ok(Value::Obj(heap.alloc_object(Object::String("No data to plot".to_string()))));
    }

    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let n = sorted.len();
    let min = sorted[0];
    let max = sorted[n - 1];
    
    let q1_idx = n / 4;
    let q2_idx = n / 2;
    let q3_idx = (3 * n) / 4;
    
    let q1 = sorted[q1_idx];
    let q2 = sorted[q2_idx];
    let q3 = sorted[q3_idx];
    let iqr = q3 - q1;
    
    let lower_whisker = (q1 - 1.5 * iqr).max(min);
    let upper_whisker = (q3 + 1.5 * iqr).min(max);
    
    let width = 40;
    let range = max - min;
    let scale = if range == 0.0 { 1.0 } else { (width - 10) as f64 / range };
    
    fn pos(v: f64, min: f64, scale: f64) -> usize {
        ((v - min) * scale) as usize
    }
    
    let min_pos = pos(min, min, scale).min(width - 5);
    let q1_pos = pos(q1, min, scale);
    let q2_pos = pos(q2, min, scale);
    let q3_pos = pos(q3, min, scale);
    let max_pos = pos(max, min, scale).min(width - 5);
    
    let mut line = vec![' '; width];
    
    // Whiskers
    for i in min_pos..=q1_pos { if i < width { line[i] = '─'; } }
    for i in q3_pos..=max_pos { if i < width { line[i] = '─'; } }
    
    // Box
    for i in q1_pos..=q3_pos { if i < width { line[i] = '█'; } }
    
    // Median
    if q2_pos < width { line[q2_pos] = '│'; }
    
    let whisker_low = if min < q1 - 1.5 * iqr { '○' } else { '│' };
    let whisker_high = if max > q3 + 1.5 * iqr { '○' } else { '│' };
    
    if min_pos < width { line[min_pos] = whisker_low; }
    if max_pos < width { line[max_pos] = whisker_high; }
    
    let chart: String = line.iter().collect();
    
    let mut output = String::new();
    output.push_str("┌────────────────────────────────────────┐\n");
    output.push_str(&format!("│      BOX PLOT: {:20}      │\n", 
        if label.len() > 20 { &label[..20] } else { &label }));
    output.push_str("├────────────────────────────────────────┤\n");
    output.push_str(&format!("│{}│\n", chart));
    output.push_str("├────────────────────────────────────────┤\n");
    output.push_str(&format!("│ Min: {:6.2}                              │\n", min));
    output.push_str(&format!("│ Q1:  {:6.2}  Median: {:6.2}  Q3: {:6.2} │\n", q1, q2, q3));
    output.push_str(&format!("│ Max: {:6.2}                              │\n", max));
    output.push_str("└────────────────────────────────────────┘\n");

    Ok(Value::Obj(heap.alloc_object(Object::String(output))))
}

// ============================================================================
// SCATTER PLOT (ASCII)
// ============================================================================

/// scatter_plot(x: List, y: List) -> String
/// Creates an ASCII scatter plot
pub fn scatter_plot_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("scatter_plot expects 2 arguments: x, y".into()));
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

    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return Err(PulseError::RuntimeError("X and Y must have same non-empty length".into()));
    }

    let height = 15;
    let width = 40;
    
    let min_x = x.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_x = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_y = y.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_y = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    let range_x = if max_x - min_x == 0.0 { 1.0 } else { max_x - min_x };
    let range_y = if max_y - min_y == 0.0 { 1.0 } else { max_y - min_y };
    
    let mut grid: Vec<Vec<char>> = vec![vec![' '; width]; height];
    
    // Add points
    for i in 0..x.len() {
        let px = ((x[i] - min_x) / range_x * (width - 1) as f64) as usize;
        let py = height - 1 - ((y[i] - min_y) / range_y * (height - 1) as f64) as usize;
        
        let px = px.min(width - 1);
        let py = py.min(height - 1);
        
        grid[py][px] = '●';
    }
    
    let mut output = String::new();
    output.push_str("┌────────────────────────────────────────┐\n");
    output.push_str("│         SCATTER PLOT                  │\n");
    output.push_str("├────────────────────────────────────────┤\n");
    
    for row in grid.iter() {
        output.push_str("│");
        for c in row.iter() {
            output.push(*c);
        }
        output.push_str(" │\n");
    }
    
    output.push_str("├────────────────────────────────────────┤\n");
    output.push_str(&format!("│ X: [{:.1}, {:.1}]                     │\n", min_x, max_x));
    output.push_str(&format!("│ Y: [{:.1}, {:.1}]                     │\n", min_y, max_y));
    output.push_str("└────────────────────────────────────────┘\n");

    Ok(Value::Obj(heap.alloc_object(Object::String(output))))
}

// ============================================================================
// HORIZONTAL BAR CHART
// ============================================================================

/// hbar_chart(data: List, labels: List) -> String
/// Creates a horizontal bar chart
pub fn hbar_chart_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 1 || args.len() > 2 {
        return Err(PulseError::RuntimeError("hbar_chart expects 1-2 arguments: data, labels (optional)".into()));
    }

    let data_list = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
            else { return Err(PulseError::RuntimeError("Expected list for data".into())); }
        }
        _ => return Err(PulseError::RuntimeError("Expected list for data".into())),
    };

    let values = list_to_f64_vec(heap, &data_list)?;
    
    let labels = if args.len() >= 2 {
        let label_list = match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::List(l)) = heap.get_object(*h) { l.clone() } 
                else { return Err(PulseError::RuntimeError("Expected list for labels".into())); }
            }
            _ => return Err(PulseError::RuntimeError("Expected list for labels".into())),
        };
        list_to_string_vec(heap, &label_list)?
    } else {
        values.iter().enumerate().map(|(i, _)| format!("Item {}", i + 1)).collect()
    };

    if values.is_empty() {
        return Ok(Value::Obj(heap.alloc_object(Object::String("No data to plot".to_string()))));
    }

    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let max_label_len = labels.iter().map(|l| l.len()).max().unwrap_or(0).min(15);
    let bar_width = 25;
    
    let mut output = String::new();
    output.push_str("┌────────────────────────────────────────────────────┐\n");
    output.push_str("│           HORIZONTAL BAR CHART                     │\n");
    output.push_str("├────────────────────────────────────────────────────┤\n");
    
    for (i, &v) in values.iter().enumerate() {
        let label = labels.get(i).cloned().unwrap_or_default();
        let display_label = if label.len() > max_label_len { 
            format!("{}..", &label[..max_label_len-2]) 
        } else { 
            label.clone() 
        };
        
        let normalized = if max_val > 0.0 { v / max_val } else { 0.0 };
        let bar_len = (normalized * bar_width as f64) as usize;
        let bar: String = "▓".repeat(bar_len);
        
        output.push_str(&format!("│ {:>width$} │ {:<bar_width$} │ {:6.2} │\n", 
            display_label, bar, v, width = max_label_len, bar_width = bar_width));
    }
    
    output.push_str("├────────────────────────────────────────────────────┤\n");
    output.push_str(&format!("│ Max value: {:6.2}                                 │\n", max_val));
    output.push_str("└────────────────────────────────────────────────────┘\n");

    Ok(Value::Obj(heap.alloc_object(Object::String(output))))
}
