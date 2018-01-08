//! Library that drives mgit.
extern crate ansi_term;
#[macro_use]
extern crate clap;

mod app;
mod cmd;

/// Entry point for the program.
pub fn main() {
    let matches = app::app()
        .subcommand(cmd::config::subcommand())
        .subcommand(cmd::pull::subcommand())
        .subcommand(cmd::status::subcommand())
        .get_matches();

    let control = app::run(&matches);

    if matches.subcommand_matches(cmd::config::NAME).is_some() {
        cmd::config::run(&app::Invocation::new(&control));
    } else if matches.subcommand_matches(cmd::pull::NAME).is_some() {
        cmd::pull::run(&app::Invocation::new(&control));
    } else if matches.subcommand_matches(cmd::status::NAME).is_some() {
        cmd::status::run(&app::Invocation::new(&control));
    } else {
        control.fatal("no command supplied, see `mgit -h` for usage info");
    }
}
