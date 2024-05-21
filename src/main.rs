use std::io::{self, Write};
use std::process::{Command, ExitCode};
use std::collections::HashMap;
use std::env;
use std::os::unix::process::CommandExt;

mod vish;

use vish::io::read_input;

fn print_prompt(shell_variables: &HashMap<String, String>) {
    match shell_variables.get("PS1") {
        Some(prompt) => eprint!("{}", prompt),
        None => eprint!(""),
    };
    let mut stdout = io::stdout();
    stdout.flush().unwrap();
}

fn get_args(input: String) -> Vec<String> {
    input.trim().split_whitespace().map(|s| s.to_string()).collect()
}

fn main() -> ExitCode {
    let mut shell_variables: HashMap<String, String> = HashMap::new();

    let ps1 = match env::var("PS1") {
        Ok(value) => value,
        Err(_) => "$ ".to_string(),
    };
    shell_variables.insert("PS1".to_string(), ps1.to_string());

    let mut exit_status: ExitCode = ExitCode::SUCCESS;

    macro_rules! exit {
        () => { exit_status = ExitCode::from(0u8); break; };
        ($code:literal) => {
            let code: i32 = $code % 255;
            exit_status = ExitCode::from(code as u8);
            break;
        };
        ($code:ident) => {
            let code: i32 = $code.parse().unwrap();
            exit_status = ExitCode::from(code as u8);
            break;
        };
    }

    loop {
        print_prompt(&shell_variables);

        let mut user_input = String::new();
        let input_size = read_input(&mut user_input);

        if input_size == -1 {
            break; // End of input (Ctrl+D)
        } else if user_input.trim().is_empty() {
            continue;
        }

        let cmdline_args = get_args(user_input);
        let command = cmdline_args[0].clone();

        match command.trim() {
            "exit" => {
                if cmdline_args.len() > 1 {
                    let code = cmdline_args[1].clone();
                    exit!(code);
                } else {
                    exit!();
                }
            }
            "exec" => {
                if cmdline_args.len() > 1 {
                    let new_command = &cmdline_args[1];
                    let argv = &cmdline_args[2..];
                    Command::new(new_command)
                        .args(argv)
                        .exec();
                }
            }
            _ => {
                let argv = &cmdline_args[1..];
                let status = Command::new(command)
                    .args(argv)
                    .spawn()
                    .expect("failed to execute command")
                    .wait()
                    .expect("failed to wait on child process");

                exit_status = match status.code() {
                    Some(code) => ExitCode::from(code as u8),
                    None => ExitCode::from(128),
                };
            }
        }

    }

    exit_status
}
