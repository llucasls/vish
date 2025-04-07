//! General-purpose utilities and libc bindings that don't fit cleanly elsewhere.
use std::ffi::CStr;

use libc;

/// Returns the effective user ID of the current process.
pub fn get_euid() -> u32 {
    unsafe { libc::geteuid() }
}

/// Returns the login name of the user associated with the current session.
///
/// # Notes
/// - The value returned may not always reflect the effective or real user ID.
/// - If the system fails to provide a login name, `None` is returned.
pub fn get_login() -> Option<String> {
    unsafe {
        let login_ptr: *mut libc::c_char = libc::getlogin();
        if login_ptr.is_null() {
            return None;
        }
        match CStr::from_ptr(login_ptr).to_str() {
            Ok(login) => Some(String::from(login)),
            Err(_) => None,
        }
    }
}

/// Returns the parent process ID (PPID) of the current process.
pub fn get_ppid() -> u32 {
    unsafe { libc::getppid() as u32 }
}

/// Returns the real user ID of the current process.
pub fn get_uid() -> u32 {
    unsafe { libc::getuid() }
}
