use clap::{App, ArgMatches};

use cfg::Config;

pub const NAME: &str = "status";
pub const ABOUT: &str = "Print repo status";

pub fn setup(_: &App) {}
pub fn run(_: &Config, _: &ArgMatches) {}
