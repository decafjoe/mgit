//! Top-level application code, state management, and program control.
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::process;

use ansi_term::{Color, Style};
use clap::{App, Arg, ArgMatches};
use git2::Repository;
use ini::Ini;
use pager::Pager;
use users;
use users::os::unix::UserExt;
use walkdir::WalkDir;

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

    if config.repos().len() == 0 {
        control.fatal("no repositories configured");
    }

    // Return and transfer ownership of control and config instances.
    (control, config)
}

// ----- ResolveError ---------------------------------------------------------

/// Represents an error during resolution of a user-supplied path.
struct ResolveError {
    /// Message describing the resolution error.
    message: String,
}

impl ResolveError {
    /// Creates and returns a new `ResolveError` instance.
    fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }

    /// Returns the message describing the error.
    fn message(&self) -> &str {
        self.message.as_str()
    }
}

// ----- resolve_path ---------------------------------------------------------

/// Resolves the given `path`.
///
/// If the path starts with `~`, this tries to resolve it to a user
/// home directory (or a subdirectory thereof).
///
/// If the path starts with the system `MAIN_SEPARATOR`, it's assumed
/// to be absolute and is left unchanged.
///
/// Otherwise, the path is assumed to be relative. If `relative_to`
/// does not have a value (i.e. is `None`) then the current working
/// directory is used for `relative_to`.
///
/// Once the path has been resolved per the above, it is canonicalized
/// using `std::fs::canonicalize` and finally returned.
fn resolve_path(
    path: &str,
    relative_to: Option<&str>,
) -> Result<PathBuf, ResolveError> {
    let mut relative_to = match relative_to {
        Some(path) => {
            // Caller passed relative_to. If a directory, return
            // as-is. Otherwise, figure out the directory containing
            // the path and return that.
            let buf = PathBuf::from(path);
            if buf.is_dir() {
                buf
            } else {
                match buf.parent() {
                    Some(path) => path.to_path_buf(),
                    None => {
                        return Err(ResolveError::new(&format!(
                            "could not get parent of relative_to ({})",
                            path
                        )))
                    }
                }
            }
        }
        None => match env::current_dir() {
            Ok(buf) => buf,
            Err(e) => {
                return Err(ResolveError::new(&format!(
                    "could not get cwd ({})",
                    e
                )))
            }
        },
    };
    let path = if path.starts_with('~') {
        // Check for `~` or `~/...` -- i.e. a bare tilde, meaning the
        // current user.
        if path.len() == 1
            || path.chars().nth(1).expect("could not get second char")
                == MAIN_SEPARATOR
        {
            let uid = users::get_current_uid();
            if let Some(user) = users::get_user_by_uid(uid) {
                let mut buf = user.home_dir().to_path_buf();
                if path.len() > 2 {
                    buf.push(&path[2..]);
                }
                buf
            } else {
                return Err(ResolveError::new(&format!(
                    "failed to look up user info for uid {}",
                    uid
                )));
            }
        } else {
            // Fully specified user (e.g. `~foo/...`) -- extract
            // username and look up home directory.
            let name =
                path[1..].split(MAIN_SEPARATOR).nth(0).expect(&format!(
                    "splitting '{}' on MAIN_SEPARATOR ('{}') failed",
                    path, MAIN_SEPARATOR
                ));
            if let Some(user) = users::get_user_by_name(name) {
                let mut buf = user.home_dir().to_path_buf();
                if path.len() > name.len() + 1 {
                    buf.push(&path[(name.len() + 2)..]);
                }
                buf
            } else {
                return Err(ResolveError::new(&format!(
                    "failed to look up user info for username '{}'",
                    name
                )));
            }
        }
    } else if path.starts_with(MAIN_SEPARATOR) {
        PathBuf::from(path)
    } else {
        relative_to.push(path);
        relative_to
    };
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(e) => Err(ResolveError::new(&format!(
            "failed to canonicalize path ({})",
            e
        ))),
    }
}

// ----- Error ----------------------------------------------------------------

/// Represents an error encountered when reading configuration.
struct Error {
    /// Configuration path associated with the error.
    config_path: String,
    /// Path of the repository, if relevant for this error.
    repo_path: Option<String>,
    /// Message describing the error.
    message: String,
    /// Optional message indicating the underlying cause of the error.
    cause: Option<String>,
}

