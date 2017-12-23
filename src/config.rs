use std::error::Error as StdError;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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
    config_path: String,
    path: String,
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
            config_path: config_path.to_owned(),
            path: repo_path.to_owned(),
            name: name,
            comment: comment,
            symbol: symbol,
            tags: Vec::new(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn tags(&self) -> &[String] {
        self.tags.as_slice()
    }
}


// ----- ReposIterator --------------------------------------------------------

pub struct ReposIterator<'a> {
    config: &'a Config,
    indices: Vec<usize>,
}

impl<'a> ReposIterator<'a> {
    pub fn new(config: &'a Config, indices: &[usize]) -> Self {
        Self{ config: config, indices: indices.to_vec() }
    }
}

impl<'a> Iterator for ReposIterator<'a> {
    type Item = &'a Repo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.indices.len() > 0 {
            Some(&self.config.repos()[self.indices.remove(0)])
        } else {
            None
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
                // TODO(jjoyce): iterate through repos and look for
                //               another one with the same path?
                //               could use self.repos() here?
                self.repos.push(Repo::new(
                    path,
                    repo_path,
                    ini.get_from(Some(repo_path.to_string()), NAME_KEY),
                    ini.get_from(Some(repo_path.to_string()), COMMENT_KEY),
                    ini.get_from(Some(repo_path.to_string()), SYMBOL_KEY)));
                // TODO(jjoyce): parse and populate tags
            }
        }

        if errors.len() > 0 {
            Err(errors)
        } else {
            Ok(())
        }
    }

    pub fn repo(&self, path: &str) -> Option<&Repo> {
        for repo in self.repos() {
            if path == repo.path() {
                return Some(repo)
            }
        }
        None
    }

    pub fn repos(&self) -> &[Repo] {
        self.repos.as_slice()
    }

    pub fn repos_iter(&self) -> ReposIterator {
        let indices = (0..self.repos.len()).collect::<Vec<_>>();
        ReposIterator::new(self, indices.as_slice())
    }

    pub fn repos_tagged(&self, tag: &str) -> ReposIterator {
        let tag = String::from(tag);
        let mut indices = Vec::new();
        for (i, repo) in self.repos.iter().enumerate() {
            if repo.tags().contains(&tag) {
                indices.push(i);
            }
        }
        ReposIterator::new(self, indices.as_slice())
    }
}
