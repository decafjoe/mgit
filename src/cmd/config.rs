//! `config` subcommand.
use ansi_term::{Color, Style};
use clap::{App, Arg, SubCommand};
use indexmap::IndexMap;

use app::{Field, Invocation};

/// Name of the command (`config`).
pub const NAME: &str = "config";

/// Name of the argument for `-t/--tag`.
const TAG_ARG: &str = "TAG";
/// Name of the argument for `-v/--verbose`.
const VERBOSE_ARG: &str = "VERBOSE";

/// Returns configured clap subcommand for `config`.
pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints configuration as interpreted by mgit")
        .arg(
            Arg::with_name(TAG_ARG)
                .help("Limits/groups display to repos with specified tag(s)")
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

/// Executes the `config` subcommand.
#[cfg_attr(feature = "cargo-clippy", allow(print_stdout))]
pub fn run(invocation: &Invocation) {
    invocation.start_pager();
    let verbose = invocation.matches().is_present(VERBOSE_ARG);
    let header = Style::new().bold().underline();
    for (tag, repos) in invocation.iter_tags(TAG_ARG) {
        if let Some(tag) = tag {
            println!("\n{}{}", header.paint("TAG:"), header.paint(tag));
        } else {
            println!();
        }
        for (path, repo) in
            repos.iter_field(Field::Path).sorted_by(Field::Path)
        {
            // Compute and take references to certain values. We do
            // this before creating the `info` map below so that
            // things are deallocated in the correct order.
            let name_default =
                &format!("{} (default)", repo.name_or_default());
            let symbol_default =
                &format!("{} (default)", repo.symbol_or_default());

            let tags_vec = repo.tags();
            let tags = if tags_vec.is_empty() {
                String::from("<none set>")
            } else {
                let mut s = String::new();
                for (i, tag) in tags_vec.iter().enumerate() {
                    if i != 0 {
                        s.push_str(", ");
                    }
                    s.push_str(tag);
                }
                s
            };

            // Buffer information into a hashmap that iterates in
            // insertion order. We need to buffer since we want to
            // draw ┖ on the last line instead of ┠, and we don't know
            // what the last line is until we look at all the settings
            // (taking `verbose` into consideration).
            let mut info = IndexMap::new();
            info.insert("config", repo.config_path());
            info.insert("path", repo.full_path());
            match repo.name() {
                Some(name) => {
                    info.insert("name", name);
                }
                None => if verbose {
                    info.insert("name", name_default);
                },
            }
            match repo.symbol() {
                Some(symbol) => {
                    info.insert("symbol", symbol);
                }
                None => if verbose {
                    info.insert("symbol", symbol_default);
                },
            }
            if verbose || !tags_vec.is_empty() {
                info.insert("tags", &tags);
            }

            // Pretty-print information, "keyed" by the user-specified
            // path from the configuration.
            println!("{}", Color::Purple.bold().paint(path));
            for (i, (key, value)) in info.iter().enumerate() {
                // 2500 is "─" (light horizontal box drawing character)
                let mut h = String::from("\u{2500}");
                // Left-pad with light horizontal bar
                for _ in 0..6 - key.len() {
                    h.push_str("\u{2500}");
                }
                h.push_str(" ");
                // 2516 is "┖" (up heavy and right light)
                // 2510 is "┠" (vertical heavy and right light)
                // Use 2516 for the last item, 2510 for all others.
                let v = if i == (info.len() - 1) {
                    "\u{2516}"
                } else {
                    "\u{2520}"
                };
                // Box-drawing chars and key in blue, value in default
                // terminal color.
                println!(
                    "{}{}",
                    Color::Blue.paint(format!("  {}{}{}: ", v, h, key)),
                    value
                )
            }
        }
    }
    println!();
}
