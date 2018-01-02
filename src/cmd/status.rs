//! Prints the status information for repositories.
use std::collections::HashMap;

use ansi_term::Style;
use ansi_term::Color::{Green, Red, Yellow};
use clap::{App, Arg, SubCommand};
use git2;
use git2::{BranchType, Error, Repository, Status, StatusOptions, StatusShow};

use config::{Repo, ReposIterator};
use invocation::Invocation;
use ui::{Severity, Summary};

// ----- Worktree -------------------------------------------------------------

/// Convenience struct for accessing worktree state.
struct Worktree<'a> {
    /// Reference to the repository instance for this worktree.
    repo: &'a Repository,
}

impl<'a> Worktree<'a> {
    /// Creates and returns a new `Worktree` for the libgit2
    /// `Repository` `repo`.
    fn new(repo: &'a Repository) -> Worktree<'a> {
        Worktree { repo: repo }
    }

    /// Internal method to create and set up the `StatusOptions`
    /// instance.
    fn status_options(&self) -> StatusOptions {
        let mut s = StatusOptions::new();
        s.exclude_submodules(true);
        s.renames_head_to_index(true);
        s.renames_index_to_workdir(true);
        s.renames_from_rewrites(true);
        s
    }

    /// Internal method to return a count by filtering on status `f`.
    fn filter(
        &self,
        s: &mut StatusOptions,
        f: Status,
    ) -> Result<usize, Error> {
        let statuses = self.repo.statuses(Some(s))?;
        Ok(statuses.iter().filter(|e| e.status().intersects(f)).count())
    }

    /// Returns count of indexed but uncommitted files.
    fn uncommitted(&self) -> Result<usize, Error> {
        let mut s = self.status_options();
        s.show(StatusShow::Index);
        Ok(self.repo.statuses(Some(&mut s))?.len())
    }

    /// Returns count of modified files.
    fn modified(&self) -> Result<usize, Error> {
        let mut s = self.status_options();
        s.show(StatusShow::Workdir);
        let flags = git2::STATUS_WT_DELETED | git2::STATUS_WT_MODIFIED
            | git2::STATUS_WT_RENAMED
            | git2::STATUS_WT_TYPECHANGE;
        self.filter(&mut s, flags)
    }

    /// Returns count of untracked files.
    fn untracked(&self) -> Result<usize, Error> {
        let mut s = self.status_options();
        s.show(StatusShow::Workdir);
        s.include_untracked(true);
        s.recurse_untracked_dirs(true);
        self.filter(&mut s, git2::STATUS_WT_NEW)
    }
}

// ----- Helper Functions -----------------------------------------------------

/// Convenience function for adding a worktree file `count`
/// (uncommitted, modified, or untracked) to the `summary`.
fn add_wt_files_note(
    summary: &mut Summary,
    description: &str,
    count: Result<usize, Error>,
) {
    let n = count.expect(&format!("failed to get {} count", description));
    let sev = match n {
        0 => Severity::Info,
        _ => Severity::Warning,
    };
    let files = match n {
        1 => "file is",
        _ => "files are",
    };
    summary.add_note(sev, &format!("{} {} {}", n, files, description));
}

/// Returns the status `Summary` for `repo`.
fn get_summary(repo: &Repo) -> Summary {
    let mut summary = Summary::new();
    let git = repo.git();

    let wt = Worktree::new(git);
    add_wt_files_note(&mut summary, "uncommitted", wt.uncommitted());
    add_wt_files_note(&mut summary, "modified", wt.modified());
    add_wt_files_note(&mut summary, "untracked", wt.untracked());

    let branches = git.branches(Some(BranchType::Local))
        .expect("failed to get branch info from repo");
    for branch_result in branches {
        if let Ok((local, _)) = branch_result {
            if let Ok(upstream) = local.upstream() {
                let l_name = local
                    .name()
                    .expect("failed to get local branch name")
                    .expect("local branch name is not valid utf-8");
                let l_oid =
                    local.get().target().expect("failed to get local oid");
                let u_name = upstream
                    .name()
                    .expect("failed to get upstream branch name")
                    .expect("upstream branch name is not valid utf-8");
                let u_oid = upstream
                    .get()
                    .target()
                    .expect("failed to get upstream oid");
                let (ahead, behind) = git.graph_ahead_behind(l_oid, u_oid)
                    .expect("failed to determine relationship between oids");
                if ahead > 0 && behind > 0 {
                    summary.add_note(
                        Severity::Warning,
                        &format!(
                            "{} has diverged from {} ({} and {} commits)",
                            l_name, u_name, ahead, behind
                        ),
                    );
                } else if ahead > 0 {
                    let s = match ahead {
                        1 => "",
                        _ => "s",
                    };
                    summary.add_note(
                        Severity::Notice,
                        &format!(
                            "{} is ahead of {} by {} commit{}",
                            l_name, u_name, ahead, s
                        ),
                    );
                } else if behind > 0 {
                    let s = match ahead {
                        1 => "",
                        _ => "s",
                    };
                    summary.add_note(
                        Severity::Warning,
                        &format!(
                            "{} is behind {} by {} commit{}",
                            l_name, u_name, behind, s
                        ),
                    );
                } else {
                    summary.add_note(
                        Severity::Info,
                        &format!("{} is up to date with {}", l_name, u_name),
                    );
                }
            }
        }
    }

    summary
}

/// Prints status for an individual repo.
///
/// `invocation` is used to check for the presence of the
/// `-v/--verbose` argument.
///
/// `repo` is the repo for which to print the status.
///
/// `cache` is a `HashMap` that maps repo names (`String`) to
/// `Summary` instances (see comment inside `run`). If the summary for
/// the repo is not present, we call `get_summary` and store that in
/// the `cache`.
#[cfg_attr(feature = "cargo-clippy", allow(print_stdout))]
fn print_repo_status(
    invocation: &Invocation,
    repo: &Repo,
    cache: &mut HashMap<String, Summary>,
) {
    let name = repo.name_or_default();
    let insert = cache.get(&name).is_none();
    if insert {
        cache.insert(name.to_owned(), get_summary(repo));
    }
    let summary = cache.get(&name).expect("failed to get summary");

    let verbose = invocation.matches().is_present(VERBOSE_ARG);
    let path = if verbose {
        format!(
            " \n  \u{2022} {}",
            repo.absolute_path()
                .expect("failed to get path for repo")
                .to_str()
                .expect("failed to cast path to string")
        )
    } else {
        "".to_owned()
    };

    let color = match summary.severity() {
        Severity::Info => Green,
        Severity::Notice => Yellow,
        Severity::Warning => Red,
    };
    println!(
        "{} {}{}",
        color.bold().paint(repo.symbol_or_default()),
        color.bold().paint(name),
        path
    );

    for note in summary.notes() {
        let style = match (verbose, note.severity()) {
            (false, _) | (true, &Severity::Info) => Style::new(),
            (true, &Severity::Notice) => Yellow.normal(),
            (true, &Severity::Warning) => Red.normal(),
        };
        if verbose || *note.severity() != Severity::Info {
            println!(
                "{}",
                style.paint(format!("  \u{2192} {}", note.content()))
            )
        }
    }
}

/// Prints status information for a given `ReposIterator`.
///
/// The repos are printed in a stable, deterministic order, sorted on
/// the repo's name.
///
/// `invocation` is simply passed through to `print_repo_status`.
///
/// `repos` is the `ReposIterator` containing the repos for which to
/// print status.
///
/// `cache` is simply passed through to `print_repo_status`. See the
/// docs on that function for more info.
fn print_repos_status(
    invocation: &Invocation,
    repos: ReposIterator,
    cache: &mut HashMap<String, Summary>,
) {
    // Sort by name so the output order is deterministic and
    // reasonably sane.
    let mut names = Vec::new();
    for repo in repos {
        names.push((repo.name_or_default(), repo))
    }
    names.sort_by(|a, b| a.0.cmp(&b.0));

    for (_, repo) in names {
        print_repo_status(invocation, repo, cache)
    }
}

// ----- Command --------------------------------------------------------------

/// Name of the command (`name`).
pub const NAME: &str = "status";

/// Name of the argument for tags.
const TAG_ARG: &str = "TAG";
/// Name of the argument for verbose output.
const VERBOSE_ARG: &str = "VERBOSE";

/// Returns configured subcommand instance for this command.
pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints status information about repositories")
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
                .help("Shows all status information, even if up-to-date")
                .short("v")
                .long("verbose"),
        )
}

/// Prints status of repositories per arguments supplied by user.
#[cfg_attr(feature = "cargo-clippy", allow(print_stdout))]
pub fn run(invocation: &Invocation) {
    // Cache the results as we get them. Repo status may be printed
    // multiple times (for multiple -t arguments) and we don't want to
    // hit the git2 API any more than we have to, since it's
    // (relatively) expensive.
    //
    // The cache maps repo name (a `String`) to its `Summary`.
    let mut cache: HashMap<String, Summary> = HashMap::new();

    if let Some(tags) = invocation.matches().values_of(TAG_ARG) {
        let style = Style::new().bold().underline();
        for tag in tags {
            println!("\n{}{}", style.paint("TAG:"), style.paint(tag));
            print_repos_status(
                invocation,
                invocation.config().repos_tagged(tag),
                &mut cache,
            )
        }
    } else {
        println!();
        print_repos_status(
            invocation,
            invocation.config().repos_iter(),
            &mut cache,
        )
    }
    println!()
}
