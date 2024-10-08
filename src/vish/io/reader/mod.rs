use std::io::{self, Read, Write, Stdin, Stdout};

use termios::*;
use termios::os::target::{VWERASE, VREPRINT};

use super::display::*;
use crate::vish::buffer::Buffer;

const NEWLINE: u8 = b'\n';

const UP: Option<Directional> = Some(Directional::Up);
const DOWN: Option<Directional> = Some(Directional::Down);
const LEFT: Option<Directional> = Some(Directional::Left);
const RIGHT: Option<Directional> = Some(Directional::Right);

#[derive(Eq, PartialEq)]
enum Directional {
    Up,
    Down,
    Left,
    Right,
}

fn moved(input: Vec<u8>) -> Option<Directional> {
    match input.as_slice() {
        b"\x1b[A" => UP,
        b"\x1b[B" => DOWN,
        b"\x1b[C" => RIGHT,
        b"\x1b[D" => LEFT,
        _ => None,
    }
}

fn handle_werase_byte(bytes: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    if bytes.is_empty() {
        return bytes;
    }

    let is_whitespace = |b: Vec<u8>| b.iter().all(|b| b.is_ascii_whitespace());
    let mut new_bytes = bytes.clone();

    for byte_array in bytes.iter().rev() {
        if !is_whitespace(byte_array.to_vec()) { break; }
        new_bytes.pop();
    }

    for byte_array in new_bytes.clone().iter().rev() {
        if is_whitespace(byte_array.to_vec()) { break; }
        new_bytes.pop();
    }

    new_bytes
}

pub struct InputReader {
    termios: Termios,
    default: Termios,
    stdin: Stdin,
    stdout: Stdout,
}

