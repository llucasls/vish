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

fn handle_werase_byte(mut bytes: Vec<Vec<u8>>, index: usize) -> Vec<Vec<u8>> {
    if bytes.is_empty() {
        return bytes;
    }

    // Find UTF-8 char position from byte offset
    let mut total = 0;
    let mut pos = bytes.len();

    for (i, ch) in bytes.iter().enumerate() {
        total += ch.len();

        if total >= index {
            pos = i + 1;
            break;
        }
    }

    pos = pos.min(bytes.len());

    // Skip trailing whitespace
    let mut start = pos;

    while start > 0
        && bytes[start - 1]
            .iter()
            .all(|b| b.is_ascii_whitespace())
    {
        start -= 1;
    }

    // Skip previous word
    while start > 0
        && !bytes[start - 1]
            .iter()
            .all(|b| b.is_ascii_whitespace())
    {
        start -= 1;
    }

    bytes.drain(start..pos);

    bytes
}

macro_rules! delete_previous_char {
    ($index:expr, $vector:expr, $stdout:expr) => {{
        $index -= 1;
        $vector.remove($index);
        let length = $vector.len() - $index;
        write!($stdout, "\x08\x1b[{}X", length + 1)?;
        for bytes in &$vector[$index..] {
            $stdout.write_all(bytes)?;
        }
        move_cursor!(-(length as isize), $stdout);
        $stdout.flush()?;
        continue;
    }}
}

macro_rules! delete_previous_word {
    ($index:expr, $vector:expr, $stdout:expr) => {{
        let previous_length = $vector.len();
        $vector = handle_werase_byte($vector, $index);
        let resulting_length = $vector.len();
        let diff = previous_length - resulting_length;
        $index -= diff;
        move_cursor!(-(diff as isize), $stdout);
        write!($stdout, "\x1b[{}X", previous_length - $index)?;
        for bytes in &$vector[$index..] {
            $stdout.write_all(bytes)?;
        }
        let remaining = $vector.len() - $index;
        move_cursor!(-(remaining as isize), $stdout);
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
            let length = $vector.len() - $index;
            write!($stdout, "\x1b[{}X", length + 1)?;
            for bytes in &$vector[$index..] {
                $stdout.write_all(bytes)?;
            }
            move_cursor!(-(length as isize), $stdout);
            $stdout.flush()?;
        }
        continue;
    }}
}

macro_rules! go_to_start {
    ($index:expr, $line:expr, $stdout:expr) => {{
        move_cursor!(-($index as isize), $stdout);
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
                        let new_vec = Vec::with_capacity(4);
                        let bytes = replace(&mut inner_vector, new_vec);
                        let length = outer_vector.len() - index;
                        write!(stdout, "\x1b[{}X", length)?;
                        stdout.write_all(&bytes)?;
                        for bytes in &outer_vector[index..] {
                            stdout.write_all(&bytes)?;
                        }
                        move_cursor!(-(length as isize), stdout);
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
                0x01 => go_to_start!(index, outer_vector, stdout),
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
                        let length = outer_vector.len() - index;
                        write!(stdout, "\x1b[{}X", length)?;
                        outer_vector.insert(index, bytes);
                        for bytes in &outer_vector[index..] {
                            stdout.write_all(bytes)?;
                        }
                        move_cursor!(-(length as isize), stdout);
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

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(e) = self.disable_raw_mode() {
            eprintln!("vish: failed to cleanup terminal: {:?}", e);
        }
    }
}

#[cfg(test)]
mod test_input_reader {
    use super::*;
    use std::io::Cursor;
    use crate::testing::ByteStr;
    use crate::assert_eq_bytes;

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
        assert_eq_bytes!(output, b"ls\x1b[1D\x1b[1D\x1b[2Xs\x1b[1D", "output");
    }

    #[test]
    fn move_back_and_insert_character() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo hat\x1b[D\x1b[D\x1b[Dt\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo that", "buffer");
        assert_eq_bytes!(output, b"echo hat\x1b[1D\x1b[1D\x1b[1D\x1b[3Xthat\x1b[3D", "output");
    }

    #[test]
    fn move_back_and_delete_character() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo that\x1b[D\x1b[D\x1b[D\x7f\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo hat", "buffer");
        assert_eq_bytes!(output, b"echo that\x1b[1D\x1b[1D\x1b[1D\x08\x1b[4Xhat\x1b[3D", "output");
    }

    #[test]
    fn move_back_and_delete_character_with_ctrl_d() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo that\x1b[D\x1b[D\x1b[D\x1b[D\x04\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo hat", "buffer");
        assert_eq_bytes!(output, b"echo that\x1b[1D\x1b[1D\x1b[1D\x1b[1D\x1b[4Xhat\x1b[3D", "output");
    }

    #[test]
    fn delete_previous_char() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo hat\x7f\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo ha", "buffer");
        assert_eq_bytes!(output, b"echo hat\x08\x1b[1X", "output");
    }

    #[test]
    fn delete_previous_word() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo hello world\x17\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo hello ", "buffer");
        assert_eq_bytes!(output, b"echo hello world\x1b[5D\x1b[5X", "output");
    }

    #[test]
    fn delete_previous_word_middle() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"echo hello world\x1b[D\x1b[D\x1b[D\x1b[D\x1b[D\x1b[D\x17\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo  world", "buffer");
        assert_eq_bytes!(output, b"echo hello world\x1b[1D\x1b[1D\x1b[1D\x1b[1D\x1b[1D\x1b[1D\x1b[5D\x1b[11X world\x1b[6D", "output");
    }

    #[test]
    fn delete_previous_word_middle_utf8() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        // maçã
        let mut input = Cursor::new(b"echo ma\xc3\xa7\xc3\xa3\x1b[D\x17\n");
        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo \xc3\xa3", "buffer");
        assert_eq_bytes!(output, b"echo ma\xc3\xa7\xc3\xa3\x1b[1D\x1b[3D\x1b[4X\xc3\xa3\x1b[1D", "output");
    }

    #[test]
    fn insert_literal_ctrl_d_at_end() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        // Ctrl+V Ctrl+D
        let mut input = Cursor::new(b"echo test\x16\x04\n");

        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo test\x04", "buffer");
        assert_eq_bytes!(output, b"echo test\x04", "output");
    }

    #[test]
    fn insert_literal_ctrl_d_in_middle() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        // Move left 3, then Ctrl+V Ctrl+D
        let mut input = Cursor::new(
            b"echo hat\x1b[D\x1b[D\x1b[D\x16\x04\n"
        );

        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo \x04hat", "buffer");

        // Expected ideal incremental redraw
        assert_eq_bytes!(
            output, b"echo hat\x1b[1D\x1b[1D\x1b[1D\x1b[3X\x04hat\x1b[3D", "output");
    }

    #[test]
    fn insert_literal_ctrl_w() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        // Ctrl+V Ctrl+W
        let mut input = Cursor::new(
            b"echo hello\x16\x17\n"
        );

        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"echo hello\x17", "buffer");
        assert_eq_bytes!(output, b"echo hello\x17", "output");
    }

    #[test]
    fn insert_literal_after_cursor_move() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        let mut input = Cursor::new(b"abcd\x1b[D\x1b[D\x16x\n");

        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"abxcd", "buffer");
        assert_eq_bytes!(output, b"abcd\x1b[1D\x1b[1D\x1b[2Xxcd\x1b[2D", "output");
    }

    #[test]
    fn insert_literal_escape() {
        let mut terminal = Terminal::new().unwrap();
        let mut buffer = Cursor::new(Vec::new());

        // Ctrl+V ESC
        let mut input = Cursor::new(b"test\x16\x1b\n");

        let mut output = Vec::new();

        let action = terminal
            .read_input(&mut buffer, &mut input, &mut output)
            .unwrap();

        assert!(matches!(action, ReadAction::Line));
        assert_eq_bytes!(buffer.get_ref(), b"test\x1b", "buffer");
        assert_eq_bytes!(output, b"test\x1b", "output");
    }
}

