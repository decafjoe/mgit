//! `status` subcommand.
use std::collections::HashMap;

use ansi_term::{Color, Style};
use clap::Arg;
use git2::{Status, StatusOptions, StatusShow};

use app::{Invocation, Repo};
use ui::{Kind, Note, Summary, TrackingBranches};

/// Name of the command (`status`).
pub const NAME: &str = "status";
/// One-line description of the command (`status`).
pub const ABOUT: &str = "Prints current status of repositories";

/// Name of the argument for `-t/--tag`.
const TAG_ARG: &str = "TAG";
/// Name of the argument for `-v/--verbose`.
const VERBOSE_ARG: &str = "VERBOSE";

/// Group number for errors encountered when fetching statuses.
const STATUS_FAILURE_GROUP: usize = 0;
/// Group number for errors encountered when getting branch status.
const BRANCH_FAILURE_GROUP: usize = 1;

/// Group number for files that are changed in index but uncommitted.
const STATUS_INDEXED_GROUP: usize = 10;
/// Group number for modified files.
const STATUS_MODIFIED_GROUP: usize = 11;
/// Group number for untracked files.
const STATUS_UNTRACKED_GROUP: usize = 12;

/// Group number for branch status messages.
const BRANCH_STATUS_GROUP: usize = 110;

/// Returns the arguments for the command.
pub fn args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name(TAG_ARG)
            .help("Limits/groups display to repos with specified tag(s)")
            .short("t")
            .long("tag")
            .multiple(true)
            .number_of_values(1),
        Arg::with_name(VERBOSE_ARG)
            .help("Shows defaults in addition to user-specified config")
            .short("v")
            .long("verbose"),
    ]
}

