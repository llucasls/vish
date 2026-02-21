use std::collections::HashMap;
use std::fmt;
use std::ops::Add;

macro_rules! parse_count {
    ($self:ident, $key:expr, $pairs:ident) => {{
        let old_value: CliValue = $self
            .options
            .get(&$key)
            .unwrap_or(&CliValue::Int(0))
            .clone();
        let new_value: CliValue = $self.parse_count_option(&$key, &$pairs)?;
        $pairs.insert($key, old_value + new_value);
    }};
}

macro_rules! parse_int {
    ($self:ident, $key:expr, $i:expr, $n:expr, $iter:ident, $next_arg:ident, $pairs:ident) => {{
        if $next_arg.is_none() && $i == $n - 1 {
            Err(CliError::NullInt($key))
        } else if $i < $n - 1 {
            let value: Option<String> = Some($iter
                .clone()
                .map(|(_, value)| value)
                .collect());
            let int_value = $self.parse_int_option(&$key, value)?;
            $pairs.insert($key, int_value);
            Ok((false, $pairs))
        } else {
            let next_value = $next_arg.map(|v| v.clone());
            let int_value = $self.parse_int_option(&$key, next_value)?;
            $pairs.insert($key.clone(), int_value);
            Ok((true, $pairs))
        }
    }};
}

macro_rules! parse_opt_int {
    ($self:ident, $key:expr, $i:expr, $n:expr, $iter:ident, $default:expr, $pairs:ident) => {{
        if $i < $n - 1 {
            let value: Option<String> = Some($iter
                .clone()
                .map(|(_, value)| value)
                .collect());
            let int_value = $self.parse_opt_int_option(value, $default)?;
            $pairs.insert($key, int_value);
            break;
        } else {
            let int_value = $self.parse_opt_int_option(None, $default)?;
            $pairs.insert($key, int_value);
        }
    }};
}

macro_rules! parse_text {
    ($self:ident, $key:expr, $i:expr, $n:expr, $iter:ident, $next_arg:ident, $pairs:ident) => {{
        let next_value = $next_arg.map(|v| v.clone());
        if next_value.is_none() && $i == $n - 1 {
            Err(CliError::NullArg($key))
        } else if $i < $n - 1 {
            let value: Option<String> = Some($iter
                .clone()
                .map(|(_, value)| value)
                .collect());
            let text_value = $self
                .parse_text_option(&$key, value)?;
            $pairs.insert($key, text_value);
            Ok((false, $pairs))
        } else {
            let text_value = $self
                .parse_text_option(&$key, next_value)?;
            $pairs.insert($key, text_value);
            Ok((true, $pairs))
        }
    }};
}

macro_rules! parse_opt_string {
    ($self:ident, $key:expr, $iter:ident, $default:expr, $pairs:ident) => {{
        let value: Option<String> = Some($iter
            .clone()
            .map(|(_, value)| value)
            .collect());
        let text_value = $self
            .parse_opt_string_option(value, $default.clone())?;
        $pairs.insert($key, text_value);
        break;
    }};
}

macro_rules! parse_list {
    ($self:ident, $key:expr, $next_arg:ident, $sep:expr, $pairs:ident) => {{
        if $next_arg.is_none() {
            Err(CliError::NullArg($key))
        } else {
            let next_value = $next_arg.map(|v| v.clone());
            let list_value = $self
                .parse_list_option(&$key, next_value, $sep)?;
            $pairs.insert($key, list_value);
            Ok((true, $pairs))
        }
    }};
}

macro_rules! insert_value {
    ($options:expr, $name:expr, $value:expr) => {{
        match $value {
            CliValue::List(new) => {
                match $options.get_mut(&$name) {
                    Some(CliValue::List(old)) => {
                        old.extend_from_slice(&new);
                    },
                    None => {
                        $options.insert($name, CliValue::List(new));
                    },
                    _ => {
                        unreachable!("non-list value found for list option");
                    },
                }
            },
            parsed_value => {
                $options.insert($name, parsed_value);
            },
        }
    }};
}

pub trait CliReadConfigs<T> {
    fn read_configs(&mut self, entry_list: T) -> &mut Self;
}

