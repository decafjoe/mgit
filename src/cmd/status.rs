use clap::{App, Arg, ArgMatches};

use cfg::Config;

pub const NAME: &str = "status";
pub const ABOUT: &str = "Print status summary for each repo";

const GROUP_ARG: &str = "GROUP";
const VERBOSE_ARG: &str = "VERBOSE";

pub fn app<'a>() -> App<'a, 'a> {
    App::new(NAME)
        .about(ABOUT)
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
