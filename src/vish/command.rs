use std::fs;
use std::io;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::io::ErrorKind::{NotFound, PermissionDenied, InvalidInput};
use std::env::{self, set_current_dir, current_dir};
use std::cmp::Ordering;
use std::ffi::OsString;
use std::fmt::format;
use std::path::PathBuf;

use libc::{self, pid_t};

use super::buffer::Buffer;
use super::io::InputReader;

use crate::shell::ShellVarError;

pub type ArgV = Vec<String>;
pub enum ShellCommand {
    SpBuiltin(String),
    Builtin(String),
    Function(String),
    Alias(String),
    Process(String),
    Script(String),
    Variable(String, String),
}

pub struct Fork {
    action: Option<Box<dyn FnOnce() -> i32>>,
}

#[derive(Clone, Copy, Debug)]
pub struct ForkedProcess {
    pid: pid_t,
    was_killed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct WaitStatus {
    raw_status: i32,
}

impl Fork {
    pub fn new() -> Self {
        Self {
            action: None,
        }
    }

    /// This method receives a closure with actions to be executed
    /// in the context of the child process. The return value will
    /// be used as the exit code for the `_exit()` function.
    ///
    /// # Safety
    /// If the process is multithreaded, the closure must not call
    /// non-async-signal-safe functions.
    pub fn procedure<F>(mut self, f: F) -> Self
        where F: FnOnce() -> i32 + 'static,
    {
        let action: Box<dyn FnOnce() -> i32> = Box::new(f);
        self.action = Some(action);
        self
    }

    pub fn spawn(self) -> io::Result<ForkedProcess> {
        match unsafe { libc::fork() } {
            -1 => Err(io::Error::last_os_error()),
            0 => {
                let code: i32 = match self.action {
                    Some(action) => {
                        action()
                    },
                    None => 0,
                };
                unsafe { libc::_exit(code) };
            },
            pid => Ok(ForkedProcess { pid, was_killed: false }),
        }
    }
}

impl Default for Fork {
    fn default() -> Self {
        Self::new()
    }
}

impl ForkedProcess {
    pub fn id(&self) -> pid_t {
        self.pid
    }

    pub fn kill(&mut self) -> io::Result<()> {
        if self.was_killed {
            return Ok(());
        }

        match unsafe { libc::kill(self.pid, libc::SIGKILL) } {
            -1 => Err(io::Error::last_os_error()),
            _ => {
                self.was_killed = true;
                Ok(())
            }
        }
    }

    pub fn send_signal(&self, signal: i32) -> io::Result<()> {
        match unsafe { libc::kill(self.pid, signal) } {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        }
    }

    pub fn wait(&self) -> io::Result<WaitStatus> {
        let mut raw_status: i32 = 0;
        let options: i32 =
            libc::WUNTRACED |
            libc::WCONTINUED;

        match unsafe {
            libc::waitpid(self.pid, &mut raw_status, options)
        } {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(WaitStatus { raw_status }),
        }
    }

    /// Returns `WaitStatus` of the child if it has already exited.
    ///
    /// This method doesn't block the calling thread and works like
    /// `ExitStatus` from the standard library.
    ///
    /// If the child has exited, then Ok(Some(status)) is returned.
    /// If the exit status is not available at this time then Ok(None)
    /// is returned. If an error occurs, then that error is returned.
    pub fn try_wait(&mut self) -> io::Result<Option<WaitStatus>> {
        let mut raw_status: i32 = 0;
        let options: i32 =
            libc::WNOHANG |
            libc::WUNTRACED |
            libc::WCONTINUED;

        match unsafe {
            libc::waitpid(self.pid, &mut raw_status, options)
        } {
            -1 => Err(io::Error::last_os_error()),
            0 => Ok(None),
            _ => Ok(Some(WaitStatus { raw_status })),
        }
    }
}

impl WaitStatus {
    /// Creates a new WaitStatus from raw wait status number.
    pub fn from_raw(raw_status: i32) -> Self {
        Self { raw_status }
    }

    /// Returns true if process exited with status `0`.
    pub fn success(&self) -> bool {
        if libc::WIFEXITED(self.raw_status) {
            libc::WEXITSTATUS(self.raw_status) == 0
        } else {
            false
        }
    }

