pub mod shell;
pub mod executor;
pub mod expander;
pub mod io;
pub mod lexer;
pub mod parser;

#[cfg(test)]
pub mod testing;

use std::process::Termination;
use std::sync::{Arc, LazyLock, RwLock};

use crate::shell::environment::Shell;

#[doc(hidden)]
type GlobalEnv = LazyLock<Arc<RwLock<Shell>>>;
pub static ENV: GlobalEnv = LazyLock::new(|| {
    Arc::new(RwLock::new(Shell::new()))
});

#[doc(hidden)]
fn main() -> impl Termination {
    crate::shell::runtime::App::main()
}
