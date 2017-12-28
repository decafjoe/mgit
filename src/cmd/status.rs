//! Prints the status information for repositories.
use clap::{App, Arg, SubCommand};

use invocation::Invocation;

pub const NAME: &str = "status";

const TAG_ARG: &str = "TAG";
const VERBOSE_ARG: &str = "VERBOSE";

pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints status information about repositories")
        .arg(Arg::with_name(TAG_ARG)
             .help("Limits display to repos with specified tag(s)")
             .short("t")
             .long("tag")
             .multiple(true)
             .number_of_values(1))
        .arg(Arg::with_name(VERBOSE_ARG)
             .help("Shows all status information, even if up-to-date")
             .short("v")
             .long("verbose"))
}

pub fn run(invocation: &Invocation) {
    invocation.control().error("the pull subcommand is not yet implemented")
}
