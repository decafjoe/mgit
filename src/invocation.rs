use std::process;

use clap::ArgMatches;

use ansi_term::Color::{Red, Yellow};
use config::Config;


// ----- WarningAction --------------------------------------------------------

#[derive(PartialEq)]
pub enum WarningAction {
    Ignore,
    Print,
    Exit,
}


// ----- Control --------------------------------------------------------------

pub struct Control {
    warning: WarningAction,
}

impl Control {
    pub fn new(warning: WarningAction) -> Self {
        Self{ warning: warning }
    }

    pub fn warning(&self, message: &str) {
        if self.warning != WarningAction::Ignore {
            eprintln!("{} {}", Yellow.bold().paint("warning"), message);
            if self.warning == WarningAction::Exit {
                process::exit(1);
            }
        }
    }

    pub fn error(&self, message: &str) {
        eprintln!("  {} {}", Red.bold().paint("error"), message);
        process::exit(1);
    }
}


// ----- Invocation -----------------------------------------------------------

pub struct Invocation<'a> {
    config: &'a Config,
    control: &'a Control,
    matches: &'a ArgMatches<'a>,
}

impl<'a> Invocation<'a> {
    pub fn new(config: &'a Config, matches: &'a ArgMatches,
               control: &'a Control) -> Self {
        Self {
            config: config,
            control: control,
            matches: matches,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn control(&self) -> &Control {
        &self.control
    }

    pub fn matches(&self) -> &ArgMatches {
        &self.matches
    }
}
