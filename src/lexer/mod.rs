mod token;
pub use token::Token;
use Token::*;

pub fn tokenize<T>(input: T) -> Box<[Token]>
where
    T: AsRef<[u8]>,
{
    let mut tokens: Vec<Token> = Vec::with_capacity(64);
    let mut current_word: Vec<u8> = Vec::with_capacity(16);

    let mut iter = input.as_ref().iter().peekable();

    while let Some(byte) = iter.next() {
        match *byte {
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
                current_word.push(*byte);
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

#[cfg(test)]
mod test_lexer {
    use super::*;

    #[test]
    fn tokenize_simple_command() {
        {
            let input = "pwd";
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                Word("pwd".into()),
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
        {
            let input = "pwd\n";
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                Word("pwd".into()),
                Newline,
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
        {
            let input = " \tpwd  \n";
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                Word("pwd".into()),
                Newline,
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
    }

    #[test]
    fn tokenize_control_flow() {
        {
            let input = "if :; then echo $HOME; fi\n";
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                If,
                Word(":".into()),
                Semi,
                Then,
                Word("echo".into()),
                Word("$HOME".into()),
                Semi,
                Fi,
                Newline,
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
        {
            let input = r#"if grep -q monitor x*.sh
            then
                ./set_monitors.sh
            fi
            "#;
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                If,
                Word("grep".into()),
                Word("-q".into()),
                Word("monitor".into()),
                Word("x*.sh".into()),
                Newline,
                Then,
                Newline,
                Word("./set_monitors.sh".into()),
                Newline,
                Fi,
                Newline,
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
        {
            let input = r#"if grep -q monitor x*.sh; then
                ./set_monitors.sh
            fi
            "#;
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                If,
                Word("grep".into()),
                Word("-q".into()),
                Word("monitor".into()),
                Word("x*.sh".into()),
                Semi,
                Then,
                Newline,
                Word("./set_monitors.sh".into()),
                Newline,
                Fi,
                Newline,
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
    }

    #[test]
    fn tokenize_loops() {
        {
            let input = "for script in *.sh; do . \"$script\" done \n";
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                For,
                Word("script".into()),
                In,
                Word("*.sh".into()),
                Semi,
                Do,
                Word(".".into()),
                Word("\"$script\"".into()),
                Done,
                Newline,
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
        {
            let input = r#"for user in $(cat user_list.txt); do
                ./db add user "$user"
            done
            "#;
            let output = tokenize(input);
            let expected: Box<[Token]> = vec![
                For,
                Word("user".into()),
                In,
                Word("$(cat".into()),
                Word("user_list.txt)".into()),
                Semi,
                Do,
                Newline,
                Word("./db".into()),
                Word("add".into()),
                Word("user".into()),
                Word("\"$user\"".into()),
                Newline,
                Done,
                Newline,
            ].into_boxed_slice();

            assert_eq!(output, expected);
        }
    }
}
