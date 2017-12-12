use clap::{App, ArgMatches, SubCommand};

use cfg::Config;

pub const NAME: &str = "pull";

pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Fetches remotes, move tracking refs forward if safe")
}
pub fn run(_: &Config, _: &ArgMatches) {}