fn get_opt_value(arg: &str) -> CliArg {
    if arg == "--" {
        CliArg::Operand
    } else if let Some(stripped) = arg.strip_prefix("--") {
        let mut parts = stripped.splitn(2, '=');
        let name = parts.next().unwrap_or_default().to_string();
        let value = parts.next().map(|v| v.to_string());
        if name.is_empty() {
            CliArg::Operand
        } else {
            CliArg::Long { name, value }
        }
    } else if let Some(flags) = arg.strip_prefix('-') {
        CliArg::Short { flags: flags.to_string() }
    } else {
        CliArg::Operand
    }
}

/// Represents optional argument variants.
#[derive(Clone, Debug)]
pub enum CliOption {
    Flag,
    Count,
    Int,
    OptInt { default: i32 },
    Text,
    OptString { default: String },
    List { sep: char },
    Alias { target: String },
}

#[derive(PartialEq, Clone)]
pub enum CliValue {
    Flag,
    Int(i32),
    Text(String),
    List(Vec<String>),
}

impl Add for CliValue {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a + b),
            (Self::Text(a), Self::Text(b)) => Self::Text(a + &b),
            (Self::List(mut a), Self::List(b)) => {
                a.extend(b);
                Self::List(a)
            }
            (a, b) => {
                panic!("warning: cannot add {:?} and {:?}", a, b);
            }
        }
    }
}

impl Add<i32> for CliValue {
    type Output = Self;
    fn add(self, rhs: i32) -> Self {
        match self {
            Self::Int(a) => Self::Int(a + rhs),
            lhs => panic!("cannot add {:?} and {:?}", lhs, rhs),
        }
    }
}

impl fmt::Debug for CliValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliValue::Flag => {
                f.debug_struct("CliValue::Flag").finish()
            },
            CliValue::Int(num) => {
                f.debug_tuple("CliValue::Int").field(num).finish()
            },
            CliValue::Text(text) => {
                f.debug_tuple("CliValue::Text").field(text).finish()
            },
            CliValue::List(list) => {
                f.debug_tuple("CliValue::List").field(list).finish()
            },
        }
    }
}

/// An argument passed to a command.
#[derive(Debug, Clone, PartialEq)]
enum CliArg {
    /// A long option like `--option` or `--option=value`
    Long { name: String, value: Option<String> },

    /// A short option like `-o` or `-abc`
    Short { flags: String },

    /// A positional argument
    Operand,
}

pub struct CliParser {
    pub options: HashMap<String, CliValue>,
    pub operators: Vec<String>,
    pub argv0: String,
    configs: HashMap<String, CliOption>,
}

impl CliParser {
    pub fn new() -> Self {
        let options: HashMap<String, CliValue> = HashMap::new();
        let operators: Vec<String> = Vec::new();
        let argv0 = String::new();
        let configs = HashMap::new();

        Self {
            options,
            operators,
            argv0,
            configs,
        }
    }

    pub fn parse(&mut self, arg_list: Vec<String>) -> Result<(), CliError> {
        if arg_list.is_empty() {
            return Err(CliError::EmptyList);
        } else if self.configs.is_empty() {
            return Err(CliError::ConfigsNotFound);
        }

        self.options.clear();
        self.operators.clear();

        let mut stop_parsing = false;
        let mut iter = arg_list.into_iter().peekable();
        self.argv0 = iter.next().unwrap();

        while let Some(arg) = iter.next() {
            if arg == "--" && !stop_parsing {
                stop_parsing = true;
                continue;
            }

            if stop_parsing {
                self.operators.push(arg);
                continue;
            }

            match get_opt_value(&arg) {
                CliArg::Short { flags } => {
                    let next_value = iter.peek();
                    let (skip, mut pairs) = self
                        .parse_short_option(flags.clone(), next_value)?;

                    for (name, value) in pairs.drain() {
                        insert_value!(self.options, name, value);
                    }

                    if skip {
                        drop(iter.next());
                    }
                },
                CliArg::Long { name, value } => {
                    let value = self.parse_long_option(&name, value)?;
                    insert_value!(self.options, name, value);
                },
                CliArg::Operand => {
                    self.operators.push(arg);
                },
            }
        }

        Ok(())
    }

