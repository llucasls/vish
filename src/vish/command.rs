use std::process::Command;
use std::os::unix::process::CommandExt;
use std::io::ErrorKind::{NotFound, PermissionDenied, InvalidInput};
use std::env::{self, set_current_dir, current_dir};
use std::cmp::Ordering;
use std::ffi::OsString;
use std::fmt::format;
use std::path::PathBuf;

use super::buffer::Buffer;
use super::io::InputReader;


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

macro_rules! error_msg {
    ($err:ident) => { $err.to_string().split(" (").collect::<Vec<_>>()[0] }
}

pub fn run_command(argv: ArgV) -> u8 {
    let size = argv.len();
    let mut command = Command::new(argv[0].clone());
    if size > 1 {
        command.args(argv[1..].to_vec());
    }
    match command.status() {
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
            print!("{} ", arg.to_string());
        }
        print!("{}", argv[size - 1].to_string());
    }
    println!();
    0
}

pub fn exec(argv: ArgV, reader: &mut InputReader) -> u8 {
    if argv.len() < 2 {
        eprintln!("vish: exec: no command passed to exec");
        return 1;
    } else if reader.disable_raw_mode().is_err() {
        eprintln!("vish: exec: failed to cleanup environment...");
        return 1;
    }

    let err = Command::new(argv[1].clone())
        .args(&argv[2..])
        .exec();
    if reader.enable_raw_mode().is_err() {
        eprintln!("warning: failed to reactivate raw mode");
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
