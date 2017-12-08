#[macro_use]
extern crate clap;

use clap::App;

fn main() {
    App::new("mgit")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Small program for managing multiple git repositories.")
        .get_matches();
}
