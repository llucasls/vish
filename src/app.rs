use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Write};
use std::process::{ExitCode, ExitStatus, Termination};
use std::os::unix::process::ExitStatusExt;

use crate::vish::buffer::Buffer;
use crate::vish::command as cmd;
use crate::vish::environment::ShellEnvironment as Env;
use crate::vish::io::InputReader;
use crate::vish::string::parse_argv;

pub struct Shell {
    pub env: Env,
}

pub struct ShellStatus {
    msg: String,
    code: i32,
}

trait Fail<T> {
    fn fail(msg: T) -> Self;
}

impl Shell {
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
                let unset_value = String::new();
                let prompt = self
                    .env
                    .shell_variables
                    .get($key)
                    .unwrap_or(&unset_value);
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
                Ok(text) => parse_argv(text),
                Err(e) => { return Err(e).into(); },
            };

            if argv.is_empty() {
                println!();
                continue;
            }

            match quote_char {
                Some('\'') | Some('"') => {
                    match buffer.write(b"\n") {
                        Ok(_) => {},
                        Err(e) => { return Err(e).into(); }
                    };
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
        let shell = Shell { env: Env::new() };
        match InputReader::new() {
            Ok(mut reader) => shell.handle_interactive_mode(&mut reader),
            Err(_) => shell.handle_batch_mode(),
        }
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
