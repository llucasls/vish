pub mod vish;
use std::process::ExitCode;

use self::vish::io::InputReader;

fn main() -> ExitCode {
    match InputReader::new() {
        Ok(mut reader) => vish::app::handle_interactive_mode(&mut reader),
        Err(_) => vish::app::handle_batch_mode(),
    }
}
