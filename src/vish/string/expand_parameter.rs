#[cfg(not(test))]
use std::env::var as get_var;

pub fn expand_parameter(input: String) -> String {
    let dollar_index = input.find('$');
    if dollar_index.is_none() {
        return input;
    }

    let mut i = dollar_index.unwrap();
    let mut result = input.clone();
    let is_valid = |c: char| c.is_alphanumeric() || c == '_';

    macro_rules! found_braces {
        ($i:expr) => {
            $i + 1 < result.len() && &result[$i + 1..=$i + 1] == "{"
        }
    }

    while i < result.len() {
        if &result[i..=i] != "$" {
            i += 1;
            continue;
        } else if ! found_braces!(i) {
            let mut end = i + 1;
            while end < result.len() && result[end..=end].chars().all(is_valid) {
                end += 1;
            }
            if end <= i + 1 {
                i += 1;
                continue;
            }
            let name = &result[i + 1..end];
            let value = get_var(name).unwrap_or_else(|_| String::new());
            result.replace_range(i..end, &value);
            i += value.len();
        } else if let Some(end) = result[i + 2..].find('}') {
            let start = i + 2;
            let end = i + 2 + end;
            let name = &result[start..end];
            let value = get_var(name).unwrap_or_else(|_| String::new());
            result.replace_range(i..end + 1, &value);
            i += value.len();
        }
    }

    result
}

#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
fn get_var(name: &str) -> Result<String, std::env::VarError> {
    let mut table = HashMap::new();
    let path = "/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/usr/local/sbin";
    table.insert("HOME", String::from("/home/bob"));
    table.insert("PATH", String::from(path));
    table.insert("PAT", String::from("Patricia"));
    table.insert("PATH1", String::from("/bin"));
    Ok(table.get(name).unwrap().clone())
}

#[cfg(test)]
mod test {
    use super::expand_parameter;

    #[test]
    fn expand_home_variable() {
        assert_eq!(
            expand_parameter(String::from("${HOME}/.config")),
            "/home/bob/.config"
        );
    }

    #[test]
    fn expand_home_variable_without_braces() {
        assert_eq!(
            expand_parameter(String::from("$HOME/.config")),
            "/home/bob/.config"
        );
    }

    #[test]
    fn expand_home_variable_twice() {
        assert_eq!(
            expand_parameter(String::from("${HOME} ${HOME}")),
            "/home/bob /home/bob"
        );
    }

    #[test]
    fn expand_wrong_variable() {
        assert_eq!(
            expand_parameter(String::from("$PATH1")),
            "/bin"
        );
    }

    #[test]
    fn do_nothing() {
        assert_eq!(
            expand_parameter(String::from("/bin")),
            "/bin"
        );
    }
}
