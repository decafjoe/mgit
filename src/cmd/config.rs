//! `config` subcommand.
use clap::{App, SubCommand};

use app::Invocation;

/// Name of the command (`config`).
pub const NAME: &str = "config";

/// Returns configured clap subcommand for `config`.
pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints configuration as interpreted by mgit")
}

/// Executes the `config` subcommand.
pub fn run(invocation: &Invocation) {
    invocation
        .control()
        .fatal("config subcommand is not implemented");
}
