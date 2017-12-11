use clap::{App, ArgMatches};

use cfg::Config;

pub const NAME: &str = "pull";
pub const ABOUT: &str = "Fetch remotes, move tracking refs forward if safe";

pub fn setup(_: &App) {}
pub fn run(_: &Config, _: &ArgMatches) {}
