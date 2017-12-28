//! Code for handling invocations of the program.
use std::process;

use clap::ArgMatches;

use ansi_term::Color::{Red, Yellow};
use config::Config;

// ----- WarningAction --------------------------------------------------------

/// Indicates the action to take upon encountering a warning.
#[derive(PartialEq)]
pub enum WarningAction {
    /// Do nothing, stay silent.
    Ignore,
    /// Display the warning to the end user.
    Print,
    /// Display the warning, exit the program.
    Exit,
}

// ----- Control --------------------------------------------------------------

/// Warnings, errors, and program control API.
pub struct Control {
    /// Action to take when encountering a warning.
    warning: WarningAction,
}

impl Control {
    /// Creates and returns a new control instance, which will do
    /// `warning` action on warnings.
    pub fn new(warning: WarningAction) -> Self {
        Self { warning: warning }
    }

    /// Registers a warning.
    ///
    /// What actually happens depends on the warning action for the
    /// particular invocation of the program.
    pub fn warning(&self, message: &str) {
        if self.warning != WarningAction::Ignore {
            eprintln!("{} {}", Yellow.bold().paint("warning"), message);
            if self.warning == WarningAction::Exit {
                process::exit(1);
            }
        }
    }

    /// Prints an error to the console and exits the program.
    pub fn error(&self, message: &str) {
        eprintln!("  {} {}", Red.bold().paint("error"), message);
        process::exit(1);
    }
}

// ----- Invocation -----------------------------------------------------------

/// All the state for a given invocation of the program.
pub struct Invocation<'a> {
    /// Parsed configuration.
    config: &'a Config,
    /// `Control` instance.
    control: &'a Control,
    /// Argument matches for the called subcommand.
    matches: &'a ArgMatches<'a>,
}

impl<'a> Invocation<'a> {
    /// Creates and returns a new invocation instance.
    pub fn new(
        config: &'a Config,
        matches: &'a ArgMatches,
        control: &'a Control,
    ) -> Self {
        Self {
            config: config,
            control: control,
            matches: matches,
        }
    }

    /// Returns the config struct for this invocation.
    pub fn config(&self) -> &Config {
        self.config
    }

    /// Returns the control struct for this invocation.
    pub fn control(&self) -> &Control {
        self.control
    }

    /// Returns the matches struct for this invocation.
    pub fn matches(&self) -> &ArgMatches {
        self.matches
    }
}
