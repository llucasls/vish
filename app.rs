use std::process::{ExitCode, ExitStatus, Termination};
use std::error::Error;
use std::os::unix::process::ExitStatusExt;

fn main() -> impl Termination { App.main() }

struct App;

pub struct AppStatus {
    msg: String,
    code: i32,
}

trait Fail<T> {
    fn fail(msg: T) -> Self;
}

impl AppStatus {
    pub fn ok() -> Self {
        Self { msg: String::new(), code: 0 }
    }

    pub fn code(code: i32) -> Self {
        Self { msg: String::new(), code }
    }
}

impl Fail<String> for AppStatus {
    fn fail(msg: String) -> Self {
        Self { msg, code: 1 }
    }
}

impl Fail<&str> for AppStatus {
    fn fail(msg: &str) -> Self {
        Self { msg: msg.into(), code: 1 }
    }
}

impl Fail<&String> for AppStatus {
    fn fail(msg: &String) -> Self {
        Self { msg: msg.into(), code: 1 }
    }
}

impl Termination for AppStatus {
    fn report(self) -> ExitCode {
        if ! self.msg.is_empty() {
            eprintln!("{}", self.msg);
        }
        ExitCode::from(self.code as u8)
    }
}

impl From<()> for AppStatus {
    fn from(_: ()) -> Self {
        Self::ok()
    }
}

impl From<Result<(), Box<dyn Error>>> for AppStatus {
    fn from(res: Result<(), Box<dyn Error>>) -> Self {
        match res {
            Ok(()) => Self::ok(),
            Err(e) => Self::fail(&e.to_string()),
        }
    }
}

impl From<ExitStatus> for AppStatus {
    fn from(status: ExitStatus) -> Self {
        match status.code() {
            Some(code) => AppStatus::code(code),
            None => {
                match status.signal() {
                    Some(signal) => AppStatus::code(signal),
                    _ => AppStatus::fail("Cannot retrieve process status"),
                }
            }
        }
    }
}

impl App {
    fn main(self) -> impl Termination {
    }
}
