use std::error::Error as StdError;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use ini::Ini;


// ----- Error ----------------------------------------------------------------

pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: &str) -> Self {
        Self{ message: message.to_owned() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}



// ----- Repo -----------------------------------------------------------------

pub struct Repo {
    config_path: PathBuf,
    path: PathBuf,
    name: Option<String>,
    comment: Option<String>,
    symbol: Option<String>,
    tags: Vec<String>,
}

impl Repo {
    pub fn new(config_path: &str, repo_path: &str, name: Option<&str>,
               comment: Option<&str>, symbol: Option<&str>) -> Self {
        let name = match name {
            Some(s) => Some(s.to_owned()),
            None => None,
        };
        let comment = match comment {
            Some(s) => Some(s.to_owned()),
            None => None,
        };
        let symbol = match symbol {
            Some(s) => Some(s.to_owned()),
            None => None,
        };
        Self {
            config_path: PathBuf::from(config_path),
            path: PathBuf::from(repo_path),
            name: name,
            comment: comment,
            symbol: symbol,
            tags: Vec::new(),
        }
    }
}


// ----- Config ---------------------------------------------------------------

const NAME_KEY: &str = "name";
const COMMENT_KEY: &str = "comment";
const SYMBOL_KEY: &str = "symbol";
const TAGS_KEY: &str = "tags";

pub struct Config {
    repos: Vec<Repo>,
}

impl Config {
    pub fn new() -> Self {
        Config{ repos: Vec::new() }
    }

    pub fn read(&mut self, path: &str) -> Result<(), Vec<Error>> {
        fn err(msg: &str) -> Result<(), Vec<Error>> {
            Err(vec![Error::new(msg)])
        }

        fn err_e(e: &StdError, msg: &str) -> Result<(), Vec<Error>> {
            err(&format!("{} ({})", msg, e))
        }

        let p = Path::new(path);
        if !p.is_file() {
            return err(&format!("path is not a file: {}", path))
        }

        let mut f = match File::open(p) {
            Ok(f) => f,
            Err(e) => return err_e(&e, "could not open file"),
        };

        let mut s = String::new();
        if let Err(e) = f.read_to_string(&mut s) {
            return err_e(&e, "could not read file")
        }

        let ini = match Ini::load_from_str(&s) {
            Ok(ini) => ini,
            Err(e) => return err_e(&e, "could not parse file"),
        };

        let mut errors = Vec::new();
        for (repo_path, _) in &ini {
            if let Some(repo_path) = repo_path.as_ref() {
                self.repos.push(Repo::new(
                    path,
                    repo_path,
                    ini.get_from(Some(repo_path.to_string()), NAME_KEY),
                    ini.get_from(Some(repo_path.to_string()), COMMENT_KEY),
                    ini.get_from(Some(repo_path.to_string()), SYMBOL_KEY)));
                // TODO(jjoyce): iterate through repos and look for
                //               another one with the same path?
                //               should be an immutable borrow, which
                //               is ok here?
                // TODO(jjoyce): parse and populate tags
            }
        }

        if errors.len() > 0 {
            Err(errors)
        } else {
            Ok(())
        }
    }
}
