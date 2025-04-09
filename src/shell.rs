//! Shell runtime state and configuration.
//!
//! This module defines the core data structures representing the internal state
//! and execution environment of a shell session. These structures serve as the
//! foundation for modeling the shell’s behavior during runtime.
//!
//! The primary structure, [`Shell`], holds persistent information relevant to a
//! running shell instance, including its configuration, environment, and
//! stateful components such as:
//!
//! - Open file descriptors
//! - The current working directory as set by `cd`
//! - File creation mask as set by `umask`
//! - File size limit as set by `ulimit`
//! - Currently registered traps from `trap`
//! - Shell parameters (e.g., environment and internal variables)
//! - Shell-defined functions
//! - Runtime options set at invocation or via `set`
//! - Background jobs and their associated process IDs
//! - Shell aliases
//! - The shell's PID and its parent's PID
//!
//! ## Components
//!
//! - [`Shell`]: Represents the full execution context of a shell session,
//!   including process IDs, configuration options, and variable definitions.
//! - [`ShellVariable`]: Represents a shell variable and its metadata, such as
//!   export and read-only status.
//! - [`ShellOptions`]: A collection of predefined boolean flags that govern
//!   shell behavior, inspired by POSIX and common shell implementations.
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::process;

use crate::vish::command::ArgV;
use crate::util::{Home, get_ppid, get_uid, get_login, get_shell};

/// Represents the runtime context of a shell session.
///
/// This struct holds the core state of an interactive or scripted shell,
/// including command-line arguments, shell variables, and process identifiers.
/// It also includes the current shell options that affect how the shell behaves.
#[derive(Debug)]
pub struct Shell {
    /// Parsed command-line arguments passed to the shell at startup.
    pub argv: ArgV,

    /// A mapping of shell variable names to their values and metadata.
    pub vars: HashMap<String, ShellVariable>,

    /// A set of active shell options (like `errexit`, `nounset`, etc.)
    /// that influence control flow, expansion, and execution behavior.
    pub opts: ShellOptions,

    /// The process ID seen by the shell itself (`$$` in POSIX shells).
    pub pid: u32,

    /// The parent process ID (`$PPID`).
    pub ppid: u32,

    real_pid: u32,
}

/// Represents a shell variable and its associated metadata.
#[derive(Debug)]
pub struct ShellVariable {
    /// The current string value of the variable.
    pub value: String,

    /// Whether the variable should be part of the environment,
    /// making it visible to child processes.
    pub exported: bool,

    /// Whether the variable is read-only and cannot be reassigned.
    pub readonly: bool,
}

/// A set of boolean flags representing shell options.
#[derive(Debug)]
pub struct ShellOptions {
    /// Automatically export all new variables.
    pub allexport: bool,

    /// Exit immediately if a command fails (i.e., returns a
    /// non-zero status).
    pub errexit: bool,

    /// Prevent `Ctrl+D` (EOF) from exiting the shell.
    pub ignoreeof: bool,

    /// Enable job control.
    pub monitor: bool,

    /// Prevent redirection (`>`) from overwriting existing files.
    pub noclobber: bool,

    /// Disable pathname expansion (globbing).
    pub noglob: bool,

    /// Parse commands but do not execute them.
    pub noexec: bool,

    /// Ignored. This is included for compatibility with other shells.
    pub nolog: bool,

    /// Notify of background job completion immediately.
    pub notify: bool,

    /// Treat unset variables as errors during parameter expansion.
    pub nounset: bool,

    /// The return value of a pipeline is the status of the last
    /// command to fail, or zero if none failed.
    pub pipefail: bool,

    /// Print shell input lines to stderr as they are read.
    pub verbose: bool,

    /// Enable `vi` editing mode for the command line.
    pub vi: bool,

    /// Print commands and their arguments as they are executed.
    pub xtrace: bool,
}

#[derive(Debug)]
pub enum ShellVarError {
    NotPresent,
    NotUnicode(OsString),
    NotWritable(String),
}

impl Shell {
    pub fn new() -> Self {
        let argv = std::env::args().collect::<ArgV>();
        let pid = process::id();
        let ppid = get_ppid();
        let opts = ShellOptions::new();
        let real_pid = process::id();

        let mut vars: HashMap<String, ShellVariable> = HashMap::new();

        let default_vars = Self::init_vars();
        for (key, val) in default_vars {
            let name = key.to_string();
            let value = val.to_string();
            let exported = false;
            let readonly = false;
            vars.insert(name, ShellVariable { value, exported, readonly });
        }

        let dyn_vars = Self::init_dyn_vars();
        for (key, opt) in dyn_vars {
            if let Some(value) = opt {
                let name = key.to_string();
                let exported = false;
                let readonly = false;
                vars.insert(name, ShellVariable { value, exported, readonly });
            }
        }

        let env_vars = Self::init_env_vars();
        for (name, value) in env_vars {
            vars.insert(name, value);
        }

        Self {
            argv,
            vars,
            pid,
            ppid,
            opts,
            real_pid,
        }
    }

