//! Library that drives mgit.
//!
//! Note that this is (the third iteration of) my first real Rust
//! project. In other words: it's not the utter disaster that was the
//! first version, but I still don't know whether I'm using Rust
//! "correctly." If the code looks wonky, it's probably not because
//! it's doing something clever; it's probably because it's wonky.
//! (Please let me know! Working out the wonkiness is an important
//! part of this exercise.)
//!
//! In terms of performance (namely, copying values around) mgit tries
//! to be as reasonable as possible. References are used wherever my
//! not-quite-complete understanding allows for it. But if there's a
//! choice between clean, clear code that copies values and some
//! monstrosity that's nasty but avoids copies, mgit chooses
//! clean+copy. In practice, this usually means copying a small-ish
//! (say, < 100 items) collection of primitive values, so the actual
//! performance hit is negligible.
//!
//! The only place where mgit really cares about performance is when
//! doing git operations. Compared to (e.g.) reading INI files or
//! iterating through vecs, git operations are extremely expensive.
//! Where possible the results of git operations are cached and
//! reused, with the assumption that the repositories won't be changed
//! from the outside while mgit is running. (And if they are, the
//! effect is that some results may be out-of-date – nothing
//! critical.)
extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate git2;
extern crate ini;
extern crate ordermap;
extern crate pager;
extern crate users;
extern crate walkdir;

mod app;
mod cmd;

/// Entry point for the program.
pub fn main() {
    let matches = app::app()
        .subcommand(cmd::config::subcommand())
        .subcommand(cmd::pull::subcommand())
        .subcommand(cmd::status::subcommand())
        .get_matches();

    let (control, config) = app::run(&matches);

    if let Some(m) = matches.subcommand_matches(cmd::config::NAME) {
        cmd::config::run(&app::Invocation::new(&control, &config, m));
    } else if let Some(m) = matches.subcommand_matches(cmd::pull::NAME) {
        cmd::pull::run(&app::Invocation::new(&control, &config, m));
    } else if let Some(m) = matches.subcommand_matches(cmd::status::NAME) {
        cmd::status::run(&app::Invocation::new(&control, &config, m));
    } else {
        control.fatal("no command supplied, see `mgit -h` for usage info");
    }
}
