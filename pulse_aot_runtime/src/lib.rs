//! Pulse AOT Runtime Library
//!
//! Provides extern "C" functions called by AOT-compiled Pulse native binaries.
//! These are linked statically into the final executable.

use std::time::{SystemTime, UNIX_EPOCH};
use std::{ffi::CStr, os::raw::c_char};

// ============ Print Functions ============

#[no_mangle]
pub extern "C" fn pulse_print_int(val: i64) {
    print!("{}", val);
}

#[no_mangle]
pub extern "C" fn pulse_print_float(bits: i64) {
    let f = f64::from_bits(bits as u64);
    print!("{}", f);
}

#[no_mangle]
pub extern "C" fn pulse_print_bool(val: i64) {
    if val != 0 {
        print!("true");
    } else {
        print!("false");
    }
}

#[no_mangle]
pub extern "C" fn pulse_print_newline() {
    println!();
}

#[no_mangle]
/// # Safety
/// `ptr` must point to a valid UTF-8 string of exactly `len` bytes.
pub unsafe extern "C" fn pulse_print_string(ptr: *const u8, len: usize) {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    if let Ok(s) = std::str::from_utf8(slice) {
        print!("{}", s);
    }
}

#[no_mangle]
/// # Safety
/// `ptr` must point to a valid null-terminated UTF-8 string.
pub unsafe extern "C" fn pulse_print_cstr(ptr: *const u8) {
    if ptr.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(ptr as *const c_char) };
    if let Ok(s) = c_str.to_str() {
        print!("{}", s);
    }
}

// ============ Legacy Print (backwards compat) ============

#[no_mangle]
pub extern "C" fn pulse_println(val: i64) {
    println!("{}", val);
}

// ============ Utility Functions ============

#[no_mangle]
pub extern "C" fn pulse_clock() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Expected a value")
        .as_secs_f64()
}

#[no_mangle]
/// # Safety
/// `msg_ptr` must point to a valid UTF-8 string of exactly `len` bytes.
pub unsafe extern "C" fn pulse_panic(msg_ptr: *const u8, len: usize) {
    let msg = unsafe {
        let slice = std::slice::from_raw_parts(msg_ptr, len);
        std::str::from_utf8_unchecked(slice)
    };
    panic!("Pulse AOT Panic: {}", msg);
}

// ============ Memory Allocation ============

#[no_mangle]
/// # Safety
/// `ptr` must point to a valid byte buffer of `len` bytes.
/// Returns a heap-allocated copy. Caller must free with `pulse_free_string`.
pub unsafe extern "C" fn pulse_alloc_string(ptr: *const u8, len: usize) -> *mut u8 {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut buf = Vec::with_capacity(len);
    buf.extend_from_slice(slice);
    let boxed = buf.into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

#[no_mangle]
/// # Safety
/// `ptr` must have been allocated by `pulse_alloc_string` with the given `len`.
pub unsafe extern "C" fn pulse_free_string(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        let _ = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) };
    }
}
