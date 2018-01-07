//! `pull` subcommand.
use clap::{App, SubCommand};

use app::Invocation;

/// Name of the command (`pull`).
pub const NAME: &str = "pull";

/// Returns configured clap subcommand for `pull`.
pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Fetches remotes and fast-forwards tracking branches if safe")
}

/// Executes the `pull` subcommand.
pub fn run(invocation: &Invocation) {
    invocation
        .control()
        .fatal("pull subcommand is not implemented");
}