impl InputReader {
    pub fn new() -> io::Result<Self> {
        let termios = Termios::from_fd(0)?;
        let default = termios;
        let stdin = io::stdin();
        let stdout = io::stdout();

        Ok(Self { termios, default, stdin, stdout })
    }

    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
        self.termios.c_lflag &= !ICANON & !ECHO;
        tcsetattr(0, TCSANOW, &self.termios)
    }

    pub fn disable_raw_mode(&self) -> io::Result<()> {
        if let Err(e) = tcsetattr(0, TCSANOW, &self.default) {
            std::process::Command::new("stty").arg("sane").status()?;
            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn read_input(&mut self, buffer: &mut Buffer) -> io::Result<Option<()>> {
        let eof_char = self.termios.c_cc[VEOF];
        let erase_char = self.termios.c_cc[VERASE];
        let werase_char = self.termios.c_cc[VWERASE];
        let kill_char = self.termios.c_cc[VKILL];
        let reprint_char = self.termios.c_cc[VREPRINT];

        let mut outer_vector: Vec<Vec<u8>> = Vec::new();
        let mut inner_vector: Vec<u8> = Vec::new();

        for byte_result in self.stdin.lock().bytes() {
            let byte = byte_result?;

            if byte == erase_char {
                outer_vector.pop();
                delete_previous_char()?;
                continue;
            }

            if byte == werase_char {
                outer_vector = handle_werase_byte(outer_vector);
                reprint_line(&outer_vector)?;
                continue;
            }

            if byte == kill_char {
                kill_line()?;
                outer_vector.clear();
                inner_vector.clear();
                continue;
            }

            if byte == reprint_char {
                reprint_line(&outer_vector)?;
                continue;
            }

            if byte == 27 {
                inner_vector.push(byte);
                continue;
            }

            if byte == 91 && !inner_vector.is_empty() && inner_vector[0] == 27 {
                inner_vector.push(byte);
                continue;
            }

            if !inner_vector.is_empty() {
                match moved(vec![27, 91, byte]) {
                    UP => {
                        inner_vector.clear();
                        continue;
                    },
                    DOWN => {
                        inner_vector.clear();
                        continue;
                    },
                    LEFT => {
                        inner_vector.clear();
                        move_left()?;
                        continue;
                    },
                    RIGHT => {
                        inner_vector.clear();
                        move_right()?;
                        continue;
                    },
                    _ => {},
                }
            }

            if byte == eof_char {
                return Ok(None);
            }

            if byte == NEWLINE {
                break;
            }

            inner_vector.push(byte);
            if std::str::from_utf8(inner_vector.as_slice()).is_ok() {
                outer_vector.push(inner_vector.clone());
                self.stdout.write_all(inner_vector.as_slice())?;
                self.stdout.flush()?;
                inner_vector.clear();
            }
        }

        for vec in outer_vector {
            for byte in vec {
                buffer.write(&[byte])?;
            }
        }

        Ok(Some(()))
    }
}

#[cfg(test)]
mod handle_werase_byte {
    use super::handle_werase_byte;

    const EMPTY: Vec<Vec<u8>> = vec![];

    fn parse_text(text: &str) -> Vec<Vec<u8>> {
        Vec::from_iter(text
            .split_inclusive(|_| true)
            .map(|string: &str| string.as_bytes().to_vec()))
    }

    fn parse_bytes(bytes: &[u8]) -> Vec<Vec<u8>> {
        Vec::from_iter(bytes
            .split_inclusive(|byte: &u8| byte.is_ascii())
            .map(|byte_array: &[u8]| byte_array.to_vec()))
    }

    fn assert_handle_werase_byte(input: Vec<Vec<u8>>, expected: Vec<Vec<u8>>) {
        let output: Vec<Vec<u8>> = handle_werase_byte(input.clone());

        let mut input_string: Vec<u8> = Vec::new();
        let mut output_string: Vec<u8> = Vec::new();
        let mut expected_string: Vec<u8> = Vec::new();

        for bytes in input { for byte in bytes { input_string.push(byte); } }

        for bytes in &output {
            for byte in bytes { output_string.push(byte.clone()); }
        }

        for bytes in &expected {
            for byte in bytes { expected_string.push(byte.clone()); }
        }

        assert_eq!(output, expected,
            "\n   input: {:?}\n  output: {:?}\nexpected: {:?}\n",
            String::from_utf8(input_string).unwrap(),
            String::from_utf8(output_string).unwrap(),
            String::from_utf8(expected_string).unwrap());
    }

    #[test]
    fn erase_last_space_and_word() {
        let bytes1 = parse_bytes(b"This isn't a coke ");
        assert_handle_werase_byte(bytes1, parse_bytes(b"This isn't a "));

        let bytes2 = parse_bytes(b"It is passion fruit juice...  ");
        assert_handle_werase_byte(bytes2, parse_bytes(b"It is passion fruit "));
    }

    #[test]
    fn erase_last_word() {
        let bytes1 = parse_bytes(b"morango melancia abacaxi");
        assert_handle_werase_byte(bytes1, parse_bytes(b"morango melancia "));

        let bytes2 = parse_text(
            "cérebro e coração são órgãos do corpo humano");
        assert_handle_werase_byte(bytes2,
            parse_text("cérebro e coração são órgãos do corpo "));

        let bytes3 = parse_text("suco  de  maracujá");
        assert_handle_werase_byte(bytes3, parse_bytes(b"suco  de  "));
    }

    #[test]
    fn erase_first_space() {
        let bytes1 = parse_bytes(b" ");
        assert_handle_werase_byte(bytes1, EMPTY);

        let bytes2 = parse_bytes(b"    ");
        assert_handle_werase_byte(bytes2, EMPTY);

        let bytes3 = parse_bytes(b"\t");
        assert_handle_werase_byte(bytes3, EMPTY);
    }

    #[test]
    fn erase_first_word() {
        let bytes1 = parse_text("morango");
        assert_handle_werase_byte(bytes1, EMPTY);

        let bytes2 = parse_text("maçã");
        assert_handle_werase_byte(bytes2, EMPTY);

        let bytes3 = parse_text("maracujá");
        assert_handle_werase_byte(bytes3, EMPTY);
    }

    #[test]
    fn do_nothing() {
        let bytes = EMPTY;
        assert_handle_werase_byte(bytes, EMPTY);
    }
}