impl Error {
    /// Creates and returns a new `Error` instance.
    fn new(
        config_path: &str,
        repo_path: Option<&str>,
        message: &str,
        cause: Option<&str>,
    ) -> Self {
        Self {
            config_path: config_path.to_owned(),
            repo_path: if let Some(repo_path) = repo_path {
                Some(repo_path.to_owned())
            } else {
                None
            },
            message: message.to_owned(),
            cause: if let Some(cause) = cause {
                Some(cause.to_owned())
            } else {
                None
            },
        }
    }

    /// Returns the underlying cause of the error.
    fn cause(&self) -> Option<&str> {
        if let Some(ref cause) = self.cause {
            Some(cause.as_str())
        } else {
            None
        }
    }

    /// Returns the configuration path associated with the error.
    fn config_path(&self) -> &str {
        self.config_path.as_str()
    }

    /// Returns the message describing the error.
    fn message(&self) -> &str {
        self.message.as_str()
    }

    /// Returns the path of the associated repository, if relevant for
    /// this error.
    fn repo_path(&self) -> Option<&str> {
        if let Some(ref repo_path) = self.repo_path {
            Some(repo_path.as_str())
        } else {
            None
        }
    }
}

// ----- Repo -----------------------------------------------------------------

/// Symbol to use if not configured by end user.
const DEFAULT_SYMBOL: &str = "\u{2022}";

/// Configuration for an individual repository.
pub struct Repo {
    /// Path to the configuration file in which the repo was defined.
    config_path: String,
    /// Path to the repository itself, as specified by the user.
    path: String,
    /// Fully-resolved, absolute path to the repository.
    full_path: String,
    /// Optional human-friendly name for the repo.
    name: Option<String>,
    /// Optional "symbol" for the repo – the character that precedes
    /// the repo name in status listings.
    symbol: Option<String>,
    /// Optional tags associated with the repo.
    tags: Vec<String>,
}

impl Repo {
    /// Creates and returns a new `Repo` instance.
    fn new(
        config_path: &str,
        path: &str,
        full_path: &str,
        name: Option<&str>,
        symbol: Option<&str>,
        tags: &[&str],
    ) -> Self {
        Self {
            config_path: config_path.to_owned(),
            path: path.to_owned(),
            full_path: full_path.to_owned(),
            name: match name {
                Some(name) => Some(name.to_owned()),
                None => None,
            },
            symbol: match symbol {
                Some(symbol) => Some(symbol.to_owned()),
                None => None,
            },
            tags: tags.iter().map(|&s| s.to_owned()).collect(),
        }
    }

    /// Returns path of configuration file in which this repo was
    /// defined.
    pub fn config_path(&self) -> &str {
        self.config_path.as_str()
    }

    /// Returns the path to the repo, as specified by the end user.
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Returns the full path to the repo.
    pub fn full_path(&self) -> &str {
        self.full_path.as_str()
    }

    /// Returns the (optionally-set) name of the repository.
    pub fn name(&self) -> Option<&str> {
        match self.name {
            Some(ref name) => Some(name.as_str()),
            None => None,
        }
    }

    /// Returns the (optionally-set) symbol the repository.
    pub fn symbol(&self) -> Option<&str> {
        match self.symbol {
            Some(ref symbol) => Some(symbol.as_str()),
            None => None,
        }
    }

    /// Returns tags associated with this repository.
    pub fn tags(&self) -> Vec<&str> {
        self.tags
            .iter()
            .map(|s: &String| s.as_str())
            .collect::<Vec<&str>>()
    }

    /// Returns `name` if set, otherwise the default value as computed
    /// from the `path`.
    pub fn name_or_default(&self) -> &str {
        if let Some(ref name) = self.name {
            name.as_str()
        } else if self.path == format!("{}", MAIN_SEPARATOR) {
            "<root>"
        } else {
            Path::new(&self.path)
                .file_name()
                .expect("failed to get file_name from repo path")
                .to_str()
                .expect("failed to convert file_name to str")
        }
    }

    /// Returns `symbol` if set, otherwise the value of
    /// `DEFAULT_SYMBOL`.
    pub fn symbol_or_default(&self) -> &str {
        if let Some(ref symbol) = self.symbol {
            symbol.as_str()
        } else {
            DEFAULT_SYMBOL
        }
    }
}

// ----- Field ----------------------------------------------------------------

/// `Repo` field, used to specify what to sort by or iterate through
/// with `Iter`.
pub enum Field {
    /// `Repo.path()`
    Path,
    /// `Repo.name_or_default()`
    Name,
}

// ----- Iter -----------------------------------------------------------------