    fn parse_long_option(
        &self,
        name: &str,
        value: Option<String>,
    ) -> Result<CliValue, CliError> {
        if let Some(entry) = self.configs.get(name) {
            match entry {
                CliOption::Flag => {
                    Ok(CliValue::Flag)
                },
                CliOption::Count => {
                    self.parse_int_option(name, value)
                },
                CliOption::Int => {
                    self.parse_int_option(name, value)
                },
                CliOption::OptInt { default } => {
                    self.parse_opt_int_option(value, *default)
                },
                CliOption::Text => {
                    self.parse_text_option(name, value)
                },
                CliOption::OptString { default } => {
                    self.parse_opt_string_option(value, default.clone())
                },
                CliOption::List { sep } => {
                    self.parse_list_option(name, value, *sep)
                },
                CliOption::Alias { target } => {
                    match self.configs.get(target) {
                        Some(CliOption::Flag) => {
                            Ok(CliValue::Flag)
                        },
                        Some(CliOption::Count) => {
                            self.parse_int_option(target, value)
                        },
                        Some(CliOption::Int) => {
                            self.parse_int_option(target, value)
                        },
                        Some(CliOption::OptInt { default }) => {
                            self.parse_opt_int_option(value, *default)
                        },
                        Some(CliOption::Text) => {
                            self.parse_text_option(target, value)
                        },
                        Some(CliOption::OptString { default }) => {
                            self.parse_opt_string_option(
                                value, default.clone())
                        },
                        Some(CliOption::List { sep }) => {
                            self.parse_list_option(target, value, *sep)
                        },
                        Some(CliOption::Alias { .. }) => {
                            Err(CliError::Alias2Alias)
                        },
                        None => {
                            Err(CliError::TargetNotFound(target.into()))
                        },
                    }
                },
            }
        } else {
            Err(CliError::OptionNotSupported(name.into()))
        }
    }

    fn parse_short_option(
        &self,
        flags: String,
        next_arg: Option<&String>,
    ) -> Result<(bool, HashMap<String, CliValue>), CliError> {
        let n: usize = flags.len();
        let mut pairs: HashMap<String, CliValue> = HashMap::new();
        let mut iter = flags.chars().enumerate();

        while let Some((i, flag)) = iter.next() {
            let name: String = String::from(flag);
            let Some(entry) = self.configs.get(&name) else {
                return Err(CliError::OptionNotSupported(name));
            };

            match entry {
                CliOption::Flag => {
                    pairs.insert(name, CliValue::Flag);
                },
                CliOption::Count => {
                    parse_count!(self, name, pairs);
                },
                CliOption::Int => {
                    return parse_int!(
                        self, name, i, n, iter, next_arg, pairs);
                },
                CliOption::OptInt { default } => {
                    parse_opt_int!(self, name, i, n, iter, *default, pairs);
                },
                CliOption::Text => {
                    return parse_text!(
                        self, name, i, n, iter, next_arg, pairs);
                },
                CliOption::OptString { default } => {
                    parse_opt_string!(self, name, iter, default, pairs);
                },
                CliOption::List { sep } => {
                    return parse_list!(self, name, next_arg, *sep, pairs);
                },
                CliOption::Alias { target } => {
                    let Some(entry) = self.configs.get(target) else {
                        return Err(
                            CliError::TargetNotFound(target.into())
                        );
                    };
                    let name: String = target.to_string();
                    match entry {
                        CliOption::Flag => {
                            pairs.insert(name, CliValue::Flag);
                        },
                        CliOption::Count => {
                            parse_count!(self, name, pairs);
                        },
                        CliOption::Int => {
                            return parse_int!(
                                self, name, i, n, iter, next_arg, pairs);
                        },
                        CliOption::OptInt { default } => {
                            parse_opt_int!(
                                self, name, i, n, iter, *default, pairs);
                        },
                        CliOption::Text => {
                            return parse_text!(
                                self, name, i, n, iter, next_arg, pairs);
                        },
                        CliOption::OptString { default } => {
                            parse_opt_string!(
                                self, name, iter, default, pairs);
                        },
                        CliOption::List { sep } => {
                            return parse_list!(
                                self, name, next_arg, *sep, pairs);
                        },
                        CliOption::Alias { .. } => {
                            return Err(CliError::Alias2Alias);
                        },
                    }
                },
            }
        }

        return Ok((false, pairs))
    }

    fn parse_count_option(
        &self,
        name: &str,
        pairs: &HashMap<String, CliValue>,
    ) -> Result<CliValue, CliError> {
        match pairs.get(name) {
            Some(CliValue::Int(num)) => Ok(CliValue::Int(num + 1)),
            None => Ok(CliValue::Int(1)),
            _ => Err(CliError::Generic("wrong type".into())),
        }
    }

