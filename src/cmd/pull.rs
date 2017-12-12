use clap::{App, ArgMatches, SubCommand};

use cfg::Config;

pub const NAME: &str = "pull";
pub const ABOUT: &str = "Fetch remotes, move tracking refs forward if safe";

pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME).about(ABOUT)
}
pub fn run(_: &Config, _: &ArgMatches) {}
