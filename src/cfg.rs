//! Facilities for reading and querying configuration.
//!
//! This code tries to do as much up-front validation of the
//! configuration as it can. That way:
//!
//! 1. We can notify the calling code (which can notify the end-user)
//!    of potential issues as early as possible.
//! 2. We can provide an API to subcommands that isn't full of
//!    `Option` and `Result` and mandatory unwrapping.

// ----- Error ----------------------------------------------------------------

/// Represents an error encountered when reading configuration.
pub struct Error {
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
    pub fn new(
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
    pub fn cause(&self) -> Option<&str> {
        if let Some(ref cause) = self.cause {
            Some(cause.as_str())
        } else {
            None
        }
    }

    /// Returns the configuration path associated with the error.
    pub fn config_path(&self) -> &str {
        self.config_path.as_str()
    }

    /// Returns the message describing the error.
    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    /// Returns the path of the associated repository, if relevant for
    /// this error.
    pub fn repo_path(&self) -> Option<&str> {
        if let Some(ref repo_path) = self.repo_path {
            Some(repo_path.as_str())
        } else {
            None
        }
    }
}

// ----- Config ---------------------------------------------------------------

/// Configuration as specified by the end user.
pub struct Config;

impl Config {
    /// Creates and returns a new, empty `Config` instance.
    pub fn new() -> Self {
        Self {}
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
    pub fn read(&mut self, path: &str) -> Vec<Error> {
        // TODO(jjoyce): kill this and replace with a real impl
        let mut rv = Vec::new();
        rv.push(Error::new(
            path,
            None,
            "something went wrong before parsing",
            None,
        ));
        rv.push(Error::new(
            path,
            None,
            "another pre-parsing warning",
            Some("this is the reason for that one"),
        ));
        rv.push(Error::new(
            path,
            Some("~/mgit"),
            "and here we have a problem inside the config",
            None,
        ));
        rv.push(Error::new(
            path,
            Some("~/mgit"),
            "this is a fully specified error, with all the things",
            Some("underlying cause for the failure inside the config"),
        ));
        rv
    }
}
