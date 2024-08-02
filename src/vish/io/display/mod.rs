use std::io::{self, Write};

pub fn delete_previous_char() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(b"\x08 \x08")?;
    stdout.flush()
}

pub fn move_left() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(b"\x1b[D")?;
    stdout.flush()
}

pub fn move_right() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(b"\x1b[C")?;
    stdout.flush()
}

pub fn kill_line() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.write_all(b"\x1b[2K\r")?;
    stdout.flush()
}

pub fn reprint_line(data_list: &Vec<Vec<u8>>) -> io::Result<()> {
    let mut stdout = io::stdout();
    kill_line()?;
    for vec in data_list {
        stdout.write_all(vec.as_slice())?;
    }
    stdout.flush()
}
