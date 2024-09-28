#[cfg(not(test))]
use crate::vish::passwd::get_home;

#[cfg(not(test))]
use std::env::var as get_var;

use crate::vish::command::ArgV;

mod expand_parameter;
pub use expand_parameter::expand_parameter;

mod substitute_command;
pub use substitute_command::substitute_command;

mod field;
pub use field::Field;

pub fn replace_tilde(user_input: String) -> String {
    let tilde_index = user_input.find('~');
    let bar_index = user_input.find('/');
    if tilde_index.is_none() {
        user_input
    } else if user_input == "~" || user_input.find("~/") == Some(0) {
        match get_var("HOME") {
            Ok(home) => {
                let mut output = user_input.clone();
                output.replace_range(0..1, home.as_str());
                output
            },
            Err(_) => user_input,
        }
    } else if tilde_index == Some(0) && bar_index != Some(1) {
        let user = match bar_index {
            Some(end) => user_input[1..end].to_string(),
            None => user_input[1..].to_string(),
        };
        match get_home(user) {
            Some(home) => {
                let mut output = user_input.clone();
                match bar_index {
                    Some(i) => {
                        output.replace_range(0..i, home.as_str());
                    },
                    None => {
                        output.replace_range(0.., home.as_str());
                    },
                }
                output
            },
            None => user_input,
        }
    } else {
        user_input
    }
}

pub fn parse_argv(text: &str) -> (ArgV, Option<char>) {
    let mut argv: ArgV = Vec::new();
    let mut in_quotes = false;
    let mut current_arg = String::new();
    let mut quote_char = '\0';

    for c in text.chars() {
        match c {
            '\'' | '"' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
            },
            '\'' | '"' if in_quotes && c == quote_char => {
                in_quotes = false;
                argv.push(replace_tilde(current_arg.clone()));
                current_arg.clear();
            },
            ' ' | '\t' if !in_quotes && !current_arg.is_empty() => {
                argv.push(replace_tilde(current_arg.clone()));
                current_arg.clear();
            },
            ' ' | '\t' if !in_quotes && current_arg.is_empty() => {},
            _ => {
                current_arg.push(c);
            }
        }
    }

    if !current_arg.is_empty() {
        argv.push(replace_tilde(current_arg));
    }

    if in_quotes {
        (argv, Some(quote_char))
    } else {
        (argv, None)
    }
}

pub fn split_argv(text: &str) -> (Vec<String>, Option<char>) {
    let mut argv: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut current_arg = String::new();
    let mut quote_char = '\0';

    for c in text.chars() {
        match c {
            '\'' | '"' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
            },
            '\'' | '"' if in_quotes && c == quote_char => {
                in_quotes = false;
                argv.push(Field::Plain(current_arg.clone()).substitute());
                current_arg.clear();
            },
            ' ' | '\t' if !in_quotes && !current_arg.is_empty() => {
                argv.push(Field::Plain(current_arg.clone()).substitute());
                current_arg.clear();
            },
            ' ' | '\t' if !in_quotes && current_arg.is_empty() => {},
            _ => {
                current_arg.push(c);
            }
        }
    }

    if !current_arg.is_empty() {
        argv.push(Field::Plain(current_arg).substitute());
    }

    if in_quotes {
        (argv, Some(quote_char))
    } else {
        (argv, None)
    }
}

#[cfg(test)]
mod split_argv {
    use super::{split_argv, Field};

    #[test]
    fn return_single_plain_field() {
        let input = "antedeguemon";
        let (output, _) = split_argv(input);
        assert_eq!(output, vec![Field::Plain("antedeguemon".to_string()).substitute()]);
    }

    #[test]
    fn return_parameter_field() {
        let input = "echo ${USER}";
        let (output, _) = split_argv(input);
        assert_eq!(output, vec![
            Field::Plain("echo".to_string()).substitute(),
            Field::Parameter("USER".to_string()).substitute()
        ]);
    }
}