/// Iterator over a sorted vec of `Repo` instances.
///
/// `Iter` is meant to be "set up" using chained method calls,
/// transferring ownership through each call. The `Iter` instance
/// itself is obtained from `Config.repos()`.
///
/// ```rust,ignore
/// for (path, repo) in config
///     .repos()
///     .iter_field(Field::Path)
///     .iter_field(Field::Name) // this and the next line are completely
///     .iter_field(Field::Path) // superfluous of course, but not an error
///     .sorted_by(Field::Path)
///     .sorted_by(Field::Name) // same as above
///     .sorted_by(Field::Path)
/// {
///     // do stuff with path and repo
/// }
/// ```
///
/// The underlying vector is not actually sorted until the first item
/// is consumed.
pub struct Iter<'a> {
    /// `Vec` of `Repo` references to iterate through. Items are
    /// popped off the front of this vec as the iterator is consumed.
    repos: Vec<&'a Repo>,
    /// `Field` to yield as the "key."
    iter_field: Field,
    /// `Field` by which to sort (always ascending).
    sort_field: Field,
    /// Indicates whether `repos` is sorted.
    sorted: bool,
}

impl<'a> Iter<'a> {
    /// Creates and returns a new `Iter` for `repos`.
    fn new(repos: Vec<&'a Repo>) -> Self {
        Self {
            repos: repos,
            iter_field: Field::Name,
            sort_field: Field::Name,
            sorted: false,
        }
    }

    /// Sets `Field` that will be yielded as the iterator "key."
    pub fn iter_field(self, field: Field) -> Self {
        Self {
            repos: self.repos,
            iter_field: field,
            sort_field: self.sort_field,
            sorted: self.sorted,
        }
    }

    /// Sets `Field` that controls sort order.
    pub fn sorted_by(self, field: Field) -> Self {
        Self {
            repos: self.repos,
            iter_field: self.iter_field,
            sort_field: field,
            sorted: self.sorted,
        }
    }

    /// Limits iteration to `Repo` instances with the tag `tag`.
    fn tagged(self, tag: &str) -> Self {
        let mut repos = Vec::new();
        for repo in self.repos {
            if repo.tags().contains(&tag) {
                repos.push(repo);
            }
        }
        Self {
            repos: repos,
            iter_field: self.iter_field,
            sort_field: self.sort_field,
            sorted: self.sorted,
        }
    }

    /// Returns the number of repos in the `Iter`.
    fn len(&self) -> usize {
        self.repos.len()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, &'a Repo);

    /// Returns the next item in the iterator.
    ///
    /// If this is the first item, the underlying vector is sorted.
    /// The "key" yielded depends on the value of `iter_field`. The
    /// value is always a reference to a `Repo`.
    fn next(&mut self) -> Option<Self::Item> {
        if !self.sorted {
            let field = &self.sort_field;
            self.repos.sort_by_key(|r| match *field {
                Field::Name => (r.name_or_default(), r.path()),
                Field::Path => (r.path(), ""),
            });
            self.sorted = true;
        }
        if self.repos.is_empty() {
            None
        } else {
            let repo = self.repos.remove(0);
            let key = match self.iter_field {
                Field::Name => repo.name_or_default(),
                Field::Path => repo.path(),
            };
            Some((key, repo))
        }
    }
}

// ----- Config ---------------------------------------------------------------

/// Configuration key that specifies repo name.
const NAME_KEY: &str = "name";
/// Configuration key that specifies repo symbol.
const SYMBOL_KEY: &str = "symbol";
/// Configuration key that specifies repo tags.
const TAGS_KEY: &str = "tags";

/// Configuration as specified by the end user.
pub struct Config {
    /// `Vec` of `Repo` instances defined in the configuration.
    repos: Vec<Repo>,
}

impl Config {
    /// Creates and returns a new, empty `Config` instance.
    fn new() -> Self {
        Self { repos: Vec::new() }
    }

    /// Returns an `Iter` instance over the repos in the
    /// configuration.
    fn repos(&self) -> Iter {
        Iter::new(self.repos.iter().collect::<Vec<&Repo>>())
    }

