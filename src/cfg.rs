use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FormatResult};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use git2::Repository;
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


// ----- Repo -----------------------------------------------------------------

pub struct Repo {
    name: String,
    repo: Repository,
}

impl Repo {
    fn new(name: &str, repo: Repository) -> Repo {
        Repo {
            name: name.to_owned(),
            repo: repo,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}


// ----- Group ----------------------------------------------------------------

pub struct Group {
    name: String,
    path: String,
    repos: HashMap<String, Repo>,
    symbol: String,
}

impl Group {
    fn new(name: &str, path: &str, symbol: &str) -> Group {
        Group {
            name: name.to_owned(),
            path: path.to_owned(),
            repos: HashMap::new(),
            symbol: symbol.to_owned(),
        }
    }

    fn add(&mut self, repo: Repo) {
        self.repos.insert(repo.name().to_owned(), repo);
    }
}


// ----- Config ---------------------------------------------------------------

pub struct Config {
    groups: HashMap<String, Group>,
}

impl Config {
    pub fn new() -> Config {
        Config{ groups: HashMap::new() }
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

        let stem = match Path::new(path).file_stem() {
            Some(stem) => stem.to_str().unwrap(),
            None => panic!("expected there to be a file_stem for path"),
        };

        let name = ini.get_from_or(Some("group"), "name", stem);
        let symbol = ini.get_from_or(Some("group") , "symbol", "•");

        if let Some(group) = self.groups.get(name) {
            return Err(Error::fatal(&path, &format!(
                "group name {} already (other file: {})", name, group.path)))
        }

        let mut group = Group::new(&name, &path, &symbol);

        let mut failed: Vec<String> = Vec::new();
        if let Some(repos_sec) = ini.section(Some("repos")) {
            for (name, path) in repos_sec.iter() {
                if let Ok(repo) = Repository::open(path) {
                    group.add(Repo::new(&name, repo));
                } else {
                    failed.push(path.to_owned());
                };
            }
        }

        self.groups.insert(name.to_owned(), group);
        Ok(())
    }
}