#[cfg(test)]
mod test_handle_werase_byte {
    use super::*;
    use crate::testing::ByteVec;
    use crate::assert_eq_bytes_vec;

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

    fn assert_handle_werase_byte(
        input: Vec<Vec<u8>>,
        expected: Vec<Vec<u8>>,
        index: usize,
    ) {
        let output: Vec<Vec<u8>> = handle_werase_byte(input.clone(), index);

        let mut input_string: Vec<u8> = Vec::new();
        let mut output_string: Vec<u8> = Vec::new();
        let mut expected_string: Vec<u8> = Vec::new();

        for bytes in input {
            input_string.extend(bytes);
        }

        for bytes in &output {
            output_string.extend(bytes);
        }

        for bytes in &expected {
            expected_string.extend(bytes);
        }

        assert_eq_bytes_vec!(output, expected,
            "\n   input: ({:?}, {})\n  output: {:?}\nexpected: {:?}\n",
            String::from_utf8(input_string).unwrap(),
            index,
            String::from_utf8(output_string).unwrap(),
            String::from_utf8(expected_string).unwrap());
    }

    #[test]
    fn erase_word_in_middle() {
        let bytes = parse_bytes(b"one two three four five");
        assert_handle_werase_byte(bytes, parse_bytes(b"one two ee four five"), 11);
    }

    #[test]
    fn erase_last_space_and_word() {
        let bytes1 = parse_bytes(b"This isn't a coke ");
        let size1 = bytes1.len();
        assert_handle_werase_byte(bytes1, parse_bytes(b"This isn't a "), size1);

        let bytes2 = parse_bytes(b"It is passion fruit juice...  ");
        let size2 = bytes2.len();
        assert_handle_werase_byte(bytes2, parse_bytes(b"It is passion fruit "), size2);
    }

    #[test]
    fn erase_last_word() {
        let bytes1 = parse_bytes(b"morango melancia abacaxi");
        assert_handle_werase_byte(bytes1, parse_bytes(b"morango melancia "), 24);

        let bytes2 = parse_text(
            "cérebro e coração são órgãos do corpo humano");
        assert_handle_werase_byte(bytes2,
            parse_text("cérebro e coração são órgãos do corpo "), 50);

        let bytes3 = parse_text("suco  de  maracujá");
        assert_handle_werase_byte(bytes3, parse_bytes(b"suco  de  "), 19);
    }

    #[test]
    fn erase_first_space() {
        let bytes1 = parse_bytes(b" ");
        assert_handle_werase_byte(bytes1, EMPTY, 1);

        let bytes2 = parse_bytes(b"    ");
        assert_handle_werase_byte(bytes2, EMPTY, 4);

        let bytes3 = parse_bytes(b"\t");
        assert_handle_werase_byte(bytes3, EMPTY, 1);
    }

    #[test]
    fn erase_first_word() {
        let bytes1 = parse_text("morango");
        assert_handle_werase_byte(bytes1, EMPTY, 7);

        let bytes2 = parse_text("maçã");
        assert_handle_werase_byte(bytes2, EMPTY, 6);

        let bytes3 = parse_text("maracujá");
        assert_handle_werase_byte(bytes3, EMPTY, 9);
    }

    #[test]
    fn do_nothing() {
        let bytes = EMPTY;
        assert_handle_werase_byte(bytes, EMPTY, 0);
    }
}
