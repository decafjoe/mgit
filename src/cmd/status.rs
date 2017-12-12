use clap::{App, Arg, ArgMatches, SubCommand};

use cfg::Config;

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

pub fn run(_: &Config, _: &ArgMatches) {}
