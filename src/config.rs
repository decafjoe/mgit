//! Configuration parser and API.
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use git2::Repository;
use ini::Ini;

use path;

// ----- Error ----------------------------------------------------------------

/// Error type for this module.
///
/// Note that underlying errors (e.g. permissions errors) are
/// "exposed" by adding their message to the `message` string. This is
/// the responsibility of whoever is constructing the error.
#[derive(Debug)]
pub struct Error {
    /// Message describing the error.
    message: String,
}

impl Error {
    /// Returns a new error instance with the specified `message`.
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

// ----- Repo -----------------------------------------------------------------

/// Symbol to use if symbol is not configured by end user.
const DEFAULT_SYMBOL: &str = "\u{2022}";

/// Configuration for an individual repository.
pub struct Repo {
    /// Path to the configuration file in which repo was defined.
    config_path: String,
    /// Path to the repository itself.
    path: String,
    /// Optional human friendly-name for the repo.
    name: Option<String>,
    /// Optional comment describing the repo.
    comment: Option<String>,
    /// Optional "symbol" for the repo – the character that precedes
    /// the repo name in status listings.
    symbol: Option<String>,
    /// Tags associated with the repo.
    tags: Vec<String>,
    /// libgit2 `Repository` instance for this repo.
    git: Option<Repository>,
}

impl Repo {
    /// Creates and returns a new Repo object.
    pub fn new(
        config_path: &str,
        repo_path: &str,
        name: Option<&str>,
        comment: Option<&str>,
        symbol: Option<&str>,
        tags: Option<&str>,
    ) -> Self {
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
        let tags = match tags {
            Some(s) => {
                let mut tags = Vec::new();
                for tag in s.split_whitespace() {
                    tags.push(tag.to_owned())
                }
                tags
            }
            None => Vec::new(),
        };
        Self {
            config_path: config_path.to_owned(),
            path: repo_path.to_owned(),
            name: name,
            comment: comment,
            symbol: symbol,
            tags: tags,
            git: None,
        }
    }

    /// Returns the expanded, absolute path to the repository.
    pub fn absolute_path(&self) -> Result<PathBuf, Error> {
        let path = match path::expand(&self.path) {
            Ok(path) => {
                if path.is_relative() {
                    let mut p = PathBuf::from(&self.config_path);
                    p.pop();
                    p.push(path);
                    p
                } else {
                    path
                }
            }
            Err(e) => {
                return Err(Error::new(&format!(
                    "failed to expand path '{}' failed ({})",
                    self.path, e
                )))
            }
        };
        match path.canonicalize() {
            Ok(path) => Ok(path),
            Err(e) => Err(Error::new(&format!(
                "failed to canonicalize path '{}' ({})",
                path.to_str().expect("failed to cast path to string"),
                e
            ))),
        }
    }

    /// Returns config path for this repository.
    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    /// Returns path to the repository as specified by the end user.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns name of the repository.
    pub fn name(&self) -> Option<String> {
        self.name.to_owned()
    }

    /// Returns name of repository if set, or the default value (last
    /// component of the repo path) if name is not set.
    pub fn name_or_default(&self) -> String {
        let msg = format!("could not get file_name from '{}'", self.path);
        match self.name() {
            Some(name) => name,
            None => PathBuf::from(&self.path)
                .file_name()
                .expect(&msg)
                .to_str()
                .expect("file_name was not valid unicode")
                .to_owned(),
        }
    }

    /// Returns comment for the repository.
    pub fn comment(&self) -> Option<String> {
        self.comment.to_owned()
    }

    /// Returns symbol for the repository.
    pub fn symbol(&self) -> Option<String> {
        self.symbol.to_owned()
    }

    /// Returns symbol of repository if set, or the default value if
    /// symbol is not set.
    pub fn symbol_or_default(&self) -> String {
        match self.symbol() {
            Some(symbol) => symbol,
            None => DEFAULT_SYMBOL.to_owned(),
        }
    }

    /// Returns tags for this repository.
    pub fn tags(&self) -> &[String] {
        self.tags.as_slice()
    }

    /// Sets the repository instance for the repo config.
    fn set_git(&mut self, git: Repository) {
        self.git = Some(git)
    }

    /// Returns `Repository` instance for this repository.
    pub fn git(&self) -> &Repository {
        match self.git {
            Some(ref repository) => repository,
            None => panic!("repository is not set"),
        }
    }
}

// ----- Config ---------------------------------------------------------------

/// Name config key.
const NAME_KEY: &str = "name";
/// Comment config key.
const COMMENT_KEY: &str = "comment";
/// Symbol config key.
const SYMBOL_KEY: &str = "symbol";
/// Tags config key.
const TAGS_KEY: &str = "tags";

/// Configuration for the program.
///
/// The idea is to `read` configuration files into the config struct,
/// then fetch them out using the (very crude) API.
///
/// Repositories are "keyed" by the path specified by the user in the
/// config file (i.e. by the thing inside the brackets). The "key" for
/// `[~/mgit]` is `~/mgit`.
///
/// An individual repo can be fetched with the `repo` method, using
/// the key.
///
/// "Lists" of repos can be fetched using `repos_iter` or
/// `repos_tagged`.
pub struct Config {
    /// `HashMap` mapping repo `path` (a `String`) to the successfully
    /// initialized `Repo` configuration.
    repos: HashMap<String, Repo>,
}

impl Config {
    /// Creates and returns a new configuration object.
    pub fn new() -> Self {
        Self {
            repos: HashMap::new(),
        }
    }