    pub fn get_var(&self, name: &str) -> Result<String, ShellVarError> {
        match self.vars.get(name) {
            Some(var) => Ok(var.value.clone()),
            None => Err(ShellVarError::NotPresent),
        }
    }

    pub fn set_var(&mut self, name: &str, value: &str) ->
        Result<(), ShellVarError> {
        if std::str::from_utf8(name.as_bytes()).is_err() {
            return Err(ShellVarError::NotUnicode(OsString::from(name)));
        } else if std::str::from_utf8(name.as_bytes()).is_err() {
            return Err(ShellVarError::NotUnicode(OsString::from(value)));
        }

        match self.vars.get_mut(name) {
            Some(var) => {
                if var.readonly {
                    return Err(ShellVarError::NotWritable(name.to_string()));
                }
                var.value = value.to_string();
                if var.exported {
                    unsafe {
                        std::env::set_var(name, value);
                    }
                }
            },
            None => {
                self.vars.insert(name.to_string(), ShellVariable {
                    value: value.to_string(),
                    exported: false,
                    readonly: false,
                });
            }
        }
        Ok(())
    }

    pub fn unset_var(&mut self, name: &str) {
        self.vars.remove(name);
        unsafe {
            std::env::remove_var(name);
        }
    }

    pub fn export_var(&mut self, name: &str) {
        match self.vars.get_mut(name) {
            Some(ShellVariable { exported: true, .. }) => {},
            Some(var) => {
                var.exported = true;
                unsafe {
                    std::env::set_var(name, var.value.clone());
                }
            },
            None => {
                self.vars.insert(name.to_string(), ShellVariable {
                    value: String::new(),
                    exported: true,
                    readonly: false,
                });
                unsafe {
                    std::env::set_var(name, String::new());
                }
            }
        }
    }

    pub fn unexport_var(&mut self, _name: &str) {}

    pub fn freeze_var(&mut self, _name: &str) {}

    /// Return hard-coded defaults for unset parameters.
    fn init_vars() -> [(&'static str, String); 8] {
        let default_path = String::from("/usr/local/bin:/bin:/usr/bin");

        let root_path: String = [
            "/usr/local/sbin",
            "/usr/local/bin",
            "/sbin",
            "/bin",
            "/usr/sbin",
            "/usr/bin",
        ].join(":");

        [
            ("IFS", String::from_utf8(b" \t\n".to_vec()).unwrap_or_default()),
            ("LINENO", String::from("1")),
            ("PATH", if get_uid() == 0 { root_path } else { default_path }),
            ("PPID", get_ppid().to_string()),
            ("PS1", String::from(if get_uid() == 0 { "# " } else { "$ " })),
            ("PS2", String::from("> ")),
            ("PS3", String::from("#? ")),
            ("PS4", String::from("+ ")),
        ]
    }

    /// Return dynamic default values for unset parameters.
    fn init_dyn_vars() -> [(&'static str, Option<String>); 5] {
        let username = get_login();
        let pwd = std::env::current_dir().ok()
            .and_then(|pb| pb.to_str().map(|s| s.to_string()));

        [
            ("HOME", Home::from_uid(get_uid())),
            ("LOGNAME", username.clone()),
            ("PWD", pwd),
            ("SHELL", get_shell(get_uid())),
            ("USER", username),
        ]
    }

    /// Return environment variables inherited from the parent process.
    fn init_env_vars() -> Vec<(String, ShellVariable)> {
        let exported = true;
        let readonly = false;
        let mut variables = Vec::new();
        for (key, value) in std::env::vars() {
            variables.push((key, ShellVariable { value, exported, readonly }));
        }
        variables
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            argv: vec![String::from("vish")],
            vars: HashMap::new(),
            pid: 1,
            ppid: 0,
            opts: ShellOptions::new(),
            real_pid: process::id(),
        }
    }
}

impl ShellOptions {
    fn new() -> Self {
        Self {
            allexport: false,
            errexit: false,
            ignoreeof: false,
            monitor: false,
            noclobber: false,
            noglob: false,
            noexec: false,
            nolog: false,
            notify: false,
            nounset: false,
            pipefail: false,
            verbose: false,
            vi: false,
            xtrace: false,
        }
    }
}

impl fmt::Display for ShellVarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellVarError::NotPresent =>
                write!(f, "shell variable not found"),
            ShellVarError::NotUnicode(s) =>
                write!(f, "value {:?} is not valid unicode", s),
            ShellVarError::NotWritable(n) =>
                write!(f, "shell variable {:?} is read-only", n),
        }
    }
}

impl PartialEq for Shell {
    fn eq(&self, other: &Self) -> bool {
        self.real_pid == other.real_pid
    }
}
