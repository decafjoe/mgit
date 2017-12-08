#[macro_use]
extern crate clap;

use clap::{App, Arg};

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

    let paths: Vec<&str> = match m.values_of(CONFIG_ARG) {
        Some(paths) => paths.collect(),
        None => vec!["~/.mgit"],
    };
}
