pub mod executor;
pub mod expander;
pub mod io;
pub mod lexer;
pub mod parser;
pub mod utils;
pub mod types;
pub mod environment;
pub mod runtime;
pub mod errors;

#[cfg(test)]
pub mod testing;

use std::process::Termination;
use std::sync::{Arc, LazyLock, RwLock};

use crate::environment::Shell;

#[doc(hidden)]
type GlobalEnv = LazyLock<Arc<RwLock<Shell>>>;
pub static ENV: GlobalEnv = LazyLock::new(|| {
    Arc::new(RwLock::new(Shell::new()))
});

#[doc(hidden)]
fn main() -> impl Termination {
    crate::runtime::App::main()
}
