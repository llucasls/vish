use std::fmt;

#[derive(Debug, PartialEq)]
pub struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError")
    }
}

impl std::error::Error for ParseError {}
