use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Write};
use std::process::{ExitCode, ExitStatus, Termination};
use std::os::unix::process::ExitStatusExt;

use crate::vish::buffer::Buffer;
use crate::vish::command::{self as cmd};
use crate::vish::io::InputReader;
use crate::vish::string::parse_argv;

pub struct App;

pub struct AppStatus {
    msg: String,
    code: i32,
}

trait Fail<T> {
    fn fail(msg: T) -> Self;
}

impl App {
    fn handle_batch_mode() -> AppStatus {
        let mut input_lines = Vec::new();

        for line in io::stdin().lines() {
            match line {
                Ok(text) => input_lines.push(text),
                Err(e) => { return AppStatus::fail(e); }
            }
        }
        AppStatus::ok()
    }

    fn handle_fallback_mode() -> AppStatus {
        AppStatus::ok()
    }

    fn handle_interactive_mode(reader: &mut InputReader) -> AppStatus {
        let mut stdout = io::stdout();

        macro_rules! draw_prompt {
            ($key:expr, $stdout:expr) => {{
                match crate::ENV.read() {
                    Ok(shell) => {
                        let prompt = shell.get_var($key).unwrap_or_default();
                        if $stdout.write_all(prompt.as_bytes()).is_err() {
                            return AppStatus::fail("failed to write to stdout");
                        }
                    },
                    Err(e) => { return AppStatus::fail(e); },
                }
            }}
        }

        macro_rules! flush {
            ($stdout:expr) => {{
                if $stdout.flush().is_err() {
                    return AppStatus::fail("failed to flush stdout");
                }
            }}
        }

        if reader.enable_raw_mode().is_err() {
            return Self::handle_fallback_mode();
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
                Err(e) => {
                    should_clear_buffer = true;
                    println!();
                    eprintln!("{}", e);
                    last_cmd_code = 2;
                    continue;
                },
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
                    return AppStatus::fail(
                        format!("{:?} is not a valid quote character", c)
                    );
                },
                None => { should_clear_buffer = true; }
            }

            println!();

            last_cmd_code = match argv[0].as_str() {
                input if input.split('=').collect::<Vec<&str>>().len() == 2 => {
                    cmd::set_var(argv)
                },
                "cd" => cmd::cd(argv),
                "pwd" => cmd::pwd(argv),
                "printf" => cmd::printf(argv),
                "echo" => cmd::echo(argv),
                "exec" => cmd::exec(argv, reader),
                "exit" => { break cmd::exit(argv, last_cmd_code); },
                "true" => 0,
                "false" => 1,
                "export" => cmd::export(argv),
                "readonly" => cmd::readonly(argv),
                "unset" => cmd::unset(argv),
                _ => cmd::run_command(argv),
            };

            match crate::ENV.write() {
                Ok(mut shell) => { shell.status = last_cmd_code; },
                Err(e) => { return AppStatus::fail(e); },
            }
        };

        exit_code.into()
    }

    pub fn main() -> impl Termination {
        match InputReader::new() {
            Ok(mut reader) => Self::handle_interactive_mode(&mut reader),
            Err(_) => Self::handle_batch_mode(),
        }
    }
}

impl AppStatus {
    pub fn code(code: i32) -> Self {
        Self { msg: String::new(), code }
    }

    pub fn ok() -> Self {
        Self { msg: String::new(), code: 0 }
    }
}

impl Debug for AppStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut formatter = f.debug_struct("AppStatus");
        if !self.msg.is_empty() {
            formatter.field("msg", &self.msg);
        }
        formatter.field("code", &self.code).finish()
    }
}

impl Display for AppStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.code == 0 {
            write!(f, "ok")
        } else {
            write!(f, "{}", &self.msg)
        }
    }
}

impl<T: ToString> Fail<T> for AppStatus {
    fn fail(msg: T) -> Self {
        Self { msg: msg.to_string(), code: 1 }
    }
}

impl From<()> for AppStatus {
    fn from(_: ()) -> Self {
        Self::ok()
    }
}

impl From<i32> for AppStatus {
    fn from(code: i32) -> Self {
        Self::code(code)
    }
}

impl From<u8> for AppStatus {
    fn from(code: u8) -> Self {
        Self::code(code as i32)
    }
}

impl<T: Into<AppStatus>> From<Option<T>> for AppStatus {
    fn from(status: Option<T>) -> Self {
        match status {
            Some(value) => value.into(),
            None => Self::code(1),
        }
    }
}

impl<E: ToString> From<Result<(), E>> for AppStatus {
    fn from(res: Result<(), E>) -> Self {
        match res {
            Ok(_) => Self::ok(),
            Err(e) => Self::fail(&e.to_string()),
        }
    }
}

impl From<ExitStatus> for AppStatus {
    fn from(status: ExitStatus) -> Self {
        match (status.code(), status.signal()) {
            (Some(code), None) => AppStatus::code(code),
            (None, Some(signal)) => AppStatus::code(signal + 128),
            _ => AppStatus::fail("Cannot retrieve process status"),
        }
    }
}

impl Termination for AppStatus {
    fn report(self) -> ExitCode {
        if !self.msg.is_empty() {
            eprintln!("{}", self.msg);
        }
        ExitCode::from(self.code as u8)
    }
}
