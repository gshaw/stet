//! The `stet` command: flags and dispatch.

use std::process::ExitCode;

use clap::Parser;

/// Keep Markdown knowledge repos in shape.
#[derive(Debug, Parser)]
#[command(name = "stet", version = crate::VERSION)]
pub struct Cli {}

pub fn main() -> ExitCode {
    match Cli::try_parse() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            let _ = e.print();
            ExitCode::from(64)
        }
    }
}
