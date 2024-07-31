use std::process::ExitCode;
use std::io::{self, Write};

use super::io::{InputReader, parse_argv};
use super::buffer::Buffer;
use super::command as cmd;
use super::environment::ShellEnvironment as Env;

fn cleanup_input(reader: &mut InputReader) -> io::Result<()> {
    reader.disable_raw_mode()
}

fn draw_prompt(stdout: &mut io::Stdout, env: &Env) -> io::Result<()> {
    let unset_value = String::new();
    let ps1 = env.shell_variables.get("PS1").unwrap_or(&unset_value);
    stdout.write_all(ps1.as_bytes())?;
    stdout.flush()
}

fn draw_newline(stdout: &mut io::Stdout) -> io::Result<()> {
    stdout.write_all(b"\n")?;
    stdout.flush()
}

macro_rules! draw_newline {
    ($stdout:expr) => {
        if draw_newline(&mut $stdout).is_err() { return 1.into(); }
    };
}

pub fn handle_interactive_mode(reader: &mut InputReader, env: Env) -> ExitCode {
    if reader.enable_raw_mode().is_err() {
        return handle_fallback_mode();
    }

    let mut buffer = Buffer::new();
    let mut stdout = io::stdout();
    let mut last_cmd_code: u8 = 0;
    let mut should_clear_buffer = true;
    let exit_code: u8 = loop {
        if draw_prompt(&mut stdout, &env).is_err() { return 1.into(); }

        if should_clear_buffer {
            buffer.clear();
        }
        match reader.read_input(&mut buffer) {
            Ok(Some(())) => {},
            Ok(None) => { draw_newline!(stdout); break last_cmd_code; },
            Err(e) => { eprintln!("{}", e); return 1.into(); },
        }

        let (argv, quote_char) = match buffer.as_str() {
            Ok(text) => parse_argv(text),
            Err(e) => {
                eprintln!("{}", e);
                return 1.into();
            }
        };

        match quote_char {
            Some('\'') => {
                match buffer.write(b"\n") {
                    Ok(_) => {},
                    Err(_) => { return 1.into(); }
                };
                draw_newline!(stdout);
                should_clear_buffer = false;
                continue;
            },
            Some('"') => {
                match buffer.write(b"\n") {
                    Ok(_) => {},
                    Err(_) => { return 1.into(); }
                };
                draw_newline!(stdout);
                should_clear_buffer = false;
                continue;
            },
            Some(_) => { return 1.into(); },
            None => { should_clear_buffer = true; }
        }

        draw_newline!(stdout);

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

    match cleanup_input(reader) {
        Ok(_) => exit_code.into(),
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
