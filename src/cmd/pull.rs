use clap::{App, ArgMatches};

use cfg::Config;

pub const NAME: &str = "pull";
pub const ABOUT: &str = "Fetch remotes, move tracking refs forward if safe";

pub fn app<'a>() -> App<'a, 'a> {
    App::new(NAME).about(ABOUT)
}
pub fn run(_: &Config, _: &ArgMatches) {}
