mod ast;

use ast::{Ast, Cmd};
use crate::lexer::Token;
use crate::errors::ParseError;
use Token::*;

pub fn parse(token_list: &[Token]) -> Result<Ast, ParseError> {
    let mut syntax_tree: Option<Ast> = None;

    for token in token_list {
        match token {
            Word(word) => {
                match &mut syntax_tree {
                    Some(Ast::Command(cmd)) => {
                        let arg = word.to_string();
                        cmd.0.push(arg);
                    },
                    Some(Ast::Sequence(list)) => {},
                    Some(Ast::Pipeline(list)) => {},
                    Some(Ast::And(lhs, rhs)) => {

                    },
                    Some(Ast::Or(lhs, rhs)) => {},
                    None => {
                        let cmd = Cmd(vec![word.into()]);
                        syntax_tree = Some(Ast::Command(cmd));
                    },
                }
            },
            _ => {}
        }
    }

    syntax_tree.ok_or(ParseError)
}

#[cfg(test)]
mod test_parser {
    use super::*;

    #[test]
    fn parse_tokens() {
        {
            let input: Box<[Token]> = vec![
                Word("ls".into()),
                Word("~/bin".into()),
                Newline,
            ].into_boxed_slice();
            let output = parse(&input).unwrap();

            let expected_cmd = vec![
                "ls".into(),
                "~/bin".into(),
            ];
            let expected: Ast = Ast::Command(Cmd(expected_cmd));

            assert_eq!(output, expected);
        }
    }

    #[test]
    fn parse_loop() {
        {
            let input: Box<[Token]> = vec![
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
            let output = parse(&input).unwrap();

            let expected_cmd = vec![
                "".into(),
            ];
            let expected: Ast = Ast::Command(Cmd(expected_cmd));

            assert_eq!(output, expected);
        }
    }
}
