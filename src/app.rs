//! Top-level application code, state management, and program control.
use std::process;

use ansi_term::{Color, Style};
use clap::{App, Arg, ArgMatches};

use cfg::Config;

/// Name for the `-c/--config` argument.
const CONFIG_ARG: &str = "CONFIG";
/// Name for the `-W/--warning` argument.
const WARNING_ARG: &str = "WARNING";

/// Returns configured top-level clap `App` instance.
pub fn app<'a>() -> App<'a, 'a> {
    App::new("mgit")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Small program for managing multiple git repositories.")
        .arg(
            Arg::with_name(CONFIG_ARG)
                .default_value("~/.mgit")
                .help("Path to configuration file or directory")
                .short("c")
                .long("config")
                .multiple(true)
                .number_of_values(1)
                .value_name("PATH"),
        )
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
pub fn run(matches: &ArgMatches) -> (Control, Config) {
    // Pull out provided arguments. Per the configuration in `app()`,
    // clap should make sure none of this ever actually panics.
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
    let config_paths = matches
        .values_of(CONFIG_ARG)
        .expect("no value for config argument");

    // Read the configuration from the provided `-c/--config` paths,
    // passing errors from the config reader to the control instance,
    // as warnings.
    let control = Control::new(warning_action);
    let mut config = Config::new();
    for path in config_paths {
        for error in config.read(path) {
            let mut s =
                format!("{}", Style::new().bold().paint(error.message()));
            if let Some(cause) = error.cause() {
                s.push_str(&format!("\n{}", cause));
            }
            s.push_str(&format!(
                "\nin config at path {}",
                Color::Cyan.bold().paint(error.config_path())
            ));
            if let Some(repo_path) = error.repo_path() {
                s.push_str(&format!(
                    "\nfor repo  at path {}",
                    Color::Blue.bold().paint(repo_path)
                ));
            }
            control.warning(s.as_str());
        }
    }

    // Return and transfer ownership of control and config instances.
    (control, config)
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

    /// Prints error condition to stdout.
    ///
    /// `label` indicates the type of the condition, `"warning"` or
    /// `"  fatal"` (the labels are "manually" aligned).
    ///
    /// `color` indicates the color for the `label`. The color will be
    /// `bold()`-ed.
    ///
    /// `message` is the message to print to stderr. If the message
    /// contains multiple lines, lines subsequent to the first are
    /// indented to `label.len()` plus one.
    fn print(&self, label: &str, color: Color, message: &str) {
        let mut s = String::from("");
        for _ in 0..label.len() {
            s.push_str(" ");
        }
        let empty = s.as_str();
        for (i, line) in message.lines().enumerate() {
            let margin = if i == 0 { label } else { empty };
            eprintln!("{} {}", color.bold().paint(margin), line);
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
            self.print("warning", Color::Yellow, message);
            if self.warning_action == Action::Fatal {
                self.fatal("encountered warning, warning action is 'fatal'");
            }
        }
    }

    /// Prints `message` to stderr, then exits the process with an
    /// exit code of `1`.
    pub fn fatal(&self, message: &str) {
        self.print("  fatal", Color::Red, message);
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
