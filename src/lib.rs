extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate git2;
extern crate ini;
extern crate pager;
extern crate walkdir;

mod cfg;
mod cmd;

use std::env;
use std::path::PathBuf;
use std::process;

use ansi_term::Color::{Red, Yellow};
use clap::{App, Arg, SubCommand};
use walkdir::WalkDir;

use cfg::Config;
use cmd::pull;
use cmd::status;

const CONFIG_ARG: &str = "CONFIG";
const QUIET_ARG: &str = "QUIET";

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
        .subcommand(pull::subcommand())
        .subcommand(status::subcommand())
        .get_matches();

    let w = !matches.is_present(QUIET_ARG);

    let mut config = Config::new();
    for p in matches.values_of(CONFIG_ARG).unwrap().collect::<Vec<_>>() {
        let path = if p.starts_with("~/") {
            let mut path = match env::home_dir() {
                Some(path) => path,
                None => panic!("could not determine home directory"),
            };
            if p.len() > 2 {
                path.push(&p[2..]);
            }
            path
        } else {
            PathBuf::from(p)
        };
        let path_str = path.to_str().unwrap();
        if path.exists() {
            if path.is_file() {
                read_config_file(w, &mut config, path_str);
            } else if path.is_dir() {
                for entry in WalkDir::new(&path) {
                    if let Ok(entry) = entry {
                        let path_str = entry.path().to_str().unwrap();
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.is_file() {
                                read_config_file(w, &mut config, path_str);
                            }
                        } else {
                            print_warning(w, &format!(
                                "failed to get metadata for {}", path_str));
                        }
                    }
                }
            } else {
                print_warning(w, &format!(
                    "{} not a file or directory, or could not be read",
                    path_str));
            }
        } else {
            print_warning(w, &format!(
                "{} does not exist or could not be read", path_str));
        }
    }

    let mut fatal = false;
    if config.group_count() < 1 {
        print_error("no configuration files were read");
        fatal = true;
    }
    if config.repo_count() < 1 {
        print_error("no repositories configured");
        fatal = true;
    }
    if fatal {
        process::exit(1);
    }

    if let Some(matches) = matches.subcommand_matches(pull::NAME) {
        pull::run(&config, &matches);
    } else if let Some(matches) = matches.subcommand_matches(status::NAME) {
        status::run(&config, &matches);
    } else {
        print_error("no command supplied, see `mgit --help` for usage info");
        process::exit(1);
    }
}

fn read_config_file(warnings: bool, config: &mut Config, path: &str) {
    if let Err(errs) = config.push(&path) {
        for e in errs {
            print_warning(warnings, &format!(
                "({}) {}", e.path(), e.message()));
        }
    }
}

fn print_error(message: &str) {
    eprintln!("  {} {}", Red.bold().paint("error"), message);
}

fn print_warning(warnings: bool, message: &str) {
    if warnings {
        eprintln!("{} {}", Yellow.bold().paint("warning"), message);
    }
}
