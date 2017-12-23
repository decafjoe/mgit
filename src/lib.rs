extern crate ansi_term;
#[macro_use] extern crate clap;
extern crate ini;
extern crate users;

use clap::{App, Arg};

use config::Config;
use invocation::{Control, Invocation, WarningAction};

mod cmd;
mod config;
mod invocation;
mod path;

const CONFIG_ARG: &str = "CONFIG";
const QUIET_ARG: &str = "QUIET";
const WARNING_ARG: &str = "WARNING";

pub fn main() {
    let matches = App::new("mgit")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Small program for managing multiple git repositories.")
        .arg(Arg::with_name(CONFIG_ARG)
             .default_value("~/.mgit")
             .help("Path to configuration file or directory")
             .short("c")
             .long("config")
             .multiple(true)
             .number_of_values(1)
             .value_name("PATH"))
        .arg(Arg::with_name(QUIET_ARG)
             .help("Suppresses warning messages")
             .short("q")
             .long("quiet"))
        .arg(Arg::with_name(WARNING_ARG)
             .help("Treats warnings as errors (overrides suppression)")
             .short("w")
             .long("warning-is-error"))
        .subcommand(cmd::config::subcommand())
        .get_matches();

    let warning_action = if matches.is_present(WARNING_ARG) {
        WarningAction::Exit
    } else if matches.is_present(QUIET_ARG) {
        WarningAction::Ignore
    } else {
        WarningAction::Print
    };
    let control = Control::new(warning_action);

    let config = Config::new();

    if let Some(m) = matches.subcommand_matches(cmd::config::NAME) {
        cmd::config::run(&Invocation::new(&config, &m, &control));
    } else {
        control.error("no command supplied, see `mgit -h` for usage info")
    }
}
