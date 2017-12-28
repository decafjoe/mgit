//! Prints the configuration.
use ansi_term::Style;
use ansi_term::Color::{Blue, Purple};
use clap::{App, Arg, SubCommand};
use ordermap::OrderMap;

use config::ReposIterator;
use invocation::Invocation;

pub const NAME: &str = "config";

const TAG_ARG: &str = "TAG";
const VERBOSE_ARG: &str = "VERBOSE";

pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints configuration values")
        .arg(Arg::with_name(TAG_ARG)
             .help("Limits display to specified tag(s)")
             .short("t")
             .long("tag")
             .multiple(true)
             .number_of_values(1))
        .arg(Arg::with_name(VERBOSE_ARG)
             .help("Shows defaults in addition to user-specified config")
             .short("v")
             .long("verbose"))
}

pub fn run(invocation: &Invocation) {
    if let Some(tags) = invocation.matches().values_of(TAG_ARG) {
        let style = Style::new().bold().underline();
        for tag in tags {
            println!("\n{}{}", style.paint("TAG:"), style.paint(tag));
            print_repos(invocation, invocation.config().repos_tagged(tag));
        }
    } else {
        println!();
        print_repos(invocation, invocation.config().repos_iter());
    }
    println!();
}

pub fn print_repos(invocation: &Invocation, repos: ReposIterator) {
    let verbose = invocation.matches().is_present(VERBOSE_ARG);

    // Sort by path so the output order is deterministic and
    // reasonably sane.
    let mut paths = Vec::new();
    for repo in repos {
        paths.push(repo.path())
    }
    paths.sort();

    for path in paths {
        let repo = invocation.config().repo(path)
            .expect(&format!("could not get repo for path {}", path));

        // Buffer information into a hashmap (which iterates in
        // insertion order). We need to buffer since we want to draw ┖
        // on the last line instead of ┠, and we don't know what the
        // last line is until we look at all the settings (taking
        // `verbose` into consideration).
        let mut facts = OrderMap::new();

        // The unwraps are ok because we do extensive checks when
        // processing the configuration, including checking that the
        // path can be resolved, exists, and can be turned into a
        // string.
        let path = repo.absolute_path().unwrap().to_str().unwrap().to_owned();
        facts.insert("path", path);

        match repo.name() {
            Some(name) => { facts.insert("name", name); },
            None => if verbose {
                facts.insert("name",
                             format!("{} (default)", repo.name_or_default()));
            },
        }

        match repo.comment() {
            Some(comment) => { facts.insert("comment", comment); },
            None => if verbose {
                facts.insert("comment", "<not set>".to_owned());
            },
        }

        match repo.symbol() {
            Some(symbol) => { facts.insert("symbol", symbol); },
            None => if verbose {
                let symbol = format!("{} (default)", repo.symbol_or_default());
                facts.insert("symbol", symbol);
            },
        }

        let tags = repo.tags();
        if tags.len() > 0 {
            let mut s = String::new();
            for (i, tag) in tags.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ")
                }
                s.push_str(&tag)
            }
            facts.insert("tags", s);
        } else if verbose {
            facts.insert("tags", "<none set>".to_owned());
        }

        println!("{}", Purple.bold().paint(repo.path()));
        for (i, (key, value)) in facts.iter().enumerate() {
            let mut line = String::from("─");
            for _ in 0..7 - key.len() {
                line.push_str("─");
            }
            line.push_str(" ");
            let left = if i == (facts.len() - 1) {
                "┖"
            } else {
                "┠"
            };
            println!("{}{}",
                     Blue.paint(format!("  {}{}{}: ", left, line, key)), value)
        }
    }
}
