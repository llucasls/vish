use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Write};
use std::process::{self, ExitCode, ExitStatus, Termination};
use std::rc::{Rc, Weak};
use std::os::unix::process::ExitStatusExt;

use crate::vish::buffer::Buffer;
use crate::vish::command::{self as cmd, ArgV};
use crate::vish::io::InputReader;
use crate::vish::string::parse_argv;

#[derive(Debug)]
pub struct Shell {
    pub self_ref: Weak<RefCell<Shell>>,
    pub argv: ArgV,
    pub vars: HashMap<String, ShellVariable>,
    pub pid: u32,
    real_pid: u32,
}

#[derive(Debug)]
pub struct ShellVariable {
    pub value: String,
    pub exported: bool,
}

pub struct ShellStatus {
    msg: String,
    code: i32,
}

trait Fail<T> {
    fn fail(msg: T) -> Self;
}

impl Shell {
    pub fn new() -> Rc<RefCell<Self>> {
        let argv = std::env::args().collect::<ArgV>();
        let pid = process::id();
        let real_pid = pid;
        let self_ref = Weak::new();

        let environment_variables = std::env::vars()
            .map(|(key, value)| (key, ShellVariable { value, exported: true }));
        let vars = HashMap::from_iter(environment_variables);

        let shell_rc = Rc::new(RefCell::new(Self {
            self_ref,
            argv,
            vars,
            pid,
            real_pid,
        }));

        shell_rc.borrow_mut().self_ref = Rc::downgrade(&shell_rc);

        shell_rc
    }

    pub fn get_var(&self, name: &str) -> String {
        match self.vars.get(name) {
            Some(var) => var.value.clone(),
            None => String::new(),
        }
    }

    pub fn set_var(&mut self, name: &str, value: &str) {
        match self.vars.get_mut(name) {
            Some(var) => {
                var.value = value.to_string();
                if var.exported {
                    unsafe {
                        std::env::set_var(name, value.to_string());
                    }
                }
            },
            None => {
                self.vars.insert(name.to_string(), ShellVariable {
                    value: value.to_string(),
                    exported: false,
                });
            }
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
                });
                unsafe {
                    std::env::set_var(name, String::new());
                }
            }
        }
    }

    pub fn unset_var(&mut self, name: &str) {
        self.vars.remove(name);
        unsafe {
            std::env::remove_var(name);
        }
    }

    pub fn get_rc(&self) -> Rc<RefCell<Shell>> {
        self.self_ref.upgrade().expect("Shell has been dropped")
    }

    fn handle_batch_mode(&self) -> ShellStatus {
        let mut input_lines = Vec::new();

        for line in io::stdin().lines() {
            match line {
                Ok(text) => input_lines.push(text),
                Err(e) => { return ShellStatus::fail(e); }
            }
        }
        ShellStatus::ok()
    }

    fn handle_fallback_mode(&self) -> ShellStatus {
        ShellStatus::ok()
    }

    fn handle_interactive_mode(&self, reader: &mut InputReader) -> ShellStatus {
        let mut stdout = io::stdout();

        macro_rules! draw_prompt {
            ($key:expr, $stdout:expr) => {{
                let prompt = self.get_var($key);
                if $stdout.write_all(prompt.as_bytes()).is_err() {
                    return ShellStatus::fail("failed to write to stdout");
                }
            }}
        }

        macro_rules! flush {
            ($stdout:expr) => {{
                if $stdout.flush().is_err() {
                    return ShellStatus::fail("failed to flush stdout");
                }
            }}
        }

        if reader.enable_raw_mode().is_err() {
            return self.handle_fallback_mode();
        }

        let mut buffer = Buffer::new();
        let mut last_cmd_code: u8 = 0;
        let mut should_clear_buffer = true;
        let exit_code: u8 = loop {
            if should_clear_buffer {
                draw_prompt!("PS1", stdout);
                flush!(stdout);
                buffer.clear();
            } else {
                draw_prompt!("PS2", stdout);
                flush!(stdout);
            }

            match reader.read_input(&mut buffer) {
                Ok(Some(())) => {},
                Ok(None) => {
                    if should_clear_buffer {
                        println!();
                        break last_cmd_code;
                    } else {
                        eprintln!(
                            "vish: Syntax error: Unterminated quoted string"
                        );
                        should_clear_buffer = true;
                        continue;
                    }
                },
                Err(e) => { return Err(e).into(); },
            }

            let (argv, quote_char) = match buffer.as_str() {
                Ok(text) => parse_argv(self.get_rc(), text),
                Err(e) => { return Err(e).into(); },
            };

            if argv.is_empty() {
                println!();
                continue;
            }

            match quote_char {
                Some('\'') | Some('"') => {
                    if let Err(e) = buffer.write(b"\n") {
                        return Err(e).into();
                    }
                    println!();
                    should_clear_buffer = false;
                    continue;
                },
                Some(c) => {
                    return ShellStatus::fail(
                        format!("{:?} is not a valid quote character", c)
                    );
                },
                None => { should_clear_buffer = true; }
            }

            println!();

            last_cmd_code = match argv[0].as_str() {
                "cd" => cmd::cd(argv),
                "pwd" => cmd::pwd(argv),
                "printf" => cmd::printf(argv),
                "echo" => cmd::echo(argv),
                "exec" => cmd::exec(argv, reader),
                "exit" => { break cmd::exit(argv, last_cmd_code); },
                "true" => 0,
                "false" => 1,
                _ => cmd::run_command(argv),
            };
        };

        match reader.disable_raw_mode() {
            Ok(_) => exit_code.into(),
            Err(e) => Err(e).into(),
        }
    }

    pub fn main() -> impl Termination {
        let shell = Shell::new();
        match InputReader::new() {
            Ok(mut reader) => shell.borrow().handle_interactive_mode(&mut reader),
            Err(_) => shell.borrow().handle_batch_mode(),
        }
    }
}