#[cfg(test)]
fn get_home(user: String) -> Option<String> {
    let mut home_dirs = std::collections::HashMap::new();
    home_dirs.insert(String::from("root"), String::from("/root"));
    home_dirs.insert(String::from("john"), String::from("/home/john"));
    home_dirs.get(&user).cloned()
}

#[cfg(test)]
fn get_var(_: &str) -> Result<String, std::env::VarError> {
    Ok(String::from("/home/kevin"))
}

#[cfg(test)]
mod replace_tilde {
    use super::replace_tilde;

    #[test]
    fn replace_single_tilde_with_home() {
        let input = String::from("~");
        let output = replace_tilde(input.clone());
        let expected = "/home/kevin";
        assert_eq!(output, expected, "\n input: `{:?}`", input);
    }

    #[test]
    fn replace_single_tilde_with_home_subdir() {
        let input = String::from("~/.config");
        let output = replace_tilde(input.clone());
        let expected = "/home/kevin/.config";
        assert_eq!(output, expected, "\n input: `{:?}`", input);
    }

    #[test]
    fn replace_tilde_for_user() {
        let input = String::from("~john");
        let output = replace_tilde(input.clone());
        let expected = "/home/john";
        assert_eq!(output, expected, "\n input: `{:?}`", input);
    }

    #[test]
    fn replace_tilde_for_user_subdir() {
        let input = String::from("~john/.config");
        let output = replace_tilde(input.clone());
        let expected = "/home/john/.config";
        assert_eq!(output, expected, "\n input: `{:?}`", input);
    }

    #[test]
    fn replace_tilde_for_root() {
        let input = String::from("~root");
        let output = replace_tilde(input.clone());
        let expected = "/root";
        assert_eq!(output, expected, "\n input: `{:?}`", input);
    }

    #[test]
    fn replace_tilde_for_root_subdir() {
        let input = String::from("~root/.config");
        let output = replace_tilde(input.clone());
        let expected = "/root/.config";
        assert_eq!(output, expected, "\n input: `{:?}`", input);
    }

    #[test]
    fn do_not_replace() {
        let input = String::from("/usr/local");
        let output = replace_tilde(input.clone());
        let expected = "/usr/local";
        assert_eq!(output, expected, "\n input: `{:?}`", input);
    }
}

#[cfg(test)]
mod parse_argv {
    use super::parse_argv;

    #[test]
    fn parse_argv_basic() {
        let input = "echo hello world";
        let expected = vec!["echo", "hello", "world"];
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }

    #[test]
    fn parse_argv_with_tilde() {
        let input = "cd ~";
        let expected = vec!["cd", "/home/kevin"];
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }

    #[test]
    fn parse_argv_with_tilde_and_path() {
        let input = "cd ~/projects";
        let expected = vec!["cd", "/home/kevin/projects"];
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }

    #[test]
    fn parse_argv_with_quoted_string() {
        let input = "echo \"hello world\"";
        let expected = vec!["echo", "hello world"];
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }

    #[test]
    fn parse_argv_with_multiple_quoted_strings() {
        let input = "echo \"hello world\" 'and universe'";
        let expected = vec!["echo", "hello world", "and universe"];
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }

    #[test]
    fn parse_argv_with_quotes_and_tilde() {
        let input = "cp \"~john/file with spaces.txt\" ~john/backup/";
        let expected = vec![
            "cp",
            "/home/john/file with spaces.txt",
            "/home/john/backup/",
        ];
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }

    #[test]
    fn parse_argv_with_single_quotes() {
        let input = "touch 'test output.log'";
        let expected = vec!["touch", "test output.log"];
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }

    #[test]
    fn parse_argv_with_empty_string() {
        let input = "";
        let expected: Vec<String> = Vec::new();
        let (result, quote_char) = parse_argv(input);
        assert_eq!(result, expected);
        assert_eq!(quote_char, None);
    }
}
