mod token;
use token::Token;

pub fn tokenize<T>(input: T) -> Box<[Token]>
where
    T: AsRef<[u8]>,
{
    let mut tokens: Vec<Token> = Vec::with_capacity(64);
    let mut words: Vec<u8> = Vec::with_capacity(64);
    let mut current_word: Vec<u8> = Vec::with_capacity(16);

    for byte in input.as_ref() {
        current_word.push(*byte);

        match &current_word {
            w if w == b"if" => {

            },
            _ => {}
        }
    }

    tokens.into_boxed_slice()
}
