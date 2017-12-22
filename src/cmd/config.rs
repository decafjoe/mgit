use clap::{App, SubCommand};

use invocation::Invocation;

pub const NAME: &str = "config";

pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints configuration values")
}
pub fn run(invocation: &Invocation) {
    invocation.control().error("the config subcommand is not yet implemented");
}