/// Executes the `status` subcommand.
pub fn run(invocation: &Invocation) {
    invocation.start_pager();
    let verbose = invocation.matches().is_present(VERBOSE_ARG);
    let header = Style::new().bold().underline();
    let mut cache: HashMap<&Repo, Summary> = HashMap::new();
    for (tag, repos) in invocation.iter_tags(TAG_ARG) {
        if let Some(tag) = tag {
            println!("\n{}{}", header.paint("TAG:"), header.paint(tag));
        } else {
            println!();
        }
        for (name, repo) in repos {
            if cache.get(repo).is_none() {
                let mut summary = Summary::new();
                let git = repo.git();

                let mut status_options = StatusOptions::new();
                status_options.show(StatusShow::IndexAndWorkdir);
                status_options.exclude_submodules(true);
                status_options.renames_head_to_index(true);
                status_options.renames_index_to_workdir(true);
                status_options.renames_from_rewrites(true);
                status_options.include_untracked(true);
                status_options.recurse_untracked_dirs(true);

                if let Ok(statuses) = git.statuses(Some(&mut status_options)) {
                    /// Returns a new `Note` for the given status
                    /// result.
                    fn note_for_status(group: usize, count: usize, description: &str) -> Note {
                        let kind = if count > 0 { Kind::Failure } else { Kind::None };
                        let files = if count == 1 { "file is" } else { "files are" };
                        Note::new(group, kind, &format!("{} {} {}", count, files, description))
                    }

                    let indexed = statuses
                        .iter()
                        .filter(|status_entry| {
                            status_entry.status().intersects(
                                Status::INDEX_DELETED
                                    | Status::INDEX_MODIFIED
                                    | Status::INDEX_NEW
                                    | Status::INDEX_RENAMED
                                    | Status::INDEX_TYPECHANGE,
                            )
                        })
                        .count();
                    summary.push_note(note_for_status(
                        STATUS_INDEXED_GROUP,
                        indexed,
                        "changed in index but uncommitted",
                    ));
                    let modified = statuses
                        .iter()
                        .filter(|status_entry| {
                            status_entry.status().intersects(
                                Status::WT_DELETED
                                    | Status::WT_MODIFIED
                                    | Status::WT_RENAMED
                                    | Status::WT_TYPECHANGE,
                            )
                        })
                        .count();
                    summary.push_note(note_for_status(STATUS_MODIFIED_GROUP, modified, "modified"));
                    let untracked = statuses
                        .iter()
                        .filter(|status_entry| status_entry.status().intersects(Status::WT_NEW))
                        .count();
                    summary.push_note(note_for_status(
                        STATUS_UNTRACKED_GROUP,
                        untracked,
                        "untracked",
                    ));
                } else {
                    summary.push_note(Note::new(
                        STATUS_FAILURE_GROUP,
                        Kind::Failure,
                        "failed to get status info",
                    ));
                }

                match TrackingBranches::for_repository(&git) {
                    Ok(branches) => {
                        for branch in branches {
                            let local_name = branch.local_name();
                            let upstream_name = branch.upstream_name();
                            let (ahead, behind) = match git
                                .graph_ahead_behind(branch.local_oid(), branch.upstream_oid())
                            {
                                Ok((ahead, behind)) => (ahead, behind),
                                Err(e) => {
                                    summary.push_note(Note::new(
                                        BRANCH_FAILURE_GROUP,
                                        Kind::Failure,
                                        &format!(
                                            "failed to determine relationship between local \
                                             branch {} and upstream branch {} ({})",
                                            local_name, upstream_name, e,
                                        ),
                                    ));
                                    continue;
                                }
                            };
                            if ahead > 0 && behind > 0 {
                                summary.push_note(Note::new(
                                    BRANCH_STATUS_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "{} has diverged from {} ({} and {} commits)",
                                        local_name, upstream_name, ahead, behind
                                    ),
                                ));
                            } else if ahead > 0 {
                                let s = if ahead == 1 { "" } else { "s" };
                                summary.push_note(Note::new(
                                    BRANCH_STATUS_GROUP,
                                    Kind::Warning,
                                    &format!(
                                        "{} is ahead of {} by {} commit{}",
                                        local_name, upstream_name, ahead, s
                                    ),
                                ));
                            } else if behind > 0 {
                                let s = if ahead == 1 { "" } else { "s" };
                                summary.push_note(Note::new(
                                    BRANCH_STATUS_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "{} is behind {} by {} commit{}",
                                        local_name, upstream_name, behind, s
                                    ),
                                ));
                            } else {
                                summary.push_note(Note::new(
                                    BRANCH_STATUS_GROUP,
                                    Kind::None,
                                    &format!("{} is up to date with {}", local_name, upstream_name),
                                ));
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            summary.push_note(Note::new(
                                BRANCH_FAILURE_GROUP,
                                Kind::Failure,
                                error.message(),
                            ));
                        }
                    }
                }

                cache.insert(repo, summary);
            }
            let summary = cache.get(repo).unwrap_or_else(|| {
                panic!(
                    "failed to get summary from cache for repo '{}'",
                    repo.name_or_default()
                )
            });
            let color = match summary.kind() {
                Kind::None | Kind::Success => Color::Green,
                Kind::Warning => Color::Yellow,
                Kind::Failure => Color::Red,
            };
            let full_path = if verbose {
                format!(" \n  \u{2022} {}", repo.full_path())
            } else {
                "".to_owned()
            };
            println!(
                "{} {}{}",
                color.bold().paint(repo.symbol_or_default()),
                color.bold().paint(name),
                full_path
            );
            for note in summary.iter() {
                let style = match (verbose, note.kind()) {
                    (true, &Kind::Warning) => Color::Yellow.normal(),
                    (true, &Kind::Failure) => Color::Red.normal(),
                    (false, _) | (true, &Kind::None) | (true, &Kind::Success) => Style::new(),
                };
                if verbose || *note.kind() != Kind::None {
                    println!("{}", style.paint(format!("  \u{2192} {}", note.message())))
                }
            }
        }
    }
    println!();
}
