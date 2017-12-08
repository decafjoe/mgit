#[macro_use]
extern crate clap;
extern crate ini;
extern crate walkdir;

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use clap::{App, Arg};
use ini::Ini;
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

    let mut config: HashMap<String, String> = HashMap::new();
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
        config.extend(read_config_path(&path_buf));
    }

    for (name, path) in config {
        println!("{}: {}", name, path);
    }
}

fn read_config_path(path: &Path) -> HashMap<String, String> {
    if path.is_file() {
        read_config_file(path)
    } else if path.is_dir() {
        let mut config = HashMap::new();
        for entry in WalkDir::new(path) {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        config.extend(read_config_file(&entry.path()));
                    }
                } else {
                    panic!("failed to get metadata for path: {:?}", path);
                }
            }
        }
        config
    } else {
        HashMap::new()
    }
}

fn read_config_file(path: &Path) -> HashMap<String, String> {
    let mut config = HashMap::new();
    if let Some(ext) = path.extension() {
        if ext == "conf" {
            if let Ok(ini) = Ini::load_from_file(path) {
                if let Some(repos) = ini.section(Some("repos")) {
                    for (key, value) in repos.iter() {
                        config.insert(key.to_owned(), value.to_owned());
                    }
                }
            } else {
                panic!("failed to read configuration file: {:?}", path);
            }
        }
    }
    config
}
