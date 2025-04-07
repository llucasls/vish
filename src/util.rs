//! General-purpose utilities and libc bindings that don't fit cleanly elsewhere.
use std::ffi;

use libc;

/// A namespace for functions related to retrieving user home directories.
///
/// # Examples
///
/// ```
/// use crate::util::Home;
///
/// if let Some(path) = Home::from_username("alice".to_string()) {
///     println!("Alice's home: {}", path);
/// }
///
/// let uid = unsafe { libc::getuid() };
/// if let Some(path) = Home::from_uid(uid) {
///     println!("Current user's home: {}", path);
/// }
/// ```
pub struct Home;

impl Home {
    /// Retrieves the home directory of a user by their username.
    ///
    /// Returns `Some(String)` containing the path to the user's home directory
    /// if found, or `None` if the username does not exist or the path is not
    /// valid UTF-8.
    #[cfg(not(test))]
    pub fn from_username(name: String) -> Option<String> {
        let c_string_name = ffi::CString::new(name.into_bytes()).ok()?;

        unsafe {
            let raw_name: *const i8 = c_string_name.as_ptr();
            let pw: *mut libc::passwd = libc::getpwnam(raw_name);
            libc::endpwent();

            if pw.is_null() {
                return None;
            }

            match ffi::CStr::from_ptr((*pw).pw_dir).to_str() {
                Ok(home) => Some(String::from(home)),
                Err(_) => None,
            }
        }
    }

    /// Retrieves the home directory of a user by their UID.
    ///
    /// Returns `Some(String)` containing the path to the home directory
    /// for the given UID, or `None` if the user doesn't exist or the path
    /// is not valid UTF-8.
    pub fn from_uid(uid: u32) -> Option<String> {
        unsafe {
            let pw: *mut libc::passwd = libc::getpwuid(uid);
            libc::endpwent();

            if pw.is_null() {
                return None;
            }

            match ffi::CStr::from_ptr((*pw).pw_dir).to_str() {
                Ok(home) => Some(String::from(home)),
                Err(_) => None,
            }
        }
    }
}

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
        match ffi::CStr::from_ptr(login_ptr).to_str() {
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
