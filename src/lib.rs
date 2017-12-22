extern crate ansi_term;
#[macro_use] extern crate clap;
extern crate ini;
extern crate users;

use clap::{App, Arg};

use config::Config;
use invocation::{Invocation, WarningAction};

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

    let warning = if matches.is_present(WARNING_ARG) {
        WarningAction::Exit
    } else if matches.is_present(QUIET_ARG) {
        WarningAction::Ignore
    } else {
        WarningAction::Print
    };

    let config = Config::new();
    let invocation = Invocation::new(&config, &matches, warning);

    if let Some(_) = matches.subcommand_matches(cmd::config::NAME) {
        cmd::config::run(&invocation);
    } else {
        let msg = "no command supplied, see `mgit --help` for usage info";
        invocation.control().error(msg);
    }
}
