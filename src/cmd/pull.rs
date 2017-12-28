//! Pulls from remotes and moves tracking branches forward if possible
//! to do so safely.
use clap::{App, Arg, SubCommand};

use invocation::Invocation;

pub const NAME: &str = "pull";

const TAG_ARG: &str = "TAG";

pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Pulls from remotes and fast-forwards tracking branches if \
                possible to do so safely")
        .arg(Arg::with_name(TAG_ARG)
             .help("Limits pull to repos with specified tag(s)")
             .short("t")
             .long("tag")
             .multiple(true)
             .number_of_values(1))
}

pub fn run(invocation: &Invocation) {
    invocation.control().error("the status subcommand is not yet implemented")
}
