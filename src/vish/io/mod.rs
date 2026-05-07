use std::io::{self, Read, Write, Cursor};
use std::mem::replace;

use termios::*;
use termios::os::target::{VWERASE, VREPRINT, VLNEXT};

use crate::{kill_line, move_cursor, reprint_line};

const NEWLINE: u8 = b'\n';

const UP: u8 = b'A';
const DOWN: u8 = b'B';
const LEFT: u8 = b'D';
const RIGHT: u8 = b'C';

fn handle_werase_byte(bytes: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    if bytes.is_empty() {
        return bytes;
    }

    let is_whitespace = |c: Vec<u8>| c.iter().all(|b| b.is_ascii_whitespace());
    let mut new_bytes = bytes.clone();

    for utf8_char in bytes.iter().rev() {
        if !is_whitespace(utf8_char.to_vec()) { break; }
        new_bytes.pop();
    }

    for utf8_char in new_bytes.clone().iter().rev() {
        if is_whitespace(utf8_char.to_vec()) { break; }
        new_bytes.pop();
    }

    new_bytes
}

macro_rules! delete_previous_char {
    ($index:expr, $vector:expr, $stdout:expr) => {{
        $index -= 1;
        $vector.remove($index);
        $stdout.write_all(b"\x08")?;
        reprint_line!($stdout, $vector);
        $stdout.flush()?;
        continue;
    }}
}

macro_rules! delete_previous_word {
    ($index:expr, $vector:expr, $stdout:expr) => {{
        let previous_length = $vector.len();
        $vector = handle_werase_byte($vector);
        let resulting_length = $vector.len();
        let diff = previous_length - resulting_length;
        $index -= diff;
        let mut stdout = io::stdout();
        move_cursor!(-(diff as isize), $stdout);
        reprint_line!(stdout, $vector);
        $stdout.flush()?;
        continue;
    }}
}

macro_rules! erase_line {
    ($index:expr, $outer_vector:expr, $inner_vector:expr, $stdout:expr) => {{
        kill_line!($stdout);
        $stdout.flush()?;
        $outer_vector.clear();
        $inner_vector.clear();
        $index = 0;
        continue;
    }}
}

macro_rules! delete_char {
    ($index:expr, $vector:expr, $stdout:expr) => {{
        if $index < $vector.len() {
            $vector.remove($index);
            reprint_line!($stdout, &$vector);
            $stdout.flush()?;
        }
        continue;
    }}
}

macro_rules! go_to_start {
    ($index:expr, $stdout:expr) => {{
        let ps1 = match $crate::ENV.read() {
            Ok(shell) => shell.get_var("PS1").unwrap_or_default(),
            Err(_) => String::new(),
        };
        $stdout.write_all(b"\r")?;
        move_cursor!(ps1.len(), $stdout);
        $stdout.flush()?;
        $index = 0;
        continue;
    }}
}

macro_rules! go_to_end {
    ($index:expr, $vector:expr, $stdout:expr) => {{
        move_cursor!(($vector.len() - $index), $stdout);
        $stdout.flush()?;
        $index = $vector.len();
        continue;
    }}
}

macro_rules! store_byte {
    ($byte:expr, $vector:expr) => {{
        $vector.push($byte);
        continue;
    }}
}

macro_rules! move_left {
    ($index:expr, $inner_vector:expr, $stdout:expr) => {{
        $inner_vector.clear();
        if $index > 0 {
            move_cursor!(-1, $stdout);
            $stdout.flush()?;
            $index -= 1;
        }
        continue;
    }}
}

macro_rules! move_right {
    ($index:expr, $inner_vector:expr, $outer_vector:expr, $stdout:expr) => {{
        $inner_vector.clear();
        if $index < $outer_vector.len() {
            move_cursor!(1, $stdout);
            $stdout.flush()?;
            $index += 1;
        }
        continue;
    }}
}

pub enum ReadAction {
    Line,
    Eof,
}

pub struct Terminal {
    pub termios: Termios,
    pub default: Termios,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        let termios = Termios::from_fd(0)?;
        let default = termios;

