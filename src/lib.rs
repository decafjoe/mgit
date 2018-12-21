//! Library that drives mgit.
//!
//! Note that this is (the third iteration of) my first real Rust project. In other
//! words: it's not the utter disaster that was the first version, but I still
//! don't know whether I'm using Rust "correctly." If the code looks wonky, it's
//! probably not because it's doing something clever; it's probably because it's
//! wonky. (Please let me know! Working out the wonkiness is an important part of
//! this exercise.)
//!
//! In terms of performance (namely, copying values around) mgit tries to be as
//! reasonable as possible. References are used wherever my not-quite-complete
//! understanding allows for it. But if there's a choice between clean, clear code
//! that copies values and some monstrosity that's nasty but avoids copies, mgit
//! chooses clean+copy. In practice, this usually means copying a small-ish (say, <
//! 100 items) collection of primitive values, so the actual performance hit is
//! negligible.
//!
//! The only place where mgit really cares about performance is when doing git
//! operations. Compared to (e.g.) reading INI files or iterating through vecs, git
//! operations are extremely expensive. Where possible the results of git
//! operations are cached and reused, with the assumption that the repositories
//! won't be changed from the outside while mgit is running. (And if they are, the
//! effect is that some results may be out-of-date – nothing critical.)
extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate crossbeam;
#[macro_use]
extern crate crossbeam_channel;
extern crate git2;
extern crate indexmap;
extern crate ini;
extern crate libc;
extern crate nix;
extern crate signal_hook;
extern crate termion;
extern crate users;
extern crate walkdir;

mod app;
mod cmd;
mod ui;

use std::{
    process,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use signal_hook::{iterator::Signals, SIGINT, SIGTERM};

use app::{init, Command};
use cmd::{config, pull, status};

static COMMANDS: [Command; 3] = [
    Command {
        name: config::NAME,
        about: config::ABOUT,
        exit_on_sigterm: config::EXIT_ON_SIGTERM,
        args: config::args,
        run: config::run,
    },
    Command {
        name: pull::NAME,
        about: pull::ABOUT,
        exit_on_sigterm: pull::EXIT_ON_SIGTERM,
        args: pull::args,
        run: pull::run,
    },
    Command {
        name: status::NAME,
        about: status::ABOUT,
        exit_on_sigterm: status::EXIT_ON_SIGTERM,
        args: status::args,
        run: status::run,
    },
];

fn exit(code: i32) {
    process::exit(code);
}

/// Entry point for the program.
pub fn main() {
    // Use a separate thread to listen for SIGINT and SIGTERM, forwarding them to the
    // main thread via a channel.
    let term_signals =
        Signals::new(&[SIGINT, SIGTERM]).expect("failed to create the signals iterator");
    let (term_tx, term_rx) = crossbeam_channel::bounded(0);
    thread::spawn(move || {
        for signal in term_signals.forever() {
            term_tx
                .send(signal)
                .expect("failed to send signal over channel");
        }
    });

    // Make two copies of a refcell that hold the count of sigterms received. One copy
    // is for the invocation instance, which is moved to a separate thread, and one is
    // for the main thread, which uses it to capture signals and pass the count
    // through the invocation to the subcommand thread.
    let term_arc_main = Arc::new(AtomicUsize::new(0));
    let term_arc_invocation = Arc::clone(&term_arc_main);

    // Initialize the application, allowing a term signal to immediately exit the
    // process.
    let (init_tx, init_rx) = crossbeam_channel::bounded(0);
    let init_guard = thread::Builder::new()
        .name("init".to_string())
        .spawn(move || init(init_tx, term_arc_invocation, exit, &COMMANDS))
        .expect("failed to spawn thread for initialization");
    select! {
        recv(init_rx) -> _ => {},
        recv(term_rx) -> _ => {
            eprintln!();
            exit(1);
        },
    }

    // Unwrap the invocation value returned by the init thread.
    let invocation = init_guard
        .join()
        .expect("failed to get results from init function");

    // Grab the value of `exit_on_sigterm`. We'll need it later.
    let exit_on_sigterm = invocation.command().exit_on_sigterm;

    // Run the subcommand in a separate thread, keeping the main thread free to listen
    // for terminate signals.
    let (run_tx, run_rx) = crossbeam_channel::bounded(0);
    thread::Builder::new()
        .name("command".to_string())
        .spawn(move || invocation.command().run(run_tx, &invocation))
        .expect("failed to spawn thread for running command");

    // Loop forever, processing sigterms while waiting for the command to complete.
    loop {
        select! {
            recv(run_rx) -> _ => { exit(0); },
            recv(term_rx) -> _ => {
                if exit_on_sigterm {
                    eprintln!();
                    exit(1);
                }
                term_arc_main.fetch_add(1, Ordering::Relaxed);
            },
        }
    }
}
