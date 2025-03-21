/// Represents tokens in the shell's lexical analysis.
///
/// This enum follows the POSIX shell specification.
pub enum Token {
    /// A generic word token.
    Word(String),
    /// An assignment word (`VAR=value`).
    AssignmentWord(String, String),
    /// A valid shell variable name.
    Name(String),
    /// A newline character.
    Newline,
    /// A file descriptor number.
    IONumber(String),
    /// An IO location (optionally supported).
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
}
