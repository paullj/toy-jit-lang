use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const i8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_echo_int(val: i64) {
    unsafe { printf(c"%lld\n".as_ptr(), val) };
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_echo_float(val: f64) {
    unsafe { printf(c"%g\n".as_ptr(), val) };
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_echo_bool(val: i64) {
    unsafe {
        printf(
            c"%s\n".as_ptr(),
            if val != 0 { c"true" } else { c"false" }.as_ptr(),
        )
    };
}