    /// Returns the exit code of the process, if any.
    pub fn code(&self) -> Option<i32> {
        if libc::WIFEXITED(self.raw_status) {
            Some(libc::WEXITSTATUS(self.raw_status))
        } else {
            None
        }
    }

    /// If the process was terminated by a signal, returns that signal.
    pub fn signal(&self) -> Option<i32> {
        if libc::WIFSIGNALED(self.raw_status) {
            Some(libc::WTERMSIG(self.raw_status))
        } else {
            None
        }
    }

    /// If the process was terminated by a signal,
    /// says whether it dumped core.
    pub fn core_dumped(&self) -> bool {
        if libc::WIFSIGNALED(self.raw_status) {
            libc::WCOREDUMP(self.raw_status)
        } else {
            false
        }
    }

    /// If the process was stopped by a signal, returns that signal.
    pub fn stopped_signal(&self) -> Option<i32> {
        if libc::WIFSTOPPED(self.raw_status) {
            Some(libc::WSTOPSIG(self.raw_status))
        } else {
            None
        }
    }

    /// Whether the process was continued from a stopped status.
    pub fn continued(&self) -> bool {
        libc::WIFCONTINUED(self.raw_status)
    }

    /// Returns the underlying raw wait status as an integer.
    pub fn into_raw(self) -> i32 {
        self.raw_status
    }
}

macro_rules! error_msg {
    ($err:ident) => { $err.to_string().split(" (").collect::<Vec<_>>()[0] }
}

pub fn run_command(argv: ArgV) -> u8 {
    if argv.is_empty() {
        eprintln!("no command was provided");
        return 1;
    }

    let cmd = &argv[0];
    let args = &argv[1..];
    match Command::new(cmd).args(args).process_group(0).status() {
        Ok(status) => status.code().unwrap_or(1) as u8,
        Err(_) => 1,
    }
}

pub fn cd(argv: ArgV) -> u8 {
    match argv.len().cmp(&2) {
        Ordering::Greater => {
            eprintln!("vish: cd: too many arguments");
            1
        },
        Ordering::Equal => {
            let name = &argv[1];
            let mut path_buf = PathBuf::new();
            let mut cur_dir: PathBuf = PathBuf::new();
            match current_dir() {
                Ok(buf) => { path_buf.push(buf.clone()); cur_dir.push(buf); },
                Err(_) => {
                    eprintln!("vish: cd: current directory is unknown");
                    return 1;
                }
            }
            if argv[1] == "-" {
                match env::var("OLDPWD") {
                    Ok(old_pwd) => {
                        path_buf.clear();
                        path_buf.push(old_pwd);
                    },
                    Err(_) => {
                        eprintln!("vish: cd: OLDPWD is not set");
                        return 1;
                    }
                }
            } else {
                match argv[1].find('/') {
                    Some(0) => {
                        path_buf.clear();
                        path_buf.push(argv[1].clone());
                    },
                    _ => {
                        for dir in argv[1].split('/') {
                            path_buf.push(dir);
                        }
                    }
                }
            }

            let path = path_buf.as_path();
            if let Err(e) = set_current_dir(path) {
                eprintln!("vish: cd: {} - {}", name, error_msg!(e));
                1
            } else {
                env::set_var("PWD", path);
                env::set_var("OLDPWD", cur_dir.as_path());
                0
            }
        },
        Ordering::Less => {
            match env::var("HOME") {
                Ok(home) => {
                    if set_current_dir(home).is_err() { 1 } else { 0 }
                },
                Err(_) => {
                    let msg = "vish: cd: HOME environment variable is not set";
                    eprintln!("{}", msg);
                    1
                }
            }
        },
    }
}

pub fn pwd(_argv: ArgV) -> u8 {
    let mut dir_path = OsString::new();
    match current_dir() {
        Ok(dir) => { dir_path.push(dir.into_os_string()); },
        Err(_) => {
            eprintln!("vish: pwd: can't retrieve current directory name");
            return 1;
        },
    }
    match dir_path.into_string() {
        Ok(dir_string) => { println!("{}", dir_string); },
        Err(dir_os_string) => { print_os_string(dir_os_string); },
    }
    0
}

pub fn parse_command<E>(name: &str) -> Result<ShellCommand, E> {
    if let Some((n, v)) = name.split_once('=') {
        Ok(ShellCommand::Variable(String::from(n), String::from(v)))
    } else {
        Ok(ShellCommand::Process(String::from("exit")))
    }
}

pub fn exit(argv: ArgV, exit_code: u8) -> u8 {
    if argv.len() < 2 {
        return exit_code;
    }
    match argv[1].parse() {
        Ok(code) => { code },
        Err(_) => {
            eprintln!("vish: exit: argument {:?} is not a number", argv[1]);
            1
        },
    }
}

pub fn printf(argv: ArgV) -> u8 {
    if argv.len() > 1 {
        for arg in &argv[1..] {
            let buf = Buffer::from_utf8(arg);
            let parsed_bytes = replace_escape_sequence(buf.get_ref());
            let Ok(new_arg) = String::from_utf8(parsed_bytes) else { todo!() };
            print!("{}", new_arg);
        }
    }
    0
}

pub fn echo(argv: ArgV) -> u8 {
    let size = argv.len();
    if size > 1 {
        for arg in &argv[1..(size - 1)] {
            print!("{} ", arg);
        }
        print!("{}", argv[size - 1]);
    }
    println!();
    0
}

pub fn exec(argv: ArgV, reader: &mut InputReader) -> u8 {
    if argv.len() < 2 {
        eprintln!("vish: exec: no command passed to exec");
        return 1;
    }

    let cmd = &argv[1];
    let args = &argv[2..];

    let err = unsafe {
        Command::new(cmd)
            .args(args)
            .pre_exec({
                let reader = reader.clone();
                move || {
                    reader.disable_raw_mode()
                }
            })
            .exec()
    };

    if let Err(e) = reader.enable_raw_mode() {
        eprintln!("warning: failed to reactivate raw mode: {}", e);
    }

    match err.kind() {
        NotFound => {
            eprintln!("vish: {}: Not found", argv[1]);
            127
        },
        PermissionDenied => {
            eprintln!("vish: {}: Permission denied", argv[1]);
            126
        },
        InvalidInput => {
            eprintln!("vish: {}: Invalid input", argv[1]);
            1
        },
        _ => {
            eprintln!("vish: Cannot execute command");
            1
        },
    }
}

pub fn export(argv: ArgV) -> u8 {
    match crate::ENV.write() {
        Ok(mut shell) => {
            for arg in &argv[1..] {
                let parts = arg.split('=').collect::<Vec<&str>>();
                if parts.len() == 1 {
                    let name = parts[0];
                    shell.export_var(name);
                } else if parts.len() == 2 {
                    let name = parts[0];
                    let value = parts[1];
                    shell.export_var(name);
                    if shell.set_var(name, value).is_err() {
                        return 1;
                    }
                }
            }
            0
        }
        Err(_) => 1,
    }
}

pub fn readonly(argv: ArgV) -> u8 {
    match crate::ENV.write() {
        Ok(mut shell) => {
            for arg in &argv[1..] {
                let parts = arg.split('=').collect::<Vec<&str>>();
                if parts.len() == 1 {
                    let name = parts[0];
                    if shell.freeze_var(name).is_err() {
                        return 1;
                    }
                } else if parts.len() == 2 {
                    let name = parts[0];
                    let value = parts[1];
                    if shell.set_var(name, value).is_err() {
                        return 1;
                    }
                    if shell.freeze_var(name).is_err() {
                        return 1;
                    }
                }
            }
            0
        }
        Err(_) => 1
    }
}

// possible types: shell built-in, function, alias, keyword or executable
pub fn r#type(argv: ArgV) -> u8 {
    let mut status = 0;
    let builtin = |name: &str| { println!("{} is a shell builtin", name); };
    let keyword = |name: &str| { println!("{} is a shell keyword", name); };
    // let function = |name: &str| { println!("{} is a function", name); };
    // let alias = |name: &str| { println!("{} is an alias", name); };
    let not_found = |name: &str| { eprintln!("{}: not found", name); };
    for arg in &argv[1..] {
        match arg.as_str() {
            "cd" => builtin("cd"),
            "pwd" => builtin("pwd"),
            "printf" => builtin("printf"),
            "echo" => builtin("echo"),
            "exec" => builtin("exec"),
            "exit" => builtin("exit"),
            "true" => builtin("true"),
            "false" => builtin("false"),
            "type" => builtin("type"),
            "export" => builtin("export"),
            "readonly" => builtin("readonly"),
            "unset" => builtin("unset"),
            "test" => builtin("test"),
            "if" => keyword("if"),
            "then" => keyword("then"),
            "elif" => keyword("elif"),
            "else" => keyword("else"),
            "fi" => keyword("fi"),
            "[" => builtin("["),
            "{" => keyword("{"),
            "}" => keyword("}"),
            _ => {
                // TODO: implement alias and function command types
                if let Ok(path) = env::var("PATH") {
                    let mut is_found: bool = false;
                    for dir in path.split(':') {
                        let executable = format!("{}/{}", dir, arg);
                        if fs::exists(&executable).ok() == Some(true) {
                            println!("{} is {}", arg, executable);
                            is_found = true;
                            break;
                        }
                    }
                    if !is_found {
                        status = 127;
                        not_found(arg);
                    }
                } else {
                    status = 127;
                    not_found(arg);
                }
            }
        }
    }
    status
}

