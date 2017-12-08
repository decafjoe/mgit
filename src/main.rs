#[macro_use]
extern crate clap;
extern crate git2;
extern crate ini;
extern crate walkdir;

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process;

use clap::{App, Arg, SubCommand};
use git2::{Repository, StatusOptions, StatusShow};
use ini::Ini;
use walkdir::WalkDir;

const CONFIG_ARG: &str = "CONFIG";
const PULL_CMD: &str = "pull";
const STATUS_CMD: &str = "status";
const VERBOSE_ARG: &str = "VERBOSE";

fn main() {
    let m = App::new("mgit")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Small program for managing multiple git repositories.")
        .arg(Arg::with_name(CONFIG_ARG)
             .help("Path to configuration file or directory")
             .short("c")
             .long("config")
             .multiple(true)
             .number_of_values(1)
             .value_name("PATH"))
        .subcommand(SubCommand::with_name(PULL_CMD)
                    .about("Fetch from remotes, move tracking refs forward"))
        .subcommand(SubCommand::with_name(STATUS_CMD)
                    .about("Print repo status to stdout")
                    .arg(Arg::with_name(VERBOSE_ARG)
                         .help("Print status of up-to-date branches")
                         .short("v")
                         .long("verbose")))
        .get_matches();

    let config_paths: Vec<&str> = match m.values_of(CONFIG_ARG) {
        Some(config_paths) => config_paths.collect(),
        None => vec!["~/.mgit"],
    };

    let mut config: HashMap<String, Repository> = HashMap::new();
    for raw_path in config_paths {
        let path = if raw_path.starts_with("~/") {
            let mut path = match env::home_dir() {
                Some(path) => path,
                None => panic!("could not determine home directory"),
            };
            if raw_path.len() > 2 {
                path.push(&raw_path[2..]);
            }
            path
        } else {
            PathBuf::from(raw_path)
        };
        if path.is_file() {
            config.extend(read_config_file(&path));
        } else if path.is_dir() {
            for entry in WalkDir::new(path) {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            config.extend(read_config_file(&entry.path()));
                        }
                    } else {
                        panic!("failed to get metadata for: {:?}", raw_path);
                    }
                }
            }
        }
    }

    if let Some(_) = m.subcommand_matches(PULL_CMD) {
        pull(&config);
    } else if let Some(_) = m.subcommand_matches(STATUS_CMD) {
        status(&config);
    } else {
        println!("no command suppled, see `mgit --help` for usage info");
        process::exit(1);
    }
}

fn read_config_file(path: &Path) -> HashMap<String, Repository> {
    let mut config = HashMap::new();
    if let Some(ext) = path.extension() {
        if ext == "conf" {
            if let Ok(ini) = Ini::load_from_file(path) {
                if let Some(repos) = ini.section(Some("repos")) {
                    for (key, value) in repos.iter() {
                        if let Ok(repo) = Repository::open(value) {
                            config.insert(key.to_owned(), repo);
                        } else {
                            panic!("failed to open repo: {}", value);
                        };
                    }
                }
            } else {
                panic!("failed to read configuration file: {:?}", path);
            }
        }
    }
    config
}

fn pull(config: &HashMap<String, Repository>) {
    println!("pull command is not yet implemented");
    process::exit(1);
}

fn status(config: &HashMap<String, Repository>) {
    let mut status_options = StatusOptions::new();
    status_options.show(StatusShow::IndexAndWorkdir);
    status_options.include_untracked(true);
    status_options.exclude_submodules(true);
    status_options.recurse_untracked_dirs(true);
    status_options.renames_head_to_index(true);
    status_options.renames_index_to_workdir(true);
    status_options.renames_from_rewrites(true);
    for (name, repo) in config {
        if let Ok(statuses) = repo.statuses(Some(&mut status_options)) {
            println!("success!");
        } else {
            panic!("failed to get status info from: {:?}", repo.path());
        };
    }
}
