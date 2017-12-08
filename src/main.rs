#[macro_use]
extern crate clap;

use clap::{App, Arg};
use std::env;
use std::path::PathBuf;

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
        println!("{:?}", path_buf);
    }
}