impl PartialEq for Shell {
    fn eq(&self, other: &Self) -> bool {
        self.real_pid == other.real_pid
    }
}

impl ShellStatus {
    pub fn code(code: i32) -> Self {
        Self { msg: String::new(), code }
    }

    pub fn ok() -> Self {
        Self { msg: String::new(), code: 0 }
    }
}

impl Debug for ShellStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut formatter = f.debug_struct("ShellStatus");
        if !self.msg.is_empty() {
            formatter.field("msg", &self.msg);
        }
        formatter.field("code", &self.code).finish()
    }
}

impl Display for ShellStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.code == 0 {
            write!(f, "ok")
        } else {
            write!(f, "{}", &self.msg)
        }
    }
}

impl<T: ToString> Fail<T> for ShellStatus {
    fn fail(msg: T) -> Self {
        Self { msg: msg.to_string(), code: 1 }
    }
}

impl From<()> for ShellStatus {
    fn from(_: ()) -> Self {
        Self::ok()
    }
}

impl From<i32> for ShellStatus {
    fn from(code: i32) -> Self {
        Self::code(code)
    }
}

impl From<u8> for ShellStatus {
    fn from(code: u8) -> Self {
        Self::code(code as i32)
    }
}

impl<E: ToString> From<Result<(), E>> for ShellStatus {
    fn from(res: Result<(), E>) -> Self {
        match res {
            Ok(_) => Self::ok(),
            Err(e) => Self::fail(&e.to_string()),
        }
    }
}

impl From<ExitStatus> for ShellStatus {
    fn from(status: ExitStatus) -> Self {
        match (status.code(), status.signal()) {
            (Some(code), None) => ShellStatus::code(code),
            (None, Some(signal)) => ShellStatus::code(signal),
            _ => ShellStatus::fail("Cannot retrieve process status"),
        }
    }
}

impl Termination for ShellStatus {
    fn report(self) -> ExitCode {
        if !self.msg.is_empty() {
            eprintln!("{}", self.msg);
        }
        ExitCode::from(self.code as u8)
    }
}
