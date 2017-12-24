//! Prints the configuration.
use ansi_term::Style;
use clap::{App, Arg, SubCommand};

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
    let mut paths = Vec::new();
    for repo in repos {
        paths.push(repo.path());
    }
    paths.sort();
    for path in paths {
        let verbose = invocation.matches().is_present(VERBOSE_ARG);
        let repo = invocation.config().repo(path)
            .expect(&format!("could not get repo for path {}", path));
        println!("{}", repo.path());
        println!("  path: {}",
                 repo.absolute_path().unwrap().to_str().unwrap());
        match repo.name() {
            Some(name) => println!("  name: {}", name),
            None => if verbose {
                println!("  name: {} (default)", repo.name_or_default())
            },
        }
        match repo.comment() {
            Some(comment) => println!("  comment: {}", comment),
            None => if verbose {
                println!("  comment: <not set>")
            },
        }
        match repo.symbol() {
            Some(symbol) => println!("  symbol: {}", symbol),
            None => if verbose {
                println!("  symbol: {} (default)", repo.symbol_or_default())
            },
        }
        let tags = repo.tags();
        if tags.len() > 0 {
            print!("  tags: ");
            for (i, tag) in tags.iter().enumerate() {
                if i != 0 {
                    print!(", ");
                }
                print!("{}", tag);
            }
            println!()
        } else if verbose {
            println!("  tags: <none set>")
        }
    }
}
