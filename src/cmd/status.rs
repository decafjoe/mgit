//! `status` subcommand.
use std::collections::HashMap;

use ansi_term::{Color, Style};
use clap::{App, Arg, SubCommand};
use git2;
use git2::{BranchType, StatusOptions, StatusShow};

use app::{Invocation, Repo};
use ui::{Kind, Note, Summary};

/// Name of the command (`status`).
pub const NAME: &str = "status";

/// Name of the argument for `-t/--tag`.
const TAG_ARG: &str = "TAG";
/// Name of the argument for `-v/--verbose`.
const VERBOSE_ARG: &str = "VERBOSE";

/// Group number for errors encountered when fetching statuses.
const STATUS_FAILURE_GROUP: usize = 0;
/// Group number for files that are changed in index but uncommitted.
const STATUS_INDEXED_GROUP: usize = 10;
/// Group number for modified files.
const STATUS_MODIFIED_GROUP: usize = 11;
/// Group number for untracked files.
const STATUS_UNTRACKED_GROUP: usize = 12;
/// Group number for errors encountered when getting branch status.
const BRANCH_FAILURE_GROUP: usize = 1;
/// Group number for branch status messages.
const BRANCH_STATUS_GROUP: usize = 110;

/// Returns configured clap subcommand for `status`.
pub fn subcommand<'a>() -> App<'a, 'a> {
    SubCommand::with_name(NAME)
        .about("Prints current status of repositories")
        .arg(
            Arg::with_name(TAG_ARG)
                .help("Limits/groups display to repos with specified tag(s)")
                .short("t")
                .long("tag")
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name(VERBOSE_ARG)
                .help("Shows defaults in addition to user-specified config")
                .short("v")
                .long("verbose"),
        )
}

/// Executes the `status` subcommand.
#[cfg_attr(feature = "cargo-clippy", allow(print_stdout))]
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
                    fn note_for_status(
                        group: usize,
                        count: usize,
                        description: &str,
                    ) -> Note {
                        let kind =
                            if count > 0 { Kind::Failure } else { Kind::None };
                        let files =
                            if count == 1 { "file is" } else { "files are" };
                        Note::new(
                            group,
                            kind,
                            &format!("{} {} {}", count, files, description),
                        )
                    }

                    let indexed = statuses
                        .iter()
                        .filter(|status_entry| {
                            status_entry.status().intersects(
                                git2::STATUS_INDEX_DELETED
                                    | git2::STATUS_INDEX_MODIFIED
                                    | git2::STATUS_INDEX_NEW
                                    | git2::STATUS_INDEX_RENAMED
                                    | git2::STATUS_INDEX_TYPECHANGE,
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
                                git2::STATUS_WT_DELETED
                                    | git2::STATUS_WT_MODIFIED
                                    | git2::STATUS_WT_RENAMED
                                    | git2::STATUS_WT_TYPECHANGE,
                            )
                        })
                        .count();
                    summary.push_note(note_for_status(
                        STATUS_MODIFIED_GROUP,
                        modified,
                        "modified",
                    ));
                    let untracked = statuses
                        .iter()
                        .filter(|status_entry| {
                            status_entry
                                .status()
                                .intersects(git2::STATUS_WT_NEW)
                        })
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

                match git.branches(Some(BranchType::Local)) {
                    Ok(branches) => for branch in branches {
                        let local = match branch {
                            Ok((local, _)) => local,
                            Err(e) => {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "failed to get info for local branch \
                                         ({})",
                                        e
                                    ),
                                ));
                                continue;
                            }
                        };
                        let local_name = match local.name() {
                            Ok(name) => if let Some(name) = name {
                                name
                            } else {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    "local branch name is not valid utf-8",
                                ));
                                continue;
                            },
                            Err(e) => {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "failed to get name of local branch \
                                         ({})",
                                        e
                                    ),
                                ));
                                continue;
                            }
                        };
                        let local_oid = if let Some(oid) = local.get().target()
                        {
                            oid
                        } else {
                            summary.push_note(Note::new(
                                BRANCH_FAILURE_GROUP,
                                Kind::Failure,
                                &format!(
                                    "failed to resolve oid for local branch \
                                     '{}'",
                                    local_name
                                ),
                            ));
                            continue;
                        };
                        let upstream = if let Ok(upstream) = local.upstream() {
                            upstream
                        } else {
                            // Assume there is no upstream branch
                            // (though technically this could be an
                            // actual error).
                            continue;
                        };
                        let upstream_name = match upstream.name() {
                            Ok(name) => if let Some(name) = name {
                                name
                            } else {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "upstream branch name for local \
                                         branch '{}' is not valid utf-8",
                                        local_name
                                    ),
                                ));
                                continue;
                            },
                            Err(e) => {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "failed to get name of upstream \
                                         branch for local branch '{}' ({})",
                                        local_name, e
                                    ),
                                ));
                                continue;
                            }
                        };
                        let upstream_oid =
                            if let Some(oid) = upstream.get().target() {
                                oid
                            } else {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "failed to resolve oid for upstream \
                                         branch '{}'",
                                        upstream_name
                                    ),
                                ));
                                continue;
                            };
                        let (ahead, behind) = match git.graph_ahead_behind(
                            local_oid,
                            upstream_oid,
                        ) {
                            Ok((ahead, behind)) => (ahead, behind),
                            Err(e) => {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    &format!(
                                        "failed to determine relationship \
                                         between local branch '{}' and \
                                         upstream branch '{}' ({})",
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
                                    "{} has diverged from {} ({} and {} \
                                     commits)",
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
                                &format!(
                                    "{} is up to date with {}",
                                    local_name, upstream_name
                                ),
                            ));
                        }
                    },
                    Err(e) => {
                        summary.push_note(Note::new(
                            BRANCH_FAILURE_GROUP,
                            Kind::Failure,
                            &format!(
                                "failed to fetch local branch data ({})",
                                e
                            ),
                        ));
                    }
                }

                cache.insert(repo, summary);
            }
            let summary = cache.get(repo).expect(&format!(
                "failed to get summary from cache for repo '{}'",
                repo.name_or_default()
            ));
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
                    (false, _)
                    | (true, &Kind::None)
                    | (true, &Kind::Success) => Style::new(),
                };
                if verbose || *note.kind() != Kind::None {
                    println!(
                        "{}",
                        style.paint(format!("  \u{2192} {}", note.message()))
                    )
                }
            }
        }
    }
    println!();
}
