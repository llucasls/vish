use std::process::ExitCode;
use std::io;

use super::io::InputReader;

fn cleanup_input(reader: &mut InputReader) -> io::Result<()> {
    reader.disable_raw_mode()
}

pub fn handle_interactive_mode(reader: &mut InputReader) -> ExitCode {
    if let Err(_) = reader.enable_raw_mode() {
        return handle_fallback_mode();
    }

    // read and execute commands from terminal
    match cleanup_input(reader) {
        Ok(_) => 0.into(),
        Err(_) => 1.into(),
    }
}

pub fn handle_batch_mode() -> ExitCode {
    let mut input_lines = Vec::new();

    for line in io::stdin().lines() {
        match line {
            Ok(text) => input_lines.push(text),
            Err(e) => {
                eprintln!("{}", e);
                return 1.into();
            }
        }
    }

    // read and execute commands from script
    0.into()
}

pub fn handle_fallback_mode() -> ExitCode {
    eprintln!("Warning: Failed to disable canonical input mode.");
    // fallback to system's default
    // read and execute commands from terminal
    0.into()
}
