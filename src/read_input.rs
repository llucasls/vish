use std::io::{self, Read, Write};
use termios::{tcsetattr, Termios, ECHO, ICANON, TCSANOW, VEOF};

pub fn read_input(input: &mut String) -> isize {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut termios = Termios::from_fd(0).unwrap();
    let original_termios = termios.clone();

    let eof_char = termios.c_cc[VEOF];

    // Disable canonical mode and echo
    termios.c_lflag &= !(ICANON | ECHO);
    tcsetattr(0, TCSANOW, &termios).unwrap();

    let mut count: isize = 0;
    let mut escape_sequence = false;
    for byte in stdin.bytes() {
        let byte = byte.unwrap();
        if escape_sequence {
            // Check for the second and third characters of the escape sequence
            if byte == b'[' {
                // Consume and ignore the rest of the escape sequence
                continue;
            } else if byte >= b'A' && byte <= b'D' {
                escape_sequence = false;
                continue;
            }
        } else if byte == b'\x1b' {
            // Start of an escape sequence
            escape_sequence = true;
            continue;
        } else if byte == b'\n' {
            break;
        } else if byte == eof_char {
            if count == 0 {
                tcsetattr(0, TCSANOW, &original_termios).unwrap();
                return -1; // Signal that Ctrl+D was pressed with no input
            }
        } else if byte == 127 {
            // Handle backspace (127 is ASCII for backspace)
            if !input.is_empty() {
                input.pop();
                count -= 1;
                print!("\x08 \x08"); // Move cursor back, print space, move cursor back again
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
