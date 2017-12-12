use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FormatResult};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use git2::Repository;
use ini::Ini;


// ----- Error ----------------------------------------------------------------

#[derive(Debug)]
pub struct Error {
    message: String,
    path: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FormatResult {
        write!(f, "{}: {}", self.message.as_str(), self.path.as_str())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.message.as_str()
    }
}

impl Error {
    fn new(path: &str, message: &str) -> Error {
        Error{ message: message.to_owned(), path: path.to_owned() }
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
    config_path: String,
    symbol: String,
    name: String,
    repo: Repository,
}

impl Repo {
    fn new(config_path: &str, symbol: &str, name: &str, repo: Repository)
           -> Repo {
        Repo {
            config_path: config_path.to_owned(),
            name: name.to_owned(),
            repo: repo,
            symbol: symbol.to_owned(),
        }
    }

    pub fn git(&self) -> &Repository {
        &self.repo
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn set_symbol(&mut self, symbol: &str) {
        self.symbol = symbol.to_owned();
    }
}


// ----- Group ----------------------------------------------------------------

pub struct Group {
    name: String,
    repos: HashMap<String, Repo>,
    symbol: String,
}

impl Group {
    fn new(name: &str, symbol: &str) -> Group {
        Group {
            name: name.to_owned(),
            repos: HashMap::new(),
            symbol: symbol.to_owned(),
        }
    }

    pub fn repos(&self) -> &HashMap<String, Repo> {
        &self.repos
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn set_symbol(&mut self, symbol: &str) {
        if symbol != self.symbol {
            self.symbol = symbol.to_owned();
            for (_, repo) in self.repos.iter_mut() {
                repo.set_symbol(symbol)
            }
        }
    }

    fn push(&mut self, repo: Repo) {
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

    pub fn groups(&self) -> &HashMap<String, Group> {
        &self.groups
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn repo_count(&self) -> usize {
        let mut rv = 0;
        for (_, group) in &self.groups {
            rv += group.repos().len();
        }
        rv
    }

    pub fn push(&mut self, path: &str) -> Result<(), Vec<Error>> {
        fn warning(path: &str, e: &StdError, message: &str) -> Vec<Error> {
            vec![Error::new(&path, &format!("{} ({})", message, e))]
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
        let group = self.groups.entry(name.to_owned())
            .or_insert(Group::new(&name, &symbol));

        if let Some(symbol) = ini.get_from(Some("group"), "symbol") {
            group.set_symbol(symbol);
        }
        let symbol = group.symbol().to_owned();

        let mut existing: HashMap<String, String> = HashMap::new();
        for (name, repo) in group.repos().iter() {
            existing.insert(name.to_owned(), repo.config_path.to_owned());
        }

        let mut warnings = Vec::new();
        if let Some(repos) = ini.section(Some("repos")) {
            for (name, p) in repos.iter() {
                match Repository::open(p) {
                    Ok(repo) => {
                        group.push(Repo::new(&path, &symbol, &name, repo));
                        if existing.contains_key(name) {
                            warnings.push(Error::new(&path, &format!(
                                "\"{}\" overrides repo of the same name from \
                                 {}", name, existing.get(name).unwrap())));
                        }
                    },
                    Err(e) => warnings.push(Error::new(&path, &format!(
                        "failed to open repo \"{}\" at {} ({})", name, p, e))),
                }
            }
        }

        if warnings.len() > 0 {
            Err(warnings)
        } else {
            Ok(())
        }
    }
}
