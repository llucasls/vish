mod token;
use token::Token;
use Token::*;

pub fn tokenize<T>(input: T) -> Box<[Token]>
where
    T: IntoIterator<Item = u8>,
{
    let mut tokens: Vec<Token> = Vec::with_capacity(64);
    let mut current_word: Vec<u8> = Vec::with_capacity(16);

    let mut iter = input.into_iter().peekable();

    while let Some(byte) = iter.next() {
        match byte {
            b if b.is_ascii_whitespace() && b != b'\n' => {
                push_word(&mut tokens, &mut current_word);
            },

            b'\n' => {
                push_word(&mut tokens, &mut current_word);
                tokens.push(Newline);
            },

            b';' => {
                push_word(&mut tokens, &mut current_word);

                match iter.peek() {
                    Some(b';') => {
                        iter.next();
                        tokens.push(DSemi);
                    },

                    Some(b'&') => {
                        iter.next();
                        tokens.push(SemiAnd);
                    },

                    _ => {
                        tokens.push(Semi);
                    },
                }
            },

            b'&' => {
                push_word(&mut tokens, &mut current_word);

                match iter.peek() {
                    Some(b'&') => {
                        iter.next();
                        tokens.push(AndIf);
                    },

                    _ => {
                        tokens.push(Ampersand);
                    },
                }
            },

            b'|' => {
                push_word(&mut tokens, &mut current_word);

                match iter.peek() {
                    Some(b'|') => {
                        iter.next();
                        tokens.push(OrIf);
                    },

                    _ => {
                        tokens.push(Pipe);
                    },
                }
            },

            b'<' => {
                push_word(&mut tokens, &mut current_word);

                match iter.peek() {
                    Some(b'<') => {
                        iter.next();

                        match iter.peek() {
                            Some(b'-') => {
                                iter.next();
                                tokens.push(DLessDash);
                            },

                            _ => {
                                tokens.push(DLess);
                            },
                        }
                    },

                    Some(b'&') => {
                        iter.next();
                        tokens.push(LessAnd);
                    },

                    Some(b'>') => {
                        iter.next();
                        tokens.push(LessGreat);
                    },

                    _ => {
                        tokens.push(Less);
                    },
                }
            },

            b'>' => {
                push_word(&mut tokens, &mut current_word);

                match iter.peek() {
                    Some(b'>') => {
                        iter.next();
                        tokens.push(DGreat);
                    },

                    Some(b'&') => {
                        iter.next();
                        tokens.push(GreatAnd);
                    },

                    Some(b'|') => {
                        iter.next();
                        tokens.push(Clobber);
                    },

                    _ => {
                        tokens.push(Great);
                    },
                }
            },

            b'{' => {
                push_word(&mut tokens, &mut current_word);
                tokens.push(Lbrace);
            },

            b'}' => {
                push_word(&mut tokens, &mut current_word);
                tokens.push(Rbrace);
            },

            b'!' => {
                push_word(&mut tokens, &mut current_word);
                tokens.push(Bang);
            },

            _ => {
                current_word.push(byte);
            },
        }
    }

    push_word(&mut tokens, &mut current_word);

    classify_keywords(&mut tokens);

    tokens.into_boxed_slice()
}

fn push_word(tokens: &mut Vec<Token>, current_word: &mut Vec<u8>) {
    if current_word.is_empty() {
        return;
    }

    let bytes = std::mem::take(current_word);

    let text = String::from_utf8(bytes)
        .expect("lexer produced invalid UTF-8");

    tokens.push(Word(text));

    current_word.reserve(16);
}

fn classify_keywords(tokens: &mut [Token]) {
    for token in tokens {
        let Token::Word(word) = token else {
            continue;
        };

        *token = match word.as_str() {
            "if" => If,
            "then" => Then,
            "else" => Else,
            "elif" => Elif,
            "fi" => Fi,
            "do" => Do,
            "done" => Done,
            "case" => Case,
            "esac" => Esac,
            "while" => While,
            "until" => Until,
            "for" => For,
            "in" => In,

            _ => continue,
        };
    }
}