    /// Reads the file at `path`, returning a vec of errors if there
    /// are any issues.
    ///
    /// Note that, except for "can't read the file" type errors,
    /// processing will continue if an error is found. In terms of the
    /// larger program, errors here are more like warnings: something
    /// is awry and the user should be notified, but processing may
    /// continue.
    ///
    /// # Errors
    ///
    /// This call can error out for a number of reasons:
    ///
    /// * `path` does not exist or is not a file.
    /// * File at `path` cannot be opened.
    /// * File at `path` cannot be read into a string.
    /// * File at `path` cannot be parsed.
    /// * Configuration contains repositories that have already been
    ///   defined.
    /// * Configuration contains repositories that cannot be opened by
    ///   the libgit2 library.
    pub fn read(&mut self, path: &str) -> Result<(), Vec<Error>> {
        /// Returns a single-item vec containing an `Error` with the
        /// specified `msg`.
        fn err(msg: &str) -> Result<(), Vec<Error>> {
            Err(vec![Error::new(msg)])
        }

        /// Returns a single-item vec containing an `Error`. The
        /// message is a combination of `msg` and the value of the
        /// error `e`.
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

        let mut absolute_paths = HashMap::new();
        for (_, repo) in self.repos() {
            absolute_paths.insert(
                repo.absolute_path().expect("could not get repo path"),
                repo.config_path().to_owned(),
            );
        }

        let mut errors = Vec::new();
        for (repo_path, _) in &ini {
            if let Some(repo_path) = repo_path.as_ref() {
                let mut repo = Repo::new(
                    path,
                    repo_path,
                    ini.get_from(Some(repo_path.to_string()), NAME_KEY),
                    ini.get_from(Some(repo_path.to_string()), COMMENT_KEY),
                    ini.get_from(Some(repo_path.to_string()), SYMBOL_KEY),
                    ini.get_from(Some(repo_path.to_string()), TAGS_KEY),
                );
                let absolute_path = match repo.absolute_path() {
                    Ok(path) => path,
                    Err(e) => {
                        errors.push(Error::new(&format!(
                            "failed to get absolute path for '{}' ({})",
                            repo_path, e
                        )));
                        continue
                    }
                };
                let absolute_path_str = absolute_path
                    .to_str()
                    .expect("could not cast absolute path to string");
                if let Some(config_path) = absolute_paths.get(&absolute_path) {
                    errors.push(Error::new(&format!(
                        "repo at '{}' already configured in config file '{}' \
                         (ignoring this definition)",
                        absolute_path_str, config_path
                    )));
                    continue
                }
                match Repository::open(&absolute_path) {
                    Ok(repository) => {
                        repo.set_git(repository);
                        self.repos.insert(repo_path.to_owned(), repo);
                    }
                    Err(e) => {
                        errors.push(Error::new(&format!(
                            "failed to open repository at '{}' ({})",
                            absolute_path_str, e
                        )));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Sorts a `Vec` of paths on name (primary) / path (secondary).
    fn paths_sort(&self, rv: &mut Vec<&str>) {
        rv.sort_by_key(|path| {
            (
                self.repo(*path)
                    .expect(&format!("failed to get repo for path {}", path))
                    .name_or_default(),
                *path,
            )
        })
    }

    /// Returns `Vec` of all paths in the config, sorted by repository
    /// name.
    ///
    /// The secondary sort parameter is the path itself, to ensure
    /// that return values from this function are deterministic.
    pub fn paths<'a>(&'a self) -> Vec<&'a str> {
        let mut rv = Vec::new();
        for (path, _) in self.repos() {
            rv.push(path.as_str())
        }
        self.paths_sort(&mut rv);
        rv
    }

    /// Returns `Vec` of paths whose repos have the tag `tag`, sorted
    /// by repository name.
    ///
    /// The secondary sort parameter is the path itself, to ensure
    /// that return values from this function are deterministic.
    pub fn paths_for_tag<'a>(&'a self, tag: &str) -> Vec<&'a str> {
        let mut rv = Vec::new();
        for (path, repo) in self.repos() {
            if repo.tags().contains(&tag.to_owned()) {
                rv.push(path.as_str())
            }
        }
        self.paths_sort(&mut rv);
        rv
    }

    /// Returns reference to repo at `path`.
    pub fn repo(&self, path: &str) -> Option<&Repo> {
        self.repos.get(path)
    }

    /// Returns reference to the repos `HashMap`.
    pub fn repos(&self) -> &HashMap<String, Repo> {
        &self.repos
    }
}
