#[cfg(not(test))]
use std::env::var as get_var;

#[derive(Debug, PartialEq)]
pub enum Field<T> {
    Plain(T),
    Parameter(T),
    Command(T),
    Arithmetic(T),
    Quoted(T),
    Position(T),
    Special(T),
}

fn is_valid_parameter(par: &str) -> bool {
    par.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_number(num: &str) -> bool {
    for c in num.chars() {
        if ! c.is_ascii_digit() {
            return false;
        }
    }
    true
}

fn in_braces(text: &str) -> bool {
    text.starts_with("${") && text.ends_with('}')
}

fn in_parenthesis(text: &str) -> bool {
    text.starts_with("$(") && text.ends_with(')')
}

fn in_single_parenthesis(text: &str) -> bool {
    if in_double_parenthesis(text) {
        return false;
    }
    text.starts_with("$(") && text.ends_with(')')
}

fn in_double_parenthesis(text: &str) -> bool {
    text.starts_with("$((") && text.ends_with("))")
}

fn in_single_quotes(text: &str) -> bool {
    text.starts_with("$'") && text.ends_with('\'') && text.len() > 2
}

fn in_backticks(text: &str) -> bool {
    text.starts_with('`') && text.ends_with('`') && text.len() > 1
}

fn is_unenclosed_parameter(text: &str) -> bool {
    text.starts_with('$') &&
        ! in_braces(text) &&
        ! in_parenthesis(text) &&
        ! in_single_quotes(text) &&
        is_valid_parameter(&text[1..])
}

fn is_special_parameter(text: &str) -> bool {
    let special_chars = ['@', '*', '#', '?', '-', '$', '!', '0'];
    text.len() > 1 && text.starts_with('$') &&
        special_chars.contains(&text.chars().nth(1).unwrap_or('_'))
}

fn get_parameter_name(text: String) -> (String, String) {
    let mut read_text = String::with_capacity(text.len());
    let mut longest_match: Option<usize> = None;

    for (i, character) in text.chars().enumerate() {
        if character.is_ascii_alphanumeric() || character == '_' {
            read_text.push(character);
            if get_var(&read_text).is_ok() {
                longest_match = Some(i + 1);
            }
        } else {
            break;
        }
    }

    if let Some(len) = longest_match {
        let name = String::from(&text[..len]);
        let remaining_chars = String::from(&text[len..]);
        return (name, remaining_chars);
    }

    (String::with_capacity(0), text)
}

impl Field<String> {
    pub fn new(text: String) -> Field<String> {
        let size: usize = text.len();

        if ! text.starts_with('$') {
            Field::Plain(text)
        } else if is_special_parameter(&text) {
            let name = text.chars().nth(1);
            if name.is_none() {
                return Field::Plain(String::new());
            }
            let mut par = String::new();
            par.push(name.unwrap());
            Field::Special(par)
        } else if is_unenclosed_parameter(&text) {
            let (par, _rest) = get_parameter_name(String::from(&text[1..]));
            Field::Parameter(par)
        } else if in_braces(&text) && !is_number(&text) {
            let par = String::from(&text[2..size - 1]);
            Field::Parameter(par)
        } else if in_braces(&text) && is_number(&text) {
            let num = String::from(text[2..size - 1].trim_start_matches('0'));
            Field::Position(num)
        } else if in_single_parenthesis(&text) {
            let command = String::from(&text[2..size - 1]);
            Field::Command(command)
        } else if in_backticks(&text) {
            let command = String::from(&text[1..size - 1]);
            Field::Command(command)
        } else if in_double_parenthesis(&text) {
            let math = String::from(&text[3..size - 2]);
            Field::Arithmetic(math)
        } else if in_single_quotes(&text) {
            let text = String::from(&text[1..size - 1]);
            Field::Quoted(text)
        } else {
            Field::Plain(text)
        }
    }

    pub fn substitute(self) -> String {
        match &self {
            Field::Plain(text) => text.to_string(),
            Field::Parameter(_) => self.substitute_parameter(),
            Field::Command(text) => format!("command: {}", text),
            Field::Arithmetic(text) => format!("arithmetic: {}", text),
            Field::Quoted(text) => format!("quoted: {}", text),
            Field::Position(text) => format!("positional parameter: {}", text),
            Field::Special(text) => format!("special parameter: {}", text),
        }
    }

    fn substitute_parameter(self) -> String {
        if let Field::Parameter(text) = self {
            match get_var(text.as_str()) {
                Ok(value) => value,
                Err(_) => String::with_capacity(0),
            }
        } else {
            String::with_capacity(0)
        }
    }
}

#[cfg(test)]
use std::env::VarError;

#[cfg(test)]
fn get_var(name: &str) -> Result<String, VarError> {
    match name {
        "HOME" => Ok("/home/gustav".to_string()),
        _ => Err(VarError::NotPresent)
    }
}

#[cfg(test)]
mod test {
    use super::Field;

    #[test]
    fn should_pass() {
        assert_eq!(
            Field::new(String::from("$HOME")),
            Field::Parameter(String::from("HOME")),
        );
    }
}
