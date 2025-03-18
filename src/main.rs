pub mod app;
pub mod vish;
pub mod macros;
use std::process::Termination;

use crate::app::Shell;

fn main() -> impl Termination { Shell::main() }
