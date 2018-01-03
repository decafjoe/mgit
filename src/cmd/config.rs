//! Prints the configuration.
use ansi_term::Style;
use ansi_term::Color::{Blue, Purple};
use clap::{App, Arg, SubCommand};
use ordermap::OrderMap;

use invocation::Invocation;

/// Name of the command (`config`).
pub const NAME: &str = "config";

/// Name of the argument for tags.
const TAG_ARG: &str = "TAG";
/// Name of the argument for verbose output.
const VERBOSE_ARG: &str = "VERBOSE";

/// Returns configured subcommand instance for this command.
pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints configuration values")
        .arg(
            Arg::with_name(TAG_ARG)
                .help("Limits display to repos with specified tag(s)")
                .short("t")
                .long("tag")
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name(VERBOSE_ARG)
                .help("Shows defaults in addition to user-specified config")
                .short("v")
                .long("verbose"),
        )
}

/// Prints the configuration per the arguments specified by the user.
#[cfg_attr(feature = "cargo-clippy", allow(print_stdout))]
pub fn run(invocation: &Invocation) {
    if let Some(tags) = invocation.matches().values_of(TAG_ARG) {
        let style = Style::new().bold().underline();
        for tag in tags {
            println!("\n{}{}", style.paint("TAG:"), style.paint(tag));
            print_repos_config(
                invocation,
                &mut invocation.config().paths_for_tag(tag),
            )
        }
    } else {
        println!();
        print_repos_config(invocation, &mut invocation.config().paths())
    }
    println!()
}

/// Prints configuration for repos at `paths`.
#[cfg_attr(feature = "cargo-clippy", allow(print_stdout))]
fn print_repos_config(invocation: &Invocation, paths: &mut Vec<&str>) {
    let verbose = invocation.matches().is_present(VERBOSE_ARG);

    // By default paths are sorted by name. For config, we want them
    // sorted by the path (since that is the "header" we're printing).
    paths.sort();

    for path in paths {
        let repo = invocation
            .config()
            .repo(path)
            .expect(&format!("could not get repo for path {}", path));

        // Buffer information into a hashmap (which iterates in
        // insertion order). We need to buffer since we want to draw ┖
        // on the last line instead of ┠, and we don't know what the
        // last line is until we look at all the settings (taking
        // `verbose` into consideration).
        let mut facts = OrderMap::new();

        facts.insert("config", repo.config_path().to_owned());

        // The unwraps are ok because we do extensive checks when
        // processing the configuration, including checking that the
        // path can be resolved, exists, and can be turned into a
        // string.
        let path = repo.absolute_path()
            .expect("failed to get path for repo")
            .to_str()
            .expect("failed to cast path to string")
            .to_owned();
        facts.insert("path", path);

        match repo.name() {
            Some(name) => {
                facts.insert("name", name);
            }
            None => if verbose {
                facts.insert(
                    "name",
                    format!("{} (default)", repo.name_or_default()),
                );
            },
        }

        match repo.comment() {
            Some(comment) => {
                facts.insert("comment", comment);
            }
            None => if verbose {
                facts.insert("comment", "<not set>".to_owned());
            },
        }

        match repo.symbol() {
            Some(symbol) => {
                facts.insert("symbol", symbol);
            }
            None => if verbose {
                let symbol = format!("{} (default)", repo.symbol_or_default());
                facts.insert("symbol", symbol);
            },
        }

        let tags = repo.tags();
        if !tags.is_empty() {
            let mut s = String::new();
            for (i, tag) in tags.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ")
                }
                s.push_str(tag)
            }
            facts.insert("tags", s);
        } else if verbose {
            facts.insert("tags", "<none set>".to_owned());
        }

        println!("{}", Purple.bold().paint(repo.path()));
        for (i, (key, value)) in facts.iter().enumerate() {
            let mut line = String::from("\u{2500}");
            for _ in 0..7 - key.len() {
                line.push_str("\u{2500}");
            }
            line.push_str(" ");
            let left = if i == (facts.len() - 1) {
                "\u{2516}"
            } else {
                "\u{2520}"
            };
            println!(
                "{}{}",
                Blue.paint(format!("  {}{}{}: ", left, line, key)),
                value
            )
        }
    }
}
