#[macro_export]
macro_rules! kill_line {
    ($stdout:expr) => {{
        $stdout.write_all(b"\x1b[2K\r")?;
    }};
}

#[macro_export]
macro_rules! move_cursor {
    ($offset:expr, $stdout:expr) => {{
        match $offset.cmp(&0) {
            std::cmp::Ordering::Less => {
                write!($stdout, "\x1b[{}D", -($offset as isize))?;
            },
            std::cmp::Ordering::Greater => {
                write!($stdout, "\x1b[{}C", $offset)?;
            },
            _ => {},
        }
    }};
}

#[macro_export]
macro_rules! reprint_line {
    ($stdout:expr, $data_list:expr) => {{
        let ps1 = match crate::ENV.read() {
            Ok(shell) => shell.get_var("PS1").unwrap_or(String::new()),
            Err(_) => String::new(),
        };
        $stdout.write_all(b"\x1b[s")?;
        kill_line!($stdout);
        $stdout.write_all(ps1.as_bytes())?;
        for utf8_char in $data_list.iter() {
            $stdout.write_all(utf8_char.as_slice())?;
        }
        $stdout.write_all(b"\x1b[u")?;
    }};
}
