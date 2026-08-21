use std::ffi::CString;
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn run() -> *const c_char {
    // Эта строка вернется в твою основную программу
    let c_str = CString::new("ПРИВЕТ ИЗ ПЛАГИНА!").unwrap();
    c_str.into_raw()
}