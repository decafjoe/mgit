use std::cmp::Ordering;

use ansi_term::Style;
use ansi_term::Color::{Green, Red, Yellow};
use clap::{App, Arg, ArgMatches, SubCommand};
use git2::Error;
use pager::Pager;

use cfg::{Config, Repo};
use wt::Worktree;

pub const NAME: &str = "status";

const GROUP_ARG: &str = "GROUP";
const VERBOSE_ARG: &str = "VERBOSE";

#[derive(Clone, PartialEq, PartialOrd)]
enum Severity {
    Info,
    Notice,
    Warning,
}

struct Note {
    content: String,
    severity: Severity,
}

impl Note {
    pub fn new(severity: Severity, content: &str) -> Note {
        Note{ content: content.to_owned(), severity: severity }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn severity(&self) -> &Severity {
        &self.severity
    }
}

struct Summary {
    notes: Vec<Note>,
}

impl Summary {
    pub fn new() -> Summary {
        Summary{ notes: Vec::new() }
    }

    pub fn notes(&self) -> &Vec<Note> {
        &self.notes
    }

    pub fn add_note(&mut self, severity: Severity, content: &str) {
        self.notes.push(Note::new(severity, content));
    }

    pub fn severity(&self) -> Severity {
        let mut rv = Severity::Info;
        for note in &self.notes {
            let s = note.severity();
            if *s > rv {
                rv = s.clone();
            }
        }
        rv
    }
}

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
                print_status(repos.get(name).unwrap(), verbose, false);
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
            // TODO(jjoyce): group = true if there are multiple repos with the
            //               same name
            print_status(repo, verbose, false);
        }
    }
    println!();
}

fn add_wt_files_note(summary: &mut Summary, desc: &str,
                     nr: Result<usize, Error>) {
    let n = nr.expect(&format!("failed to get {} count", desc));
    let sev = match n {
        0 => Severity::Info,
        _ => Severity::Warning,
    };
    let files = match n {
        1 => "file is",
        _ => "files are",
    };
    summary.add_note(sev, &format!("{} {} {}", n, files, desc));
}

fn print_status(repo: &Repo, verbose: bool, group_name: bool) {
    let mut summary = Summary::new();

    let wt = Worktree::new(repo.git());
    add_wt_files_note(&mut summary, "uncommitted", wt.uncommitted());
    add_wt_files_note(&mut summary, "modified", wt.modified());
    add_wt_files_note(&mut summary, "untracked", wt.untracked());

    // TODO(jjoyce): add notes to the summary for each tracking branch

    let color = match summary.severity() {
        Severity::Info => Green,
        Severity::Notice => Yellow,
        Severity::Warning => Red,
    };
    let group = if verbose || group_name {
        format!(" ({})", repo.group_name())
    } else {
        "".to_owned()
    };
    let path = if verbose {
        format!(" \n  • {}", repo.path())
    } else {
        "".to_owned()
    };
    println!("{} {}{}{}",
             color.bold().paint(repo.symbol()),
             color.bold().paint(repo.name()),
             color.paint(group),
             path);
    for note in summary.notes() {
        let style = match (verbose, note.severity()) {
            (false, _) | (true, &Severity::Info) => Style::new(),
            (true, &Severity::Notice) => Yellow.normal(),
            (true, &Severity::Warning) => Red.normal(),
        };
        if verbose || *note.severity() != Severity::Info {
            println!("{}", style.paint(format!("  → {}", note.content())));
        }
    }
}