    /// Reads configuration at `path`, returning a list of errors
    /// encountered.
    ///
    /// If `path` is a directory, it is recursively walked and any
    /// files with the extension `.conf` are read into the
    /// configuration.
    ///
    /// # Notes
    ///
    /// This is the method that contains all the up-front validation
    /// referenced in the module-level docs. It's very picky.
    ///
    /// If there are any errors reading `path` or its children (e.g.
    /// `path` does not exist, permissions issues) those are returned.
    ///
    /// This will also return errors with the configuration itself
    /// (e.g. a file defines a repository that has already been
    /// configured, repository path does not exist or is not a git
    /// repo).
    fn read(&mut self, path: &str) -> Vec<Error> {
        let path_str = path;
        let path = match resolve_path(path, None) {
            Ok(buf) => buf,
            Err(e) => {
                return vec![
                    Error::new(
                        path_str,
                        None,
                        "failed to resolve config path",
                        Some(e.message()),
                    ),
                ]
            }
        };

        let mut rv = Vec::new();
        let mut paths = Vec::new();
        if path.is_file() {
            paths.push(path);
        } else if path.is_dir() {
            for entry in WalkDir::new(&path) {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        rv.push(Error::new(
                            path_str,
                            None,
                            "failure when walking directory",
                            Some(&format!("{}", e)),
                        ));
                        continue;
                    }
                };
                if entry.path().is_file() {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "conf" {
                            paths.push(entry.path().to_path_buf());
                        }
                    }
                }
            }
        } else {
            rv.push(Error::new(
                path_str,
                None,
                "path is not a file or directory",
                None,
            ));
        }

        let mut full_paths = HashMap::new();
        for repo in &self.repos {
            full_paths.insert(
                repo.full_path().to_owned(),
                repo.config_path().to_owned(),
            );
        }

        for path in paths {
            let path_str = if let Some(s) = path.to_str() {
                s
            } else {
                rv.push(Error::new(
                    path_str,
                    None,
                    "subpath contains invalid unicode",
                    None,
                ));
                continue;
            };
            let mut f = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    rv.push(Error::new(
                        path_str,
                        None,
                        "failed to open file",
                        Some(&format!("{}", e)),
                    ));
                    continue;
                }
            };
            let mut s = String::new();
            if let Err(e) = f.read_to_string(&mut s) {
                rv.push(Error::new(
                    path_str,
                    None,
                    "failed to read file",
                    Some(&format!("{}", e)),
                ));
                continue;
            }
            let ini = match Ini::load_from_str(&s) {
                Ok(ini) => ini,
                Err(e) => {
                    rv.push(Error::new(
                        path_str,
                        None,
                        "failed to parse file",
                        Some(&format!("{}", e)),
                    ));
                    continue;
                }
            };
            for (section, settings) in &ini {
                let repo_path = if let Some(ref path) = *section {
                    path
                } else {
                    continue;
                };
                let full_path = match resolve_path(repo_path, Some(path_str)) {
                    Ok(path) => path,
                    Err(e) => {
                        rv.push(Error::new(
                            path_str,
                            Some(repo_path),
                            "failed to resolve repo path",
                            Some(e.message()),
                        ));
                        continue;
                    }
                };
                let full_path_str = if let Some(s) = full_path.to_str() {
                    s
                } else {
                    rv.push(Error::new(
                        path_str,
                        Some(repo_path),
                        "absolute path contains invalid unicode",
                        None,
                    ));
                    continue;
                };
                if let Some(config_path) = full_paths.get(full_path_str) {
                    rv.push(Error::new(
                        path_str,
                        Some(repo_path),
                        "repo is already configured (ignoring new definition)",
                        Some(&format!("first configured in {}", config_path)),
                    ));
                    continue;
                }
                if let Err(e) = Repository::open(&full_path) {
                    rv.push(Error::new(
                        path_str,
                        Some(repo_path),
                        "failed to open repository",
                        Some(e.message()),
                    ));
                    continue;
                }
                let tags = match settings.get(TAGS_KEY) {
                    Some(s) => s.split_whitespace().collect::<Vec<&str>>(),
                    None => vec![],
                };
                let repo = Repo::new(
                    path_str,
                    repo_path,
                    full_path_str,
                    match settings.get(NAME_KEY) {
                        Some(s) => Some(s.as_str()),
                        None => None,
                    },
                    match settings.get(SYMBOL_KEY) {
                        Some(s) => Some(s.as_str()),
                        None => None,
                    },
                    tags.as_slice(),
                );
                full_paths
                    .insert(full_path_str.to_owned(), path_str.to_owned());
                self.repos.push(repo);
            }
        }

        rv
    }
}

// ----- Action ---------------------------------------------------------------

