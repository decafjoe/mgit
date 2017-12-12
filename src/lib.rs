extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate git2;
extern crate ini;

mod cfg;
mod cmd;

use std::process;

use ansi_term::Color::{Red, Yellow};
use clap::{App, Arg, SubCommand};

use cfg::{Config, ErrorKind};
use cmd::pull;
use cmd::status;

const CONFIG_ARG: &str = "CONFIG";

pub fn main() {
    let app = App::new("mgit")
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
             .value_name("PATH"));

    let subcommand = SubCommand::with_name(pull::NAME).about(pull::ABOUT);
    pull::setup(&subcommand);
    let app = app.subcommand(subcommand);

    let subcommand = SubCommand::with_name(status::NAME).about(status::ABOUT);
    status::setup(&subcommand);
    let app = app.subcommand(subcommand);

    let matches = app.get_matches();

    let error_prefix = Red.bold().paint("  error ");
    let error = |msg: &str| {
        eprintln!("{}{}", error_prefix, msg);
    };

    let warning_prefix = Yellow.bold().paint("warning ");
    let warning = |msg: &str| {
        eprintln!("{}{}", warning_prefix, msg);
    };

    let mut config = Config::new();
    for path in matches.values_of(CONFIG_ARG).unwrap().collect::<Vec<&str>>() {
        if let Err(e) = config.push(&path) {
            match *e.kind() {
                ErrorKind::Fatal => {
                    error(&format!("({}) {}", e.path(), e.message()));
                    process::exit(1);
                },
                ErrorKind::Warning => {
                    warning(&format!("({}) {}", e.path(), e.message()));
                }
            }
        }
    }

    if let Some(matches) = matches.subcommand_matches(pull::NAME) {
        pull::run(&config, &matches);
    } else if let Some(matches) = matches.subcommand_matches(status::NAME) {
        status::run(&config, &matches);
    } else {
        error("no command supplied, see `mgit --help` for usage info");
        process::exit(1);
    }
}