    fn parse_int_option(
        &self,
        name: &str,
        value: Option<String>
    ) -> Result<CliValue, CliError> {
        match value {
            Some(input) => {
                match input.parse::<i32>() {
                    Ok(num) => Ok(CliValue::Int(num)),
                    Err(_) => Err(CliError::InvalidInt(input)),
                }
            },
            None => Err(CliError::NullInt(name.to_string())),
        }
    }

    fn parse_opt_int_option(
        &self,
        value: Option<String>,
        default: i32,
    ) -> Result<CliValue, CliError> {
        match value {
            Some(input) => {
                match input.parse::<i32>() {
                    Ok(num) => Ok(CliValue::Int(num)),
                    Err(_) => Err(CliError::InvalidInt(input)),
                }
            },
            None => Ok(CliValue::Int(default)),
        }
    }

    fn parse_text_option(
        &self,
        name: &str,
        value: Option<String>
    ) -> Result<CliValue, CliError> {
        if let Some(text) = value {
            Ok(CliValue::Text(text))
        } else {
            Err(CliError::NullArg(name.to_string()))
        }
    }

    fn parse_opt_string_option(
        &self,
        value: Option<String>,
        default: String,
    ) -> Result<CliValue, CliError> {
        let text: String = value.unwrap_or(default);
        Ok(CliValue::Text(text))
    }

    fn parse_list_option(
        &self,
        name: &str,
        value: Option<String>,
        sep: char,
    ) -> Result<CliValue, CliError> {
        match value {
            Some(input) => {
                let mut list: Vec<String> = Vec::new();
                if !input.is_empty() {
                    for item in input.split(sep) {
                        list.push(item.to_string())
                    }
                }
                Ok(CliValue::List(list))
            },
            None => Err(CliError::NullArg(name.to_string()))
        }
    }
}

impl<T> CliReadConfigs<&[(T, CliOption)]> for CliParser
where T: ToString
{
    fn read_configs(&mut self, entry_list: &[(T, CliOption)]) -> &mut Self {
        for (name, value) in entry_list {
            self.configs.insert(name.to_string(), value.to_owned());
        }
        self
    }
}

impl CliReadConfigs<Vec<(String, CliOption)>> for CliParser {
    fn read_configs(&mut self, entry_list: Vec<(String, CliOption)>)
        -> &mut Self {
            for (name, value) in entry_list {
                self.configs.insert(name, value);
            }
            self
    }
}

