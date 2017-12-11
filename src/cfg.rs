use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FormatResult};
use std::fs::File;
use std::io::Read;

use ini::Ini;


// ----- ErrorKind ------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    Fatal,
    Warning,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> FormatResult {
        write!(f, "{}", match *self {
            ErrorKind::Fatal => "fatal",
            ErrorKind::Warning => "warning",
        })
    }
}


// ----- Error ----------------------------------------------------------------

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    path: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FormatResult {
        write!(f, "{}: in {}: {}", self.kind, self.message.as_str(),
               self.path.as_str())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.message.as_str()
    }
}

impl Error {
    fn fatal(path: &str, message: &str) -> Error {
        Error {
            kind: ErrorKind::Fatal,
            message: message.to_owned(),
            path: path.to_owned(),
        }
    }

    fn warning(path: &str, message: &str) -> Error {
        Error {
            kind: ErrorKind::Warning,
            message: message.to_owned(),
            path: path.to_owned(),
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}


// ----- Config ---------------------------------------------------------------

pub struct Config;

impl Config {
    pub fn new() -> Config {
        Config{}
    }

    pub fn push(&mut self, path: &str) -> Result<(), Error> {
        fn warning(path: &str, e: &StdError, message: &str) -> Error {
            Error::warning(&path, &format!("{} ({})", message, e))
        }

        let mut f = match File::open(path) {
            Ok(f) => f,
            Err(e) => return Err(warning(&path, &e, "could not open file")),
        };

        let mut s = String::new();
        if let Err(e) = f.read_to_string(&mut s) {
            return Err(warning(&path, &e, "could not read file"))
        }

        let ini = match Ini::load_from_str(&s) {
            Ok(ini) => ini,
            Err(e) => return Err(warning(&path, &e, "could not parse file")),
        };

        Ok(())
    }
}
