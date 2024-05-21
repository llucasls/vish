use std::io;
use std::process::{Command,ExitCode};
use std::collections::HashMap;
use std::env;
use std::os::unix::process::CommandExt;

fn print_prompt(shell_variables: &HashMap<String, String>) {
    match shell_variables.get("PS1") {
        Some(prompt) => eprint!("{}", prompt),
        None => eprint!(""),
    };
}

fn read_input(user_input: &mut String) -> usize {
    let stdin = io::stdin();
    match stdin.read_line(user_input) {
        Ok(v) => v,
        Err(_) => panic!("Can't read user input."),
    }
}

fn get_args(input: String) -> Vec<String> {
    let mut cmdline_args: Vec<String> = vec![];
    for arg in input.trim().split_whitespace() {
        cmdline_args.push(arg.to_string());
    }
    cmdline_args
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

        if input_size == 0 {
            break;
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
                    .expect("failed")
                    .wait()
                    .unwrap();

                match status.code() {
                    Some(code) => { exit_status = ExitCode::from(code as u8); },
                    None => { exit_status = ExitCode::from(128); },
                };
            }
        }

    }

    exit_status
}
