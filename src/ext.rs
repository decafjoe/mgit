//! "Extensions" for mgit. Right now "extensions = flame graphs".
use std::{
    env,
    fs::{create_dir, File},
    io::Write,
    path::Path,
    process,
};

use chrono::Local;
use flame;

use app::resolve_path;

/// TODO(jjoyce): doc
const DEFAULT_FLAME_GRAPHS_PATH: &str = "./flame-graphs/";
/// TODO(jjoyce): doc
const DISABLE_FLAME_GRAPHS_ENVVAR: &str = "MGIT_DISABLE_FLAME_GRAPHS";
/// TODO(jjoyce): doc
const FLAME_GRAPHS_INDEX_FILENAME: &str = "index.html";
/// TODO(jjoyce): doc
const FLAME_GRAPHS_PATH_ENVVAR: &str = "MGIT_FLAME_GRAPHS_PATH";

/// TODO(jjoyce): doc
#[noflame]
pub fn exit(code: i32) {
    let disable = match env::var(DISABLE_FLAME_GRAPHS_ENVVAR) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("note: {} {}", DISABLE_FLAME_GRAPHS_ENVVAR, e);
            eprintln!("note: enabling flame graph output");
            "no".to_owned()
        },
    };
    if disable == "no" {
        let error = |message: &str| {
            eprintln!("error: {}", message);
            process::exit(code);
        };
        let raw_graphs_directory_value = match env::var(FLAME_GRAPHS_PATH_ENVVAR) {
            Ok(value) => {
                eprintln!("note: {} is set", FLAME_GRAPHS_PATH_ENVVAR);
                eprintln!("note: using path from var: {}", value);
                value
            },
            Err(e) => {
                eprintln!("note: {} {}", FLAME_GRAPHS_PATH_ENVVAR, e);
                eprintln!("note: using default path: {}", DEFAULT_FLAME_GRAPHS_PATH);
                DEFAULT_FLAME_GRAPHS_PATH.to_owned()
            },
        };
        let graphs_directory = match resolve_path(&raw_graphs_directory_value, None) {
            Ok(p) => p,
            Err(e) => error(&format!("failed to resolve path: {}", e.message())),
        };
        if let Some(graphs_directory_str) = graphs_directory.to_str() {
            if graphs_directory.is_dir() {
                let timestamp = Local::now().format("%F_%H-%M-%S").to_string();
                eprintln!("note: directory for this run is {}/", timestamp);
                let run_directory = graphs_directory.join(Path::new(&timestamp));
                if let Err(e) = create_dir(&run_directory) {
                    error(&format!("failed to create directory for run: {}", e));
                }
                let mut index_f = match File::create(
                    run_directory.join(Path::new(FLAME_GRAPHS_INDEX_FILENAME)),
                ) {
                    Ok(f) => f,
                    Err(e) => {
                        error(&format!(
                            "failed to open {} for writing: {}",
                            FLAME_GRAPHS_INDEX_FILENAME, e
                        ));
                        return;
                    },
                };
                if let Err(e) = write!(
                    index_f,
                    "<!DOCTYPE html>\n<html>\n  <head></head>\n  <body>"
                ) {
                    error(&format!(
                        "failed to write header to {}: {}",
                        FLAME_GRAPHS_INDEX_FILENAME, e
                    ));
                }
                for thread in flame::threads() {
                    let name = match thread.name {
                        Some(name) => name,
                        None => thread.id.to_string(),
                    };
                    let filename = format!("{}.html", name);
                    if let Err(e) = write!(
                        index_f,
                        "\n    <br><a href=\"{}\">{}</a>",
                        filename, filename
                    ) {
                        error(&format!(
                            "failed to write link to {}: {}",
                            FLAME_GRAPHS_INDEX_FILENAME, e
                        ));
                    }
                    let mut f = match File::create(run_directory.join(Path::new(&filename))) {
                        Ok(f) => f,
                        Err(e) => {
                            error(&format!("failed to open {} for writing: {}", filename, e));
                            return;
                        },
                    };
                    if let Err(e) = flame::dump_html_custom(f, &thread.spans) {
                        error(&format!(
                            "failed to dump flame graph to {}: {}",
                            filename, e
                        ));
                    }
                }
                if let Err(e) = write!(index_f, "\n  </body>\n</html>") {
                    error(&format!(
                        "failed to write footer to {}: {}",
                        FLAME_GRAPHS_INDEX_FILENAME, e
                    ));
                }
            } else {
                error(&format!(
                    "output path is not a directory: {}",
                    graphs_directory_str
                ));
            }
        } else {
            error("failed to convert graph directory path to string");
        }
    }
    process::exit(code);
}
