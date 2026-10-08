/// Represents tokens in the shell's lexical analysis.
///
/// This enum follows the POSIX shell specification.
#[derive(Debug, PartialEq)]
pub enum Token {
    /// A generic word token.
    Word(String),
    /// A token marking the end of an input line.
    Newline,
    /// A file descriptor number.
    IONumber(String),
    /// An IO location (a variable with a file descriptor number).
    IOLocation(String),
    /// `&&`
    AndIf,
    /// `||`
    OrIf,
    /// `;;`
    DSemi,
    /// `;&`
    SemiAnd,
    /// `<<`
    DLess,
    /// `>>`
    DGreat,
    /// `<&`
    LessAnd,
    /// `>&`
    GreatAnd,
    /// `<>`
    LessGreat,
    /// `<<-`
    DLessDash,
    /// `>|`
    Clobber,
    /// `if`
    If,
    /// `then`
    Then,
    /// `else`
    Else,
    /// `elif`
    Elif,
    /// `fi`
    Fi,
    /// `do`
    Do,
    /// `done`
    Done,
    /// `case`
    Case,
    /// `esac`
    Esac,
    /// `while`
    While,
    /// `until`
    Until,
    /// `for`
    For,
    /// `{`
    Lbrace,
    /// `}`
    Rbrace,
    /// `!`
    Bang,
    /// `in`
    In,
    /// `;`
    Semi,
    /// `&`
    Ampersand,
    /// `|`
    Pipe,
    /// `<`
    Less,
    /// `>`
    Great,
    /// A meta-character token.
    Meta(char),
}
