//! Library that drives mgit.
extern crate ansi_term;
#[macro_use] extern crate clap;
extern crate ini;
extern crate pager;
extern crate users;
extern crate walkdir;

use clap::{App, Arg};
use pager::Pager;
use walkdir::WalkDir;

use config::Config;
use invocation::{Control, Invocation, WarningAction};

mod cmd;
mod config;
mod invocation;
mod path;

/// Argument name for -c/--config.
const CONFIG_ARG: &str = "CONFIG";
/// Argument name for -q/--quiet.
const QUIET_ARG: &str = "QUIET";
/// Argument name for -w/--warning.
const WARNING_ARG: &str = "WARNING";

/// Entry point for the program.
pub fn main() {
    Pager::with_pager("less -efFnrX").setup();

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

    let mut config = Config::new();
    for path_str in matches.values_of(CONFIG_ARG).unwrap() {
        let path = match path::expand(path_str) {
            Ok(path_buf) => path_buf,
            Err(e) => {
                control.warning(&format!(
                    "{}: could not resolve path ({})", path_str, e));
                continue
            },
        };
        if !path.exists() {
            control.warning(&format!(
                "{}: does not exist or could not be read", path_str));
            continue
        }
        if path.is_file() {
            if let Err(errors) = config.read(path_str) {
                for e in errors {
                    control.warning(&format!("{}: {}", path_str, e));
                }
            }
            continue
        }
        if !path.is_dir() {
            control.warning(&format!(
                "{}: not a file or directory, or could not be read",
                path_str));
            continue
        }
        for entry in WalkDir::new(&path) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    control.warning(&format!(
                        "{}: failure when walking directory ({})",
                        path_str, e));
                    continue
                }
            };
            if entry.path().is_file() {
                let p = entry.path().to_str().expect(&format!(
                    "{}: failure while walking directory (could not turn \
                     an entry's path into str - invalid unicode?)", path_str));
                if let Err(errors) = config.read(p) {
                    for e in errors {
                        control.warning(&format!("{}: {}", p, e))
                    }
                }
            }
        }
    }

    if config.repos().len() < 1 {
        control.error("no repositories configured")
    }

    if let Some(m) = matches.subcommand_matches(cmd::config::NAME) {
        cmd::config::run(&Invocation::new(&config, &m, &control));
    } else {
        control.error("no command supplied, see `mgit -h` for usage info")
    }
}
