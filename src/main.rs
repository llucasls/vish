pub mod app;
pub mod shell;
pub mod vish;
#[doc(hidden)]
pub mod macros;
pub mod util;
use std::process::Termination;
use std::sync::{Arc, LazyLock, RwLock};

use crate::shell::Shell;

#[doc(hidden)]
type GlobalEnv = LazyLock<Arc<RwLock<Shell>>>;
pub static ENV: GlobalEnv = LazyLock::new(|| {
    Arc::new(RwLock::new(Shell::new()))
});

#[doc(hidden)]
fn main() -> impl Termination {
    crate::app::App::main()
}
