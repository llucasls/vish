pub mod app;
pub mod vish;
pub mod macros;
pub mod util;
use std::process::Termination;

use crate::app::Shell;

fn main() -> impl Termination { Shell::main() }
