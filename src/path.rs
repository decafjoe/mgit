use std::path::{MAIN_SEPARATOR, PathBuf};

use users::{get_current_uid, get_user_by_name, get_user_by_uid};
use users::os::unix::UserExt;

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
                if path.len() > 1 {
                    buf.push(&path[1..]);
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
                buf.push(&path[(name.len() + 1)..]);
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