        Ok(Self { termios, default })
    }

    pub fn disable_raw_mode(&self) -> io::Result<()> {
        if let Err(e) = tcsetattr(0, TCSANOW, &self.default) {
            std::process::Command::new("stty").arg("sane").status()?;
            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
        self.termios.c_lflag &= !ICANON & !ECHO & !IEXTEN;
        tcsetattr(0, TCSANOW, &self.termios)
    }

    pub fn read_input<R, W>(
        &mut self,
        buffer: &mut Cursor<Vec<u8>>,
        stdin: R,
        stdout: &mut W,
    ) -> io::Result<ReadAction>
    where
        R: Read,
        W: Write,
    {
        let eof_char = self.termios.c_cc[VEOF];
        let erase_char = self.termios.c_cc[VERASE];
        let werase_char = self.termios.c_cc[VWERASE];
        let kill_char = self.termios.c_cc[VKILL];
        let reprint_char = self.termios.c_cc[VREPRINT];
        let lnext_char = self.termios.c_cc[VLNEXT];

        let mut outer_vector: Vec<Vec<u8>> = Vec::with_capacity(256);
        let mut inner_vector: Vec<u8> = Vec::with_capacity(4);
        let mut index: usize = 0;
        let mut insert_literal: bool = false;

        #[allow(clippy::unbuffered_bytes)]
        for byte_result in stdin.bytes() {
            let byte = byte_result?;

            match byte {
                _ if insert_literal => {
                    inner_vector.push(byte);
                    if std::str::from_utf8(inner_vector.as_slice()).is_err() {
                        continue;
                    } else if index < outer_vector.len() {
                        reprint_line!(stdout, &outer_vector);
                        move_cursor!(1, stdout);
                        let new_vec = Vec::with_capacity(4);
                        let bytes = replace(&mut inner_vector, new_vec);
                        outer_vector.insert(index, bytes);
                    } else {
                        stdout.write_all(inner_vector.as_slice())?;
                        let new_vec = Vec::with_capacity(4);
                        let bytes = replace(&mut inner_vector, new_vec);
                        outer_vector.push(bytes);
                    }
                    stdout.flush()?;
                    index += 1;
                    insert_literal = false;
                },
                b if b == lnext_char => {
                    insert_literal = true;
                },
                b if b == erase_char && index == 0 => { continue; },
                0x08 | 0x7f if index == 0 => { continue; },
                b if b == erase_char || b == 0x08 || b == 0x7f => {
                    delete_previous_char!(index, outer_vector, stdout)
                },
                b if b == werase_char => delete_previous_word!(
                    index, outer_vector, stdout
                ),
                b if b == kill_char => erase_line!(
                    index, outer_vector, inner_vector, stdout
                ),
                b if b == reprint_char => reprint_line!(
                    stdout, outer_vector
                ),
                b if b == eof_char && outer_vector.is_empty() => {
                    return Ok(ReadAction::Eof);
                },
                b if b == eof_char => delete_char!(
                    index, outer_vector, stdout
                ),
                0x01 => go_to_start!(index, stdout),
                0x05 => go_to_end!(index, outer_vector, stdout),
                0x1b => store_byte!(byte, inner_vector),
                b'[' if inner_vector.as_slice() == b"\x1b" => store_byte!(
                    byte, inner_vector
                ),
                UP if matches!(inner_vector.as_slice(), b"\x1b[") => {
                    inner_vector.clear();
                    continue;
                },
                DOWN if matches!(inner_vector.as_slice(), b"\x1b[") => {
                    inner_vector.clear();
                    continue;
                },
                RIGHT if matches!(inner_vector.as_slice(), b"\x1b[") => {
                    move_right!(index, inner_vector, outer_vector, stdout);
                },
                LEFT if matches!(inner_vector.as_slice(), b"\x1b[") => {
                    move_left!(index, inner_vector, stdout);
                },
                NEWLINE => { break; },
                b if b < 0x20 => { continue; },
                _ => {
                    inner_vector.push(byte);
                    if std::str::from_utf8(inner_vector.as_slice()).is_err() {
                        continue;
                    } else if index < outer_vector.len() {
                        let new_vec = Vec::with_capacity(4);
                        let bytes = replace(&mut inner_vector, new_vec);
                        outer_vector.insert(index, bytes);
                        reprint_line!(stdout, &outer_vector);
                        move_cursor!(1, stdout);
                    } else {
                        let new_vec = Vec::with_capacity(4);
                        let bytes = replace(&mut inner_vector, new_vec);
                        stdout.write_all(&bytes)?;
                        outer_vector.push(bytes);
                    }
                    stdout.flush()?;
                    index += 1;
                }
            }
        }

        for vec in outer_vector {
            if let Ok(s) = std::str::from_utf8(&vec) {
                buffer.write_all(s.as_bytes())?;
            }
        }

        Ok(ReadAction::Line)
    }
}

#[cfg(test)]
mod test_input_reader {
    use super::*;
    use std::fmt;
    use std::io::Cursor;

    #[repr(transparent)]
    #[derive(PartialEq)]
    struct ByteStr(Vec<u8>);

    impl<B: AsRef<[u8]>> From<B> for ByteStr {
        fn from(value: B) -> Self {
            Self(Vec::from(value.as_ref()))
        }
    }

    impl fmt::Display for ByteStr {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for byte in &self.0 {
                match byte {
                    b' ' => write!(f, " ")?,
                    b'\t' => write!(f, r"\t")?,
                    b'\n' => write!(f, r"\n")?,
                    b'\r' => write!(f, r"\r")?,
                    0x0B => write!(f, r"\v")?,
                    0x0C => write!(f, r"\f")?,
                    0x1b => write!(f, r"\e")?,
                    b'\\' => write!(f, r"\\")?,
                    b'"'  => write!(f, "\\\"")?,

                    0x20..=0x7E => write!(f, "{}", *byte as char)?,

                    _ => write!(f, r"\x{:02x}", *byte)?,
                };
            }
            Ok(())
        }
    }

    macro_rules! assert_eq_bytes {
        ($value:expr, $expected:expr) => {
            assert_eq!(ByteStr::from($value), ByteStr::from($expected))
        };
        ($value:expr, $expected:expr, $msg:expr) => {
            assert_eq!(ByteStr::from($value), ByteStr::from($expected), $msg)
        };
    }

    impl fmt::Debug for ByteStr {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "b\"{}\"", self)
        }
    }

    #[test]
    fn reads_line_and_echoes() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo hello world\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo hello world", "bufer");
        assert_eq_bytes!(output, b"echo hello world", "output");
    }

    #[test]
    fn move_back_with_arrow() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo hello\x1b[D\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo hello", "buffer");
        // number 1 (byte 49) introduced by move_cursor!
        assert_eq_bytes!(output, b"echo hello\x1b[1D", "output");
    }

    #[test]
    fn delete_character_with_ctrl_d() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"ls\x1b[D\x1b[D\x04\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"s", "buffer");
        assert_eq_bytes!(output, b"ls\x1b[1D\x1b[1D\x1b[2K\r$ s", "output");
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(e) = self.disable_raw_mode() {
            eprintln!("vish: failed to cleanup terminal: {:?}", e);
        }
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
            for byte in bytes { output_string.push(*byte); }
        }

        for bytes in &expected {
            for byte in bytes { expected_string.push(*byte); }
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