impl CliReadConfigs<HashMap<String, CliOption>> for CliParser {
    fn read_configs(&mut self, entry_list: HashMap<String, CliOption>)
        -> &mut Self {
            self.configs = entry_list;
            self
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CliError {
    InvalidInt(String),
    NullArg(String),
    NullInt(String),
    ConfigsNotFound,
    TargetNotFound(String),
    Alias2Alias,
    TypeNotSupported(String),
    OptionNotSupported(String),
    EmptyList,
    Generic(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::InvalidInt(value) =>
                write!(f, "'{}' is not a valid number", value),
            CliError::NullArg(name) =>
                write!(f, "option '{}' must take an argument", name),
            CliError::NullInt(name) =>
                write!(f, "option '{}' requires a numeric argument", name),
            CliError::ConfigsNotFound =>
                write!(f, "no configuration options were found"),
            CliError::TargetNotFound(name) =>
                write!(f, "alias target '{}' was not found", name),
            CliError::Alias2Alias =>
                write!(f, "cannot create an alias to another alias"),
            CliError::TypeNotSupported(name) =>
                write!(f, "type '{}' is not supported", name),
            CliError::OptionNotSupported(name) =>
                write!(f, "option '{}' is not supported", name),
            CliError::EmptyList =>
                write!(f, "arg_list cannot be empty"),
            CliError::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_cli_parser() {
        let parser = CliParser::new();
        let options: HashMap<String, CliValue> = HashMap::new();
        let operators: Vec<String> = Vec::new();
        let argv0: String = String::new();
        assert_eq!(parser.options, options);
        assert_eq!(parser.operators, operators);
        assert_eq!(parser.argv0, argv0);
    }

    #[test]
    fn read_config_slice() {
        let mut parser = CliParser::new();
        let configs = vec![
            ("port".to_string(), CliOption::Int),
            ("hostname".to_string(), CliOption::Text),
            ("entrypoint".to_string(), CliOption::Text),
            ("p".to_string(), CliOption::Alias { target: "port".to_string() }),
            ("g".to_string(), CliOption::Flag),
        ];
        parser.read_configs(configs);
        parser.parse(vec![
            "vish".to_string(),
            "-g".to_string(),
            "--port=5000".to_string(),
            "-p2000".to_string(),
            "./dist".to_string(),
        ]).unwrap();

        let mut options: HashMap<String, CliValue> = HashMap::new();

        options.insert("g".to_string(), CliValue::Flag);
        options.insert("port".to_string(), CliValue::Int(2000));

        assert_eq!(parser.options, options);

        assert_eq!(parser.options, options);
        assert_eq!(parser.operators, vec!["./dist"]);
        assert_eq!(parser.argv0, "vish");
    }

    #[test]
    fn missing_required_argument() {
        let mut parser = CliParser::new();
        parser.read_configs(vec![
            ("port".to_string(), CliOption::Int),
        ]);

        let result = parser.parse(vec![
            "vish".to_string(),
            "--port".to_string(),
        ]);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::NullInt(_)));
    }


    #[test]
    fn invalid_integer_value() {
        let mut parser = CliParser::new();
        parser.read_configs(vec![
            ("port".to_string(), CliOption::Int),
        ]);

        let result = parser.parse(vec![
            "vish".to_string(),
            "--port=abc".to_string(),
        ]);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::InvalidInt(_)));
    }

    #[test]
    fn optional_integer_with_default() {
        let mut parser = CliParser::new();
        parser.read_configs(vec![
            ("retry".to_string(), CliOption::OptInt { default: 3 }),
        ]);

        parser.parse(vec![
            "vish".to_string(),
            "--retry".to_string(),
        ]).unwrap();

        assert_eq!(
            parser.options.get("retry"),
            Some(&CliValue::Int(3))
        );
    }

    #[test]
    fn list_option_with_separator() {
        let mut parser = CliParser::new();
        parser.read_configs(vec![
            ("paths".to_string(), CliOption::List { sep: ':' }),
        ]);

        parser.parse(vec![
            "vish".to_string(),
            "--paths=a:b:c".to_string(),
        ]).unwrap();

        assert_eq!(
            parser.options.get("paths"),
            Some(&CliValue::List(vec![
                "a".into(), "b".into(), "c".into()
            ]))
        );
    }

    #[test]
    fn combined_short_flags() {
        let mut parser = CliParser::new();
        let configs = [
            ("a", CliOption::Flag),
            ("b", CliOption::Flag),
            ("c", CliOption::Flag),
        ].as_ref();

        parser.read_configs(configs);

        parser.parse(vec![
            "vish".to_string(),
            "-abc".to_string(),
        ]).unwrap();

        assert_eq!(parser.options.get("a").unwrap(), &CliValue::Flag);
        assert_eq!(parser.options.get("b").unwrap(), &CliValue::Flag);
        assert_eq!(parser.options.get("c").unwrap(), &CliValue::Flag);
    }

    #[test]
    fn list_option_multiple_times() {
        let mut parser = CliParser::new();
        parser.read_configs(vec![
            ("include".to_string(), CliOption::List { sep: ',' }),
        ]);

        parser.parse(vec![
            "vish".to_string(),
            "--include=a,b".to_string(),
            "--include=c,d".to_string(),
        ]).unwrap();

        assert_eq!(
            parser.options.get("include"),
            Some(&CliValue::List(vec![
                "a".into(), "b".into(), "c".into(), "d".into()
            ]))
        );
    }

    #[test]
    fn stop_parsing_after_double_dash() {
        let mut parser = CliParser::new();
        parser.read_configs(vec![
            ("verbose".to_string(), CliOption::Flag),
        ]);

        parser.parse(vec![
            "vish".to_string(),
            "--verbose".to_string(),
            "--".to_string(),
            "--not-an-option".to_string(),
        ]).unwrap();

        assert_eq!(
            parser.options.get("verbose"),
            Some(&CliValue::Flag)
        );
        assert_eq!(
            parser.operators,
            vec!["--not-an-option".to_string()]
        );
    }
}
