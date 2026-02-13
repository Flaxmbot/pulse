use std::time::{SystemTime, UNIX_EPOCH};

#[no_mangle]
pub extern "C" fn pulse_println(val: i64) {
    println!("{}", val);
}

#[no_mangle]
pub extern "C" fn pulse_clock() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

#[no_mangle]
pub unsafe extern "C" fn pulse_panic(msg_ptr: *const u8, len: usize) {
    let msg = unsafe {
        let slice = std::slice::from_raw_parts(msg_ptr, len);
        std::str::from_utf8_unchecked(slice)
    };
    panic!("Pulse AOT Panic: {}", msg);
}