pub fn unset(argv: ArgV) -> u8 {
    let mut status: u8 = 0;
    match crate::ENV.write() {
        Ok(mut shell) => {
            for arg in &argv[1..] {
                match shell.unset_var(arg) {
                    Ok(()) => {},
                    Err(ShellVarError::NotWritable(name)) => {
                        eprintln!(
                            "vish: unset: cannot unset readonly variable {}",
                            name
                        );
                        status = 1;
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                        status = 1;
                    },
                }
            }
            status
        },
        Err(_) => 1,
    }
}

pub fn set_var(argv: ArgV) -> u8 {
    match crate::ENV.write() {
        Ok(mut shell) => {
            for arg in &argv[0..] {
                let parts = arg.split('=').collect::<Vec<&str>>();
                if parts.len() == 2 {
                    let name = parts[0];
                    let value = parts[1];
                    if shell.set_var(name, value).is_err() {
                        return 1;
                    }
                }
            }
            0
        }
        Err(_) => 1
    }
}

fn replace_escape_sequence(input: &[u8]) -> Vec<u8> {
    let hex_seq = br"\x1b";
    let oct_seq = br"\033";
    let char_seq = br"\e";

    let size = input.len();
    let hex_size = hex_seq.len();
    let oct_size = oct_seq.len();
    let char_size = char_seq.len();

    let hex_array = hex_seq.as_slice();
    let oct_array = oct_seq.as_slice();
    let char_array = char_seq.as_slice();

    let escape_byte = b'\x1b';
    let mut output = Vec::new();
    let mut i = 0;

    while i < size {
        if size > hex_size && i <= size - hex_size && &input[i..i + hex_size] == hex_array {
            output.push(escape_byte);
            i += hex_size;
        } else if size > oct_size && i <= size - oct_size && &input[i..i + oct_size] == oct_array {
            output.push(escape_byte);
            i += oct_size;
        } else if size > char_size && i <= size - char_size && &input[i..i + char_size] == char_array {
            output.push(escape_byte);
            i += char_size;
        } else {
            output.push(input[i]);
            i += 1;
        }
    }

    output
}

#[cfg(test)]
mod replace_escape_sequence {
    use super::replace_escape_sequence;

    #[test]
    fn replace_hex_escape() {
        let input = &br"\x1b[".to_vec();
        assert_eq!(replace_escape_sequence(input), b"\x1b[");
    }

    #[test]
    fn replace_oct_escape() {
        let input = &br"\033[".to_vec();
        assert_eq!(replace_escape_sequence(input), b"\x1b[");
    }

    #[test]
    fn replace_char_escape() {
        let input = &br"\e[".to_vec();
        assert_eq!(replace_escape_sequence(input), b"\x1b[");
    }
}

fn print_os_string(text: OsString) {
    let mut dir_path = OsString::new();
    dir_path.push(text);
    let output: Vec<_> = format(format_args!("{:?}", dir_path))
        .chars()
        .collect::<Vec<_>>()[1..]
        .to_vec();
    for character in output.iter().take(output.len() - 1) {
        print!("{}", character);
    }
    println!();
}
