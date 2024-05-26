use std::process::{Command, ExitCode};

pub fn run_command(command: &mut Command) -> ExitCode {
    let status_result = command.status();
    let mut code: u8 = 0;

    if let Err(_) = status_result {
        return 1.into();
    } else if let Ok(status) = status_result {
        code = status.code().unwrap_or(1) as u8;
    }

    code.into()
}

fn replace_escape_sequence(input: &Vec<u8>) -> Vec<u8> {
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

    while i < input.len() {
        if i <= size - hex_size && &input[i..i + hex_size] == hex_array {
            output.push(escape_byte);
            i += hex_size;
        } else if i <= size - oct_size && &input[i..i + oct_size] == oct_array {
            output.push(escape_byte);
            i += oct_size;
        } else if i <= size - char_size && &input[i..i + char_size] == char_array {
            output.push(escape_byte);
            i += char_size;
        } else {
            output.push(input[i]);
            i += 1;
        }
    }

    output
}
