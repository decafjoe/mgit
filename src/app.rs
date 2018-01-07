//! Top-level program state and handlers.
use std::process;

use clap::{App, ArgMatches};

/// Returns configured top-level clap `App` instance.
pub fn app<'a>() -> App<'a, 'a> {
    App::new("mgit")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Small program for managing multiple git repositories.")
}

pub fn run(_matches: &ArgMatches) -> Control {
    Control::new()
}

// ----- Control --------------------------------------------------------------

/// High level program control – warnings and fatal errors.
pub struct Control;

impl Control {
    /// Creates and returns a new control instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Prints `message` to stderr, then exits the process with an
    /// exit code of `1`.
    pub fn fatal(&self, message: &str) {
        eprintln!("{}", message);
        process::exit(1);
    }
}

// ----- Invocation -----------------------------------------------------------

/// All state for a given invocation of the program.
pub struct Invocation<'a> {
    /// `Control` instance.
    control: &'a Control,
}

impl<'a> Invocation<'a> {
    /// Creates and returns a new invocation instance.
    pub fn new(control: &'a Control) -> Self {
        Self { control: control }
    }

    /// Returns the control struct for this invocation.
    pub fn control(&self) -> &Control {
        self.control
    }
}
