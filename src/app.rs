//! Top-level application code, state management, and program control.
use std::process;

use clap::{App, Arg, ArgMatches};

/// Name for the `-W/--warning` argument.
const WARNING_ARG: &str = "WARNING";

/// Returns configured top-level clap `App` instance.
pub fn app<'a>() -> App<'a, 'a> {
    App::new("mgit")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Small program for managing multiple git repositories.")
        .arg(
            Arg::with_name(WARNING_ARG)
                .default_value("print")
                .help("Action to take on warnings")
                .short("W")
                .long("warning")
                .possible_values(&["ignore", "print", "fatal"])
                .takes_value(true)
                .value_name("ACTION"),
        )
}

/// Runs the application with the specified `matches`, returning
/// initialized state/control instances.
pub fn run(matches: &ArgMatches) -> Control {
    let warning_action_value = matches
        .value_of(WARNING_ARG)
        .expect("no value for warning action argument");
    let warning_action = match warning_action_value {
        "ignore" => Action::Ignore,
        "print" => Action::Print,
        "fatal" => Action::Fatal,
        &_ => panic!(
            "unexpected value for warning action ('{}')",
            warning_action_value
        ),
    };
    let control = Control::new(warning_action);
    control.warning("this is a test warning"); // TODO(jjoyce): kill this
    control
}

// ----- Action ---------------------------------------------------------------

/// Represents an action to take in response to an error condition.
#[derive(PartialEq)]
pub enum Action {
    /// Ignore the error, do nothing.
    Ignore,
    /// Print the error but continue execution.
    Print,
    /// Print the error, then terminate execution with a fatal error.
    Fatal,
}

// ----- Control --------------------------------------------------------------

/// High level program control – warnings and fatal errors.
pub struct Control {
    /// Action to take on warnings.
    warning_action: Action,
}

impl Control {
    /// Creates and returns a new control instance.
    pub fn new(warning_action: Action) -> Self {
        Self {
            warning_action: warning_action,
        }
    }

    /// Registers a warning with the specified `message`.
    ///
    /// The action taken depends on the `warning_action` supplied to
    /// the constructor:
    ///
    /// * If `Ignore`, nothing is done.
    /// * If `Print`, `message` is printed to stderr.
    /// * If `Fatal`, `message` is printed to stderr, then `fatal()`
    ///   is called with an error message noting that warnings are
    ///   fatal.
    pub fn warning(&self, message: &str) {
        if self.warning_action != Action::Ignore {
            eprintln!("{}", message);
            if self.warning_action == Action::Fatal {
                self.fatal("encountered warning, warning action is 'fatal'");
            }
        }
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
