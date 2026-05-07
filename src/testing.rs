use std::fmt;

#[repr(transparent)]
#[derive(PartialEq)]
pub struct ByteStr(Vec<u8>);

impl<Bytes: AsRef<[u8]>> From<Bytes> for ByteStr {
    fn from(value: Bytes) -> Self {
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

impl fmt::Debug for ByteStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"{}\"", self)
    }
}

#[repr(transparent)]
#[derive(PartialEq)]
pub struct ByteVec(Vec<u8>);

impl<Bytes: AsRef<[u8]>> From<Vec<Bytes>> for ByteVec {
    fn from(value: Vec<Bytes>) -> Self {
        let mut values: Vec<u8> = Vec::new();
        for bytes in value {
            values.extend(bytes.as_ref());
        }
        Self(values)
    }
}

impl fmt::Display for ByteVec {
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

impl fmt::Debug for ByteVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"{}\"", self)
    }
}

#[macro_export]
macro_rules! assert_eq_bytes {
    ($output:expr, $expected:expr) => {
        assert_eq!(ByteStr::from($output), ByteStr::from($expected))
    };
    ($output:expr, $expected:expr, $msg:expr) => {
        assert_eq!(ByteStr::from($output), ByteStr::from($expected), $msg)
    };
    ($output:expr, $expected:expr, $msg:literal, $($value:expr),*) => {
        assert_eq!(ByteStr::from($output), ByteStr::from($expected), $msg, $($value),*)
    };
}

#[macro_export]
macro_rules! assert_eq_bytes_vec {
    ($output:expr, $expected:expr) => {
        assert!(ByteVec::from($output) == ByteVec::from($expected))
    };
    ($output:expr, $expected:expr, $msg:expr) => {
        assert!(ByteVec::from($output) == ByteVec::from($expected), $msg)
    };
    ($output:expr, $expected:expr, $msg:literal, $($value:expr),*) => {
        assert!(ByteVec::from($output) == ByteVec::from($expected), $msg, $($value),*)
    };
}
