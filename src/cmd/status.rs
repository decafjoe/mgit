//! `status` subcommand.
use clap::{App, SubCommand};

use app::Invocation;

/// Name of the command (`status`).
pub const NAME: &str = "status";

/// Returns configured clap subcommand for `status`.
pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME).about("Prints current status of repositories")
}

/// Executes the `status` subcommand.
pub fn run(invocation: &Invocation) {
    invocation
        .control()
        .fatal("status subcommand is not implemented");
}
