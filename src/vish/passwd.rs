use libc;
use std::ffi::{CStr, CString};

extern "C" {
    fn get_home_dir(username: *const libc::c_char) -> *const libc::c_char;
}

pub fn get_home(user: String) -> Option<String> {
    let username = CString::new(user).ok()?;
    unsafe {
        let home_dir_ptr = get_home_dir(username.as_ptr());

        if home_dir_ptr.is_null() {
            None
        } else {
            let home_dir = CStr::from_ptr(home_dir_ptr).to_str().ok()?;
            Some(String::from(home_dir))
        }
    }
}
