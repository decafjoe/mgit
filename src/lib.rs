//! Library that drives mgit.
//!
//! Note that this is (the third iteration of) my first real Rust project. In
//! other words: it's not the utter disaster that was the first version, but I
//! still don't know whether I'm using Rust "correctly." If the code looks
//! wonky, it's probably not because it's doing something clever; it's probably
//! because it's wonky. (Please let me know! Working out the wonkiness is an
//! important part of this exercise.)
//!
//! In terms of performance (namely, copying values around) mgit tries to be as
//! reasonable as possible. References are used wherever my not-quite-complete
//! understanding allows for it. But if there's a choice between clean, clear
//! code that copies values and some monstrosity that's nasty but avoids
//! copies, mgit chooses clean+copy. In practice, this usually means copying a
//! small-ish (say, < 100 items) collection of primitive values, so the actual
//! performance hit is negligible.
//!
//! The only place where mgit really cares about performance is when doing git
//! operations. Compared to (e.g.) reading INI files or iterating through vecs,
//! git operations are extremely expensive. Where possible the results of
//! git operations are cached and reused, with the assumption that the
//! repositories won't be changed from the outside while mgit is running. (And
//! if they are, the effect is that some results may be out-of-date – nothing
//! critical.)
extern crate ansi_term;
#[macro_use]
extern crate chan;
extern crate chan_signal;
#[macro_use]
extern crate clap;
extern crate crossbeam;
extern crate git2;
extern crate indexmap;
extern crate ini;
extern crate pager;
extern crate termion;
extern crate users;
extern crate walkdir;

mod app;
mod cmd;
mod ui;

use std::{
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use chan_signal::Signal;

use app::{init, Command};
use cmd::{config, pull, status};

static COMMANDS: [Command; 3] = [
    Command {
        name: config::NAME,
        about: config::ABOUT,
        args: config::args,
        run: config::run,
    },
    Command {
        name: pull::NAME,
        about: pull::ABOUT,
        args: pull::args,
        run: pull::run,
    },
    Command {
        name: status::NAME,
        about: status::ABOUT,
        args: status::args,
        run: status::run,
    },
];

fn exit(code: i32) {
    process::exit(code);
}

/// Entry point for the program.
pub fn main() {
    // Channel to listen to for termination signals.
    let terminate_signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);

    // Two copies of the refcell that hold the "received terminate signal" state.
    // One copy for the invocation instance, which is moved to a separate thread,
    // and one for the main thread, which uses it to pass the signal through the
    // invocation to the subcommand thread.
    let terminate_arc = Arc::new(AtomicBool::new(false));
    let init_terminate_arc = Arc::clone(&terminate_arc);

    // Initialize the application, allowing a term signal to immediately exit the
    // process.
    let (init_done_tx, init_done) = chan::sync(0);
    let init_guard = thread::spawn(move || init(init_done_tx, exit, init_terminate_arc, &COMMANDS));
    chan_select! {
        init_done.recv() => {},
        terminate_signal.recv() -> _ => {
            eprintln!();
            exit(1);
        },
    }

    // Unwrap the invocation value returned by the init thread.
    let invocation = init_guard
        .join()
        .expect("failed to get results from init function");

    // Run the subcommand in a separate thread, keeping the main thread free to
    // listen for terminate signals.
    let (run_done_tx, run_done) = chan::sync(0);
    thread::spawn(move || invocation.command().run(run_done_tx, &invocation));

    // If we get a terminate signal, set the "should terminate" flag. The
    // subcommand can check this via the `Invocation.should_terminate()`
    // method. When the flag is set, the subcommand should clean up and exit as
    // soon as it can.
    chan_select! {
        run_done.recv() => { exit(0); },
        terminate_signal.recv() -> _ => {
            terminate_arc.store(true, Ordering::Relaxed);
        },
    }

    // If we get a second terminate signal before the command has cleaned up and
    // exited, hard bail out of the process.
    chan_select! {
        run_done.recv() => {},
        terminate_signal.recv() => { eprintln!(); },
    }
    exit(1);
}
