use std::io::{self, Read, Write, Seek, SeekFrom, Cursor};
use std::fmt;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

pub type Byte = u8;
pub type ByteArray = [u8];

pub struct Buffer {
    cursor: Cursor<Vec<Byte>>,
}

impl Buffer {
    // Create new buffer
    pub fn new() -> Self {
        Buffer {
            cursor: Cursor::new(Vec::new()),
        }
    }

    // Create buffer from byte vector
    pub fn from(data: Vec<u8>) -> Self {
        Buffer {
            cursor: Cursor::new(data),
        }
    }

    // Create buffer from string slice
    pub fn from_utf8(text: &str) -> Self {
        Buffer {
            cursor: Cursor::new(text.into()),
        }
    }

    // Return the number of bytes stored in the buffer
    pub fn len(&self) -> usize {
        self.cursor.get_ref().len()
    }

    // Return the byte at the current position
    pub fn byte(&self) -> Option<Byte> {
        let pos = self.position() as usize;
        if pos < self.len() {
            Some(self.get_ref()[pos])
        } else {
            None
        }
    }

    // Return true if the inner vector is valid utf8 and false otherwise
    pub fn is_valid_utf8(&self) -> bool {
        std::str::from_utf8(self.get_ref()).is_ok()
    }

    // Return the current position
    pub fn position(&self) -> u64 {
        self.cursor.position()
    }

    // Set the position to a given number
    pub fn set_position(&mut self, pos: u64) {
        self.cursor.set_position(pos);
    }

    // Update the cursor position
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(pos)
    }

    // Write bytes from byte array into buffer
    pub fn write(&mut self, buf: &ByteArray) -> io::Result<usize> {
        self.cursor.write(buf)
    }

    // Read bytes from buffer and place them into byte array
    pub fn read(&mut self, buf: &mut ByteArray) -> io::Result<usize> {
        self.cursor.read(buf)
    }

    // Insert bytes into buffer at current position
    pub fn insert(&mut self, data: &ByteArray) {
        let pos = self.cursor.position() as usize;
        let inner_vec = self.cursor.get_mut();

        inner_vec.splice(pos..pos, data.iter().cloned());
        self.cursor.set_position((pos + data.len()) as u64);
    }

    // Update the cursor position while respecting boundaries
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

    // Return reference to inner vector
    pub fn get_ref(&self) -> &Vec<Byte> {
        self.cursor.get_ref()
    }

    // Return mutable reference to inner vector
    pub fn get_mut(&mut self) -> &mut Vec<Byte> {
        self.cursor.get_mut()
    }

    // Return byte vector content as string
    pub fn as_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.get_ref().to_vec())
    }

    // Return byte vector content as string slice
    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(self.get_ref())
    }

    // Iterate over inner vector
    pub fn iter(&self) -> std::slice::Iter<'_, Byte> {
        self.get_ref().iter()
    }

    // Consume buffer and return inner vector
    pub fn into_inner(self) -> Vec<Byte> {
        self.cursor.into_inner()
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
