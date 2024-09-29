use std::process::ExitCode;
use std::io::{self, Write};

use super::io::InputReader;
use super::buffer::Buffer;
use super::command as cmd;
use super::environment::ShellEnvironment as Env;
use super::string::parse_argv;

fn cleanup_input(reader: &mut InputReader) -> io::Result<()> {
    reader.disable_raw_mode()
}

fn draw_prompt(stdout: &mut io::Stdout, env: &Env, key: &str) -> io::Result<()> {
    let unset_value = String::new();
    let prompt = env.shell_variables.get(key).unwrap_or(&unset_value);
    stdout.write_all(prompt.as_bytes())?;
    stdout.flush()
}

fn draw_newline(stdout: &mut io::Stdout) -> io::Result<()> {
    stdout.write_all(b"\n")?;
    stdout.flush()
}

pub fn handle_interactive_mode(reader: &mut InputReader, env: Env) -> ExitCode {
    let mut stdout = io::stdout();

    macro_rules! draw_prompt {
        ($key:expr) => {
            if draw_prompt(&mut stdout, &env, $key).is_err() {
                return 1.into();
            }
        };
    }

    macro_rules! draw_newline {
        () => {
            if draw_newline(&mut stdout).is_err() { return 1.into(); }
        };
    }

    if reader.enable_raw_mode().is_err() {
        return handle_fallback_mode();
    }

    let mut buffer = Buffer::new();
    let mut last_cmd_code: u8 = 0;
    let mut should_clear_buffer = true;
    let exit_code: u8 = loop {
        if should_clear_buffer {
            draw_prompt!("PS1");
            buffer.clear();
        } else {
            draw_prompt!("PS2");
        }
        match reader.read_input(&mut buffer) {
            Ok(Some(())) => {},
            Ok(None) => {
                if should_clear_buffer {
                    draw_newline!();
                    break last_cmd_code;
                } else {
                    eprintln!("vish: Syntax error: Unterminated quoted string");
                    should_clear_buffer = true;
                    continue;
                }
            },
            Err(e) => { eprintln!("{}", e); return 1.into(); },
        }

        let (argv, quote_char) = match buffer.as_str() {
            Ok(text) => parse_argv(text),
            Err(e) => {
                eprintln!("{}", e);
                return 1.into();
            }
        };

        if argv.is_empty() {
            draw_newline!();
            continue;
        }

       match quote_char {
            Some('\'') | Some('"') => {
                match buffer.write(b"\n") {
                    Ok(_) => {},
                    Err(_) => { return 1.into(); }
                };
                draw_newline!();
                should_clear_buffer = false;
                continue;
            },
            Some(_) => { return 1.into(); },
            None => { should_clear_buffer = true; }
        }

        draw_newline!();

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
