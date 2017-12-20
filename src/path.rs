use std::path::{MAIN_SEPARATOR, PathBuf};

use users::{get_current_uid, get_user_by_name, get_user_by_uid};
use users::os::unix::UserExt;

#[derive(Debug, PartialEq)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: &str) -> Error {
        Error{ message: message.to_owned() }
    }
}

pub fn expand(path: &str) -> Result<PathBuf, Error> {
    let sep = MAIN_SEPARATOR;
    if path.starts_with("~") {
        if path.len() == 1 || path.chars().nth(1).unwrap() == sep {
            let uid = get_current_uid();
            if let Some(user) = get_user_by_uid(uid) {
                let mut buf = user.home_dir().to_path_buf();
                if path.len() > 2 {
                    buf.push(&path[2..]);
                }
                Ok(buf)
            } else {
                return Err(Error::new(&format!(
                    "failed to look up user info for uid {}", uid)))
            }
        } else {
            let name = path[1..].split(sep).nth(0).expect(&format!(
                "splitting '{}' on MAIN_SEPARATOR ('{}') failed", path, sep));
            if let Some(user) = get_user_by_name(name) {
                let mut buf = user.home_dir().to_path_buf();
                if path.len() > name.len() + 1 {
                    buf.push(&path[(name.len() + 2)..]);
                }
                Ok(buf)
            } else {
                return Err(Error::new(&format!(
                    "failed to look up user info for username {}", name)))
            }
        }
    } else {
        Ok(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn assert_expanded(expected: &str, input: &str) {
        assert_eq!(Ok(PathBuf::from(expected)), expand(input));
    }

    #[test]
    fn no_tilde() {
        assert_expanded("/foo/bar/baz", "/foo/bar/baz");
    }

    #[test]
    fn user_bare() {
        let path = env::home_dir().expect("could not determine home dir");
        assert_eq!(Ok(PathBuf::from(path)), expand("~"));
    }

    #[test]
    fn user_path() {
        let mut path = env::home_dir().expect("could not determine home dir");
        path.push("foo/bar/baz");
        assert_eq!(Ok(PathBuf::from(path)), expand("~/foo/bar/baz"));
    }

    #[test]
    fn invalid_user() {
        let message = "failed to look up user info for username foobarbaz";
        assert_eq!(Err(Error::new(message)), expand("~foobarbaz"));
    }

    #[cfg(target_os = "macos")]
    mod macos {
        use super::*;

        #[test]
        fn root_bare() {
            assert_expanded("/var/root", "~root");
        }

        #[test]
        fn root_with_path() {
            assert_expanded("/var/root/foo/bar/baz", "~root/foo/bar/baz");
        }
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::*;

        #[test]
        fn root_bare() {
            assert_expanded("/root", "~root");
        }

        #[test]
        fn root_with_path() {
            assert_expanded("/root/foo/bar/baz", "~root/foo/bar/baz");
        }
    }
}