/// Represents an action to take in response to an error condition.
#[derive(PartialEq)]
enum Action {
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
    fn new(warning_action: Action) -> Self {
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

// ----- TagIter --------------------------------------------------------------

/// Weird, kind of hacky iterator to support a common UI pattern.
///
/// Let me explain. All the subcommands take (zero or more) `-t/--tag`
/// arguments. This struct works in conjunction with
/// `Invocation.iter_tags()` to let calling code handle those
/// arguments without doing a bunch of legwork.
///
/// ```rust,ignore
/// for (tag, repos) in invocation.iter_tags(TAG_ARG) {
///     // see notes below
/// }
/// ```
///
/// If `tag` is `None`:
///
/// * The user supplied no `-t` arguments
/// * `repos` is an `Iter` over all the configured repos
/// * There will be exactly one item (this one) yielded from the
///   `TagIter`
///
/// Otherwise, `tag` will be `Some(&str)`, meaning:
///
/// * The user supplied one or more `-t` arguments
/// * `repos` is an `Iter` over the repos with tag `tag`
/// * There will be one or more items yielded from the `TagIter` (note
///   that the same `Repo` may be yielded multiple times if it matches
///   multiple `-t` arguments)
pub struct TagIter<'a> {
    /// Reference to the `Config` to query.
    config: &'a Config,
    /// Optional reference to a `Vec` of tags through which to
    /// iterate. `None` indicates no tag arguments were provided by
    /// the end user.
    tags: Option<Vec<&'a str>>,
    /// Indices into `tags` for the remaining tags. Values are popped
    /// from the front of this vec as the iterator is consumed. When
    /// `tags` is `None`, this is a single-item vec whose value
    /// doesn't matter (see docs for `next()`).
    indices: Vec<usize>,
}

impl<'a> TagIter<'a> {
    /// Creates and returns a new `TagIter` instance.
    fn new(config: &'a Config, tags: Option<Vec<&'a str>>) -> Self {
        let indices = match tags {
            None => vec![0],
            Some(ref tags) => (0..tags.len()).collect(),
        };
        Self {
            config: config,
            tags: tags,
            indices: indices,
        }
    }
}

impl<'a> Iterator for TagIter<'a> {
    type Item = (Option<&'a str>, Iter<'a>);

    /// Pops the first index off the front of `indices`, then uses
    /// that to determine what to yield.
    ///
    /// When `tags` is a vec, this yields the tag at the index and an
    /// `Iter` over the repos with the specified tag.
    ///
    /// When `tags` is `None`, this discards the index value and
    /// yields `None` for the tag and an `Iter` over all configured
    /// repos. (The next time `next()` is called, `indices` is empty
    /// and [just] `None` is yielded, stopping iteration after the
    /// first pass.)
    fn next(&mut self) -> Option<Self::Item> {
        if self.indices.is_empty() {
            return None;
        }
        let index = self.indices.remove(0);
        if let Some(ref tags) = self.tags {
            let tag = tags[index];
            Some((Some(tag), self.config.repos().tagged(tag)))
        } else {
            Some((None, self.config.repos()))
        }
    }
}

// ----- Invocation -----------------------------------------------------------

/// Pager command and arguments. Tries to act like a number of git
/// porcelain commands, like `git diff`.
const PAGER: &str = "less -efFnrX";

/// All state for a given invocation of the program.
pub struct Invocation<'a> {
    /// `Config` instance.
    config: &'a Config,
    /// `Control` instance.
    control: &'a Control,
    /// `ArgMatches` instance, for the subcommand arguments.
    matches: &'a ArgMatches<'a>,
}

impl<'a> Invocation<'a> {
    /// Creates and returns a new invocation instance.
    pub fn new(
        control: &'a Control,
        config: &'a Config,
        matches: &'a ArgMatches,
    ) -> Self {
        Self {
            config: config,
            control: control,
            matches: matches,
        }
    }

    /// Returns the control struct for this invocation.
    pub fn control(&self) -> &Control {
        self.control
    }

    /// Returns the matches struct for this invocation.
    pub fn matches(&self) -> &ArgMatches {
        self.matches
    }

    /// Returns a `TagIter` based on the end-user arguments supplied
    /// in the argument named `arg`.
    ///
    /// See the documentation for `TagIter` for a full explanation.
    pub fn iter_tags(&self, arg: &str) -> TagIter {
        let tags = match self.matches().values_of(arg) {
            Some(tags) => Some(tags.collect()),
            None => None,
        };
        TagIter::new(self.config, tags)
    }

    /// Starts the pager with mgit's "standard" arguments.
    pub fn start_pager(&self) {
        Pager::with_pager(PAGER).setup();
    }
}
