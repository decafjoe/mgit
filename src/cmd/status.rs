use std::cmp::Ordering;

use ansi_term::Style;
use clap::{App, Arg, ArgMatches, SubCommand};
use pager::Pager;

use cfg::{Config, Repo};

pub const NAME: &str = "status";

const GROUP_ARG: &str = "GROUP";
const VERBOSE_ARG: &str = "VERBOSE";

pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints status summary for each repo")
        .arg(Arg::with_name(GROUP_ARG)
            .help("Separates output by group (default is a merged list)")
            .short("g")
            .long("group"))
        .arg(Arg::with_name(VERBOSE_ARG)
             .help("Prints status information even if not notable")
             .short("v")
             .long("verbose"))
}

pub fn run(config: &Config, matches: &ArgMatches) {
    Pager::with_pager("less -efFnrX").setup();
    println!();
    let verbose = matches.is_present(VERBOSE_ARG);
    if matches.is_present(GROUP_ARG) {
        let style = Style::new().bold().underline();
        let groups = config.groups();
        let mut names = groups.keys().collect::<Vec<&String>>();
        names.sort();
        for name in names {
            println!("{}", style.paint(name.as_str()));
            let repos = groups.get(name).unwrap().repos();
            let mut names = repos.keys().collect::<Vec<&String>>();
            names.sort();
            for name in names {
                print_status(verbose, repos.get(name).unwrap());
            }
        }
    } else {
        let mut repos = Vec::new();
        for (g, group) in config.groups() {
            for (r, repo) in group.repos() {
                repos.push((r.to_owned(), g.to_owned(), repo));
            }
        }
        repos.sort_by(|a, b| {
            let result = a.0.cmp(&b.0);
            match result {
                Ordering::Equal => a.1.cmp(&b.1),
                _ => result,
            }
        });
        for (_, _, repo) in repos {
            print_status(verbose, repo);
        }
    }
    println!();
}

fn print_status(verbose: bool, repo: &Repo) {
    println!("{}", repo.name());
}
