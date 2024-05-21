use std::io::{self, Read, Write};
use termios::{tcsetattr, Termios, ECHO, ICANON, TCSANOW, VEOF};

const NEWLINE: u8 = b'\n';
const BACKSPACE: u8 = 127;
const ESCAPE: u8 = b'\x1b';

fn delete_previous_char() {
    print!("\x08 \x08");
}

pub fn read_input(input: &mut String) -> isize {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut termios = Termios::from_fd(0).unwrap();
    let original_termios = termios.clone();

    let eof_char = termios.c_cc[VEOF];

    termios.c_lflag &= !(ICANON | ECHO);
    tcsetattr(0, TCSANOW, &termios).unwrap();

    let mut count: isize = 0;
    let mut escape_sequence = false;
    for byte in stdin.bytes() {
        let byte = byte.unwrap();
        if escape_sequence {
            if byte == b'[' {
                continue;
            } else if byte >= b'A' && byte <= b'D' {
                escape_sequence = false;
                continue;
            }
        } else if byte == ESCAPE {
            escape_sequence = true;
            continue;
        } else if byte == NEWLINE {
            break;
        } else if byte == eof_char {
            if count == 0 {
                tcsetattr(0, TCSANOW, &original_termios).unwrap();
                return -1;
            }
        } else if byte == BACKSPACE {
            if !input.is_empty() {
                input.pop();
                count -= 1;
                delete_previous_char();
                stdout.flush().unwrap();
            }
        } else {
            input.push(byte as char);
            count += 1;
            print!("{}", byte as char);
            stdout.flush().unwrap();
        }
    }

    tcsetattr(0, TCSANOW, &original_termios).unwrap();
    count
}
