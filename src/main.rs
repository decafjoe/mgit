#[macro_use]
extern crate clap;
extern crate walkdir;

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use clap::{App, Arg};
use walkdir::WalkDir;

const CONFIG_ARG: &str = "CONFIG";

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
        .get_matches();

    let config_paths: Vec<&str> = match m.values_of(CONFIG_ARG) {
        Some(config_paths) => config_paths.collect(),
        None => vec!["~/.mgit"],
    };

    let mut config: HashMap<&str, &str> = HashMap::new();
    for path in config_paths {
        let path_buf = if path.starts_with("~/") {
            let mut path_buf = match env::home_dir() {
                Some(path_buf) => path_buf,
                None => panic!("could not determine home directory"),
            };
            if path.len() > 2 {
                path_buf.push(&path[2..]);
            }
            path_buf
        } else {
            PathBuf::from(path)
        };
        populate_config_from_path(&config, &path_buf);
    }
}

fn populate_config_from_path(config: &HashMap<&str, &str>, path: &Path) {
    if path.is_file() {
        populate_config_from_file(&config, &path);
    } else {
        for entry in WalkDir::new(path) {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        populate_config_from_file(&config, &entry.path());
                    }
                } else {
                    panic!("failed to get metadata for path");
                }
            }
        }
    }
}

fn populate_config_from_file(config: &HashMap<&str, &str>, path: &Path) {
    println!("populating config from file: {:?}", path);
}
