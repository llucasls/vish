use std::fmt;
use std::io::{self, Read, Write, Seek, SeekFrom, Cursor};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

/// A buffer for handling dynamically allocated bytes,
/// with utilities for safely converting to UTF-8.
pub struct Buffer {
    cursor: Cursor<Vec<u8>>,
}

impl Buffer {
    /// Create new buffer
    pub fn new() -> Self {
        let cursor = Cursor::new(Vec::new());
        Self { cursor }
    }

    /// Create buffer from string slice
    pub fn from_utf8(text: &str) -> Self {
        let cursor: Cursor<Vec<u8>> = Cursor::new(text.into());
        Self { cursor }
    }

    /// Return the byte at the current position
    pub fn byte(&self) -> Option<u8> {
        let pos = self.position() as usize;
        if pos < self.len() {
            Some(self.get_ref()[pos])
        } else {
            None
        }
    }

    pub fn bytes(&self) -> std::str::Bytes<'_> {
        match self.as_str() {
            Ok(text) => text.bytes(),
            Err(_) => "".bytes(),
        }
    }

    /// Remove all values
    pub fn clear(&mut self) {
        self.get_mut().clear();
        self.set_position(0);
    }

    /// Return mutable reference to inner vector
    pub fn get_mut(&mut self) -> &mut Vec<u8> {
        self.cursor.get_mut()
    }

    /// Return reference to inner vector
    pub fn get_ref(&self) -> &Vec<u8> {
        self.cursor.get_ref()
    }

    /// Insert bytes into buffer at current position
    pub fn insert(&mut self, data: &[u8]) {
        let pos = self.cursor.position() as usize;
        let inner_vec = self.cursor.get_mut();

        inner_vec.splice(pos..pos, data.iter().cloned());
        self.cursor.set_position((pos + data.len()) as u64);
    }

    /// Consume buffer and return inner vector
    pub fn into_inner(self) -> Vec<u8> {
        self.cursor.into_inner()
    }

    /// Return true if vector is empty
    pub fn is_empty(&self) -> bool {
        self.cursor.get_ref().is_empty()
    }

    /// Return true if the inner vector is valid utf8 and false otherwise
    pub fn is_valid_utf8(&self) -> bool {
        std::str::from_utf8(self.get_ref()).is_ok()
    }

    /// Iterate over inner vector
    pub fn iter(&self) -> std::slice::Iter<'_, u8> {
        self.get_ref().iter()
    }

    /// Return the number of bytes stored in the buffer
    pub fn len(&self) -> usize {
        self.cursor.get_ref().len()
    }

    /// Return the current position
    pub fn position(&self) -> u64 {
        self.cursor.position()
    }

    /// Update the cursor position while respecting boundaries
    pub fn safe_seek(&mut self, pos: SeekFrom) -> u64 {
        let current_pos = self.cursor.position();
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                let end_pos = self.len() as i64;
                (end_pos + offset).max(0) as u64
            },
            SeekFrom::Current(offset) => {
                let new_pos = (current_pos as i64 + offset).max(0) as u64;
                new_pos.min(self.len() as u64)
            },
        };

        let bounded_pos = new_pos.min(self.len() as u64);
        self.cursor.set_position(bounded_pos);
        bounded_pos
    }

    /// Update the cursor position
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(pos)
    }

    /// Set the position to a given number
    pub fn set_position(&mut self, pos: u64) {
        self.cursor.set_position(pos);
    }

    /// Return byte vector content as string
    pub fn as_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.get_ref().to_vec())
    }

    /// Return byte vector content as string slice
    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(self.get_ref())
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(data: Vec<u8>) -> Self {
        let cursor = Cursor::new(data);
        Self { cursor }
    }
}

impl From<&[u8]> for Buffer {
    fn from(data: &[u8]) -> Self {
        let cursor = Cursor::new(data.to_vec());
        Self { cursor }
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer")
            .field("inner", &self.cursor.get_ref())
            .field("pos", &self.cursor.position())
            .finish()
    }
}

impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.cursor.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.cursor.flush()
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}
