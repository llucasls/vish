use std::collections::HashMap;
use std::env;

type ShVar = HashMap<String, String>;

pub struct ShellEnvironment {
    pub shell_variables: ShVar,
}

impl ShellEnvironment {
    pub fn new() -> Self {
        let mut shell_variables = HashMap::new();
        Self::init_par(&mut shell_variables, "PS1", "$ ");
        Self::init_par(&mut shell_variables, "PS2", "> ");

        Self { shell_variables }
    }

    fn init_par(shell_variables: &mut ShVar, key: &str, default: &str) {
        shell_variables.insert(
            String::from(key),
            env::var(String::from(key)).unwrap_or(String::from(default)));
    }
}

impl Default for ShellEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
