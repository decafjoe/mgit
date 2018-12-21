//! `pull` subcommand.
use std::{
    collections::{HashMap, HashSet},
    io::{stdout, Write},
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use ansi_term::{Color, Style};
use clap::Arg;
use crossbeam;
use crossbeam_channel::{self, Receiver, Sender};
use git2::{ObjectType, ResetType, StatusOptions, StatusShow};
use libc;
use termion::{
    self, clear, cursor,
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
};

use app::{Invocation, Repo};
use ui::{Kind, Note, Summary, TrackingBranches};

/// Name of the command (`pull`).
pub const NAME: &str = "pull";
/// One-line description of the command (`pull`).
pub const ABOUT: &str = "Fetches from remotes and fast-forwards local tracking branches if safe";

/// Name of the argument for `-c/--concurrent`.
const CONCURRENT_ARG: &str = "CONCURRENT";
/// Default number of concurrent fetches.
const CONCURRENT_DEFAULT: &str = "8";

/// Name of the argument for tags.
const TAG_ARG: &str = "TAG";

/// Group number for errors encountered when fetching.
const FETCH_FAILURE_GROUP: usize = 0;
/// Group number for errors encountered when fetching.
const BRANCH_FAILURE_GROUP: usize = 1;

/// Group number for fetch successes.
const FETCH_SUCCESS_GROUP: usize = 100;
/// Group number for branch status messages.
const BRANCH_STATUS_GROUP: usize = 101;

/// Number of times per second to update status of operations, as well
/// as the UI showing the status.
const UPDATE_FREQUENCY: u64 = 100;

/// Number of milliseconds after which a terminal resize is considered
/// "settled."
const DEBOUNCE_MILLIS: u64 = 500;

/// Convenience type for a `HashMap` mapping a `Repo` to its `Summary`.
type Results<'a> = HashMap<&'a Repo, Summary>;

/// Returns the arguments for the command.
pub fn args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name(CONCURRENT_ARG)
            .default_value(CONCURRENT_DEFAULT)
            .help("Number of concurrent fetches")
            .short("c")
            .long("concurrent"),
        Arg::with_name(TAG_ARG)
            .help("Limits pull to repos with specified tag(s)")
            .short("t")
            .long("tag")
            .multiple(true)
            .number_of_values(1),
    ]
}

/// Executes the `pull` subcommand.
pub fn run(invocation: &Invocation) {
    let concurrent_str = invocation
        .matches()
        .value_of(CONCURRENT_ARG)
        .unwrap_or_else(|| panic!("expected {} to have an argument", CONCURRENT_ARG));
    let concurrent = match concurrent_str.parse::<u8>() {
        Ok(concurrent) => concurrent,
        Err(e) => {
            return invocation.control().fatal(&format!(
                "failed to interpret value '{}' for {} ({})",
                concurrent_str, CONCURRENT_ARG, e
            ));
        }
    };
    if concurrent < 1 {
        invocation.control().fatal(&format!(
            "{} must be one or greater (got '{}')",
            CONCURRENT_ARG, concurrent
        ));
    }

    // Make a list of the repos we need to fetch, taking -t/--tag into account.
    let mut repo_set = HashSet::new();
    for (_, repos) in invocation.iter_tags(TAG_ARG) {
        for (_, repo) in repos {
            repo_set.insert(repo);
        }
    }

    // `remotes` starts as a vec of all the `(&Repo, remote: &str)` pairs we need
    // to fetch. As fetch threads become available, items are popped from the front
    // of this vec. Once the vec is empty, we're done. (...after we wait for the
    // current fetches to finish, of course.)
    let mut remotes = Vec::new();

    // `results` maps a `&Repo` to its `Summary`. Fetch threads trasmit `Summary`
    // instances back to the main thread, which are then merged into the master
    // `Summary` stored in this map.
    let mut results: Results = HashMap::new();

    // Iterator on which we check `next()` for Ctrl-c from the user. This is required because the
    // terminal does not translate keyboard input into interrupts when it is in raw mode. So we
    // watch for that key chord in addition to checking `sigterms_received()`, which can still be
    // triggered by signals from outside this program.
    let mut stdin = termion::async_stdin().keys();

    // Represents the termination state of the operation. See the documentation
    // on the `TerminationState` enum for more information.
    let mut termination_state = TerminationState::None;

    // The block controls the scope of `stdout`. We put the terminal into raw mode
    // to display the in-progress UI. When `stdout` goes out of scope, the terminal
    // state is reset via the destructor.
    {
        let mut stdout = stdout()
            .into_raw_mode()
            .expect("failed to put terminal into raw mode");

        // The UI instance controls all output to the terminal while the fetch threads
        // are running. UI code is messy -- so we hide the complexity. That way, the
        // main loop logic isn't cluttered.
        let mut ui = UI::new(&mut stdout);

        // Initialize `remotes`, `results`, and `ui`.
        for repo in repo_set {
            let mut summary = Summary::new();
            match repo.git().remotes() {
                Ok(names) => {
                    for name in names.iter() {
                        if let Some(name) = name {
                            remotes.push((repo, name.to_owned()));
                            ui.push_remote(repo, name);
                        } else {
                            summary.push_note(Note::new(
                                FETCH_FAILURE_GROUP,
                                Kind::Failure,
                                "skipped remote with invalid utf-8 name",
                            ));
                        }
                    }
                }
                Err(e) => {
                    summary.push_note(Note::new(
                        FETCH_FAILURE_GROUP,
                        Kind::Failure,
                        &format!("failed to get remotes ({})", e),
                    ));
                }
            }
            results.insert(repo, summary);
        }

        // `active` keeps track of how many fetch threads are currently running.
        let mut active = 0;

        // Turn `UPDATE_FREQUENCY` into an amount of time to sleep between updates.
        let t = Duration::from_millis(1000 / UPDATE_FREQUENCY);

        // `results_tx` gets cloned and handed off to each fetch thread. The
        // thread is expected to send a single message:
        //
        //   (&Repo, String, Summary)
        //
        // Once `results_rx` receives the message, the main loop assumes the
        // fetch thread is complete, and it will start a new fetch thread.
        let (results_tx, results_rx) = crossbeam_channel::unbounded();

        // Handles to the senders whose receiving ends are in the threads
        // running the `git fetch` subprocesses. If the user wants to hard
        // cancel the fetches, a single message is sent across each channel from
        // the main thread to the child threads, which lets them know to
        // terminate.
        let mut term_txs: Vec<Sender<bool>> = Vec::new();

        // Use crossbeam magic (?) because Rust threading primitives are above my head
        // and this is, like, incredibly clean-looking and appears to work exactly as
        // expected.
        crossbeam::scope(|scope| {
            // Loop until all the current threads are complete and we have nothing left to do.
            while active > 0 || !remotes.is_empty() {
                // Merge the completed `Summary`s into the master `Summary`.
                for (repo, name, summary) in results_rx.try_iter() {
                    results
                        .get_mut(repo)
                        .expect("failed to get summary for repo")
                        .push_summary(&summary);
                    let state = match summary.kind() {
                        Kind::None => State::NoChange,
                        Kind::Success => State::Success,
                        Kind::Warning => State::Warning,
                        Kind::Failure => State::Failure,
                    };
                    // Notify the UI of the change in state for the remote.
                    ui.update_state(repo, &(name as String), state);
                    // Free up a thread for use.
                    active -= 1;
                }
                // Process any keystrokes, looking for ctrl-c.
                while let Some(key) = stdin.next() {
                    if key.expect("failed to parse keyboard input") == Key::Ctrl('c') {
                        invocation.sigterm_received();
                    }
                }
                // Move to "soft" termination state if we're currently running
                // normally but the user has asked for termination.
                if termination_state == TerminationState::None && invocation.sigterms_received() > 0
                {
                    // Drain the pending fetches, setting their state to canceled.
                    while !remotes.is_empty() {
                        let (repo, name) = remotes.remove(0);
                        ui.update_state(repo, &name, State::Canceled);
                    }
                    ui.cancel(&results);
                    termination_state = TerminationState::Soft;
                }
                // Move to "hard" termination state if we're currently in "soft"
                // termination state and we have received two or more sigterms.
                if termination_state == TerminationState::Soft && invocation.sigterms_received() > 1
                {
                    for tx in &term_txs {
                        // The `term_txs` vec has references to all threads that
                        // have been started. If some have completed, those rx
                        // sides will be dead and sending a message will error
                        // out. This is an expected behavior, so ignore any
                        // errors.
                        let _ = tx.send(true);
                    }
                    termination_state = TerminationState::Hard;
                }
                // If there are available threads, and fetches to be done – start them up.
                while active < concurrent && !remotes.is_empty() {
                    let (repo, name) = remotes.remove(0);
                    // Tell the UI we have started the fetch.
                    ui.update_state(repo, &name, State::Fetching);
                    let results_tx = results_tx.clone();
                    let (term_tx, term_rx) = crossbeam_channel::bounded(1);
                    term_txs.push(term_tx);
                    scope
                        .builder()
                        .name(format!("{}:{}", repo.name_or_default(), name))
                        .spawn(move |_| {
                            let summary = fetch_and_ff(&term_rx, repo, &name);
                            results_tx
                                .send((repo, name, summary))
                                .expect("failed to transmit results to main thread");
                        })
                        .expect("failed to spawn thread for pull operation");
                    // Note that a new thread is in use.
                    active += 1;
                }
                // Give the UI a chance to update itself.
                ui.update(&results);
                // Rest for a sec before checking all the things again.
                thread::sleep(t);
            }
        })
        .expect("one or more threads panicked");
        // Tell the UI we are done fetching.
        ui.cleanup();
    } // end scope of `stdout`, terminal state should be reset

    // If the user sent two sigterms, assume it signals the intent "get me the
    // hell out of here as quickly as possible" -- don't bother them with a
    // summary.
    if termination_state == TerminationState::Hard {
        println!();
        return;
    }

    let header = Style::new().bold().underline();
    for (tag, repos) in invocation.iter_tags(TAG_ARG) {
        if let Some(tag) = tag {
            println!("\n{}{}", header.paint("TAG:"), header.paint(tag));
        } else {
            println!();
        }
        for (name, repo) in repos {
            let summary = results
                .get(repo)
                .expect("failed to look up results for repo");
            let style = style_for_kind(&summary.kind());
            println!(
                "{} {}",
                style.bold().paint(repo.symbol_or_default()),
                style.bold().paint(name)
            );
            for note in summary.iter() {
                let style = match *note.kind() {
                    Kind::None => Style::new(),
                    Kind::Success => Color::Green.normal(),
                    Kind::Warning => Color::Yellow.normal(),
                    Kind::Failure => Color::Red.normal(),
                };
                println!("{}", style.paint(format!("  \u{2192} {}", note.message())));
            }
        }
    }
    println!();
}

// ----- TerminationState ---------------------------------------------------------------------------------------------

#[derive(PartialEq)]
enum TerminationState {
    /// Not termination; running normally.
    None,
    /// Soft termination; allow running fetches to complete, do not start any
    /// new ones.
    Soft,
    /// Hard termination; kill all fetch processes and exit.
    Hard,
}

// ----- style_for_kind -----------------------------------------------------------------------------------------------

/// Returns the "standard" `Style` for the given `kind`.
fn style_for_kind(kind: &Kind) -> Style {
    match *kind {
        Kind::None => Style::new(),
        Kind::Success => Color::Green.normal(),
        Kind::Warning => Color::Yellow.normal(),
        Kind::Failure => Color::Red.normal(),
    }
}

// ----- fetch_and_ff -------------------------------------------------------------------------------------------------

/// Fetches remote, fast-forwards tracking branches if safe to do so, and
/// returns a `Summary` with the results of those operations.
///
/// # Fast-forwards
///
/// If a remote is fetched successfully, mgit iterates through the list of
/// local branches that are tracking an upstream branch from the remote. If the
/// remote is a simple fast-forward from local, mgit goes ahead and does so.
///
/// mgit will not touch the local branch if it contains commits that are not
/// known to the upstream (i.e. if local is ahead of upstream, or if the
/// branches have diverged).
///
/// If the local branch is HEAD, mgit will additionally check that the worktree
/// is completely clean (i.e. there is nothing in the index, there are no
/// modified files, there are no untracked files). If the worktree is anything
/// but pristine, mgit will not try to fast-forward.
///
/// # Git Executable vs libgit2
///
/// For the fetch, the git executable is used instead of the libgit2 bindings
/// (i.e. this creates a child process that runs `git fetch <remote>` in the
/// repo's directory).
///
/// A while back I wrote a Python version of mgit which also used the libgit2
/// bindings and it did not play well with git-remote-gcrypt. I'm sure it can
/// be made to work, but the number of lines of code it would take compared to
/// the couple tens of lines it takes to use a child process makes it a hard
/// sell.
///
/// More generally, using the libgit2 API would seem to break *any* git remote
/// helper program that relies on the `git-remote-XYZ`-as-a-command-on-PATH
/// pattern.
///
/// Performance-wise, the fetch itself is going to be in a completely different
/// league of slow than any difference between subprocess and in-process API
/// usage. So... no loss there.
///
/// Technically, I guess the git executable might not be present (and the code
/// does not handle this case). But, seriously, who's using mgit that doesn't
/// have git installed and on the PATH? (Those sound an awful lot like famous
/// last words.)
#[allow(clippy::cast_possible_wrap)]
fn fetch_and_ff(term_rx: &Receiver<bool>, repo: &Repo, name: &str) -> Summary {
    // The `git fetch` subprocess can spawn its own subprocesses. If we need to kill `git fetch` we
    // want to kill all its children as well. To do so, we make sure `git fetch` and its children
    // all have the same process group id (which we make sure is different than the parent process'
    // pgid), then use `killpg(pgid)` to kill the children without touching the parent.
    //
    // By default children inherit the same pgid as the parent, so setting the right pgid for the
    // `git fetch` means its children will also have the correct value.
    //
    // We use `before_exec` to set the pgid for `git fetch`. Per the documentation, `before_exec`
    // runs after the process fork, so the child will have a new, unique pid. When `setpgid(pid,
    // pgid)` is called with a 0 for the first argument, the call applies to the calling process
    // (our child). When pgid is 0, the pgid is set to the same value as the pid.
    let mut child = Command::new("git")
        .args(&["fetch", name])
        .current_dir(repo.full_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .before_exec(|| unsafe {
            libc::setpgid(0, 0);
            Ok(())
        })
        .spawn()
        .expect("failed to start `git fetch` command");

    // Periodically check whether the process has exited, or whether the mgit has received a
    // sigterm (in which case the child processes are killed and an empty summary returned
    // immediately).
    let t = Duration::from_millis(1000 / UPDATE_FREQUENCY);
    while None
        == child
            .try_wait()
            .expect("failed to get status of child process")
    {
        if term_rx.try_recv().is_ok() {
            unsafe {
                libc::killpg(child.id() as i32, 9);
            }
            return Summary::new();
        }
        thread::sleep(t);
    }

    // Make a final blocking call (which shouldn't actually block) to get the output from the
    // command and determine whether it completed successfully.
    let error = match child.wait_with_output() {
        Ok(out) => {
            if out.status.success() {
                None
            } else {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let rv = if stdout.len() > 0 && stderr.len() > 0 {
                    format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr)
                } else if stdout.len() > 0 {
                    stdout.into_owned()
                } else {
                    stderr.into_owned()
                };
                Some(rv)
            }
        }
        Err(e) => Some(format!("{}", e)),
    };

    let git = repo.git();
    let mut summary = Summary::new();
    if let Some(message) = error {
        // If the fetch failed, add the error message to the summary and bail out.
        summary.push_note(Note::new(
            FETCH_FAILURE_GROUP,
            Kind::Failure,
            &format!("failed to fetch from {}: {}", name, message),
        ));
    } else {
        summary.push_note(Note::new(
            FETCH_SUCCESS_GROUP,
            Kind::None,
            &format!("fetched from {}", name),
        ));
        match TrackingBranches::for_remote(&git, name) {
            Ok(branches) => {
                for branch in branches {
                    let local_name = branch.local_name();
                    let upstream_name = branch.upstream_name();
                    let upstream_oid = branch.upstream_oid();
                    let (ahead, behind) =
                        match git.graph_ahead_behind(branch.local_oid(), upstream_oid) {
                            Ok((ahead, behind)) => (ahead, behind),
                            Err(e) => {
                                summary.push_note(Note::new(
                                    BRANCH_FAILURE_GROUP,
                                    Kind::Failure,
                                    &format!(
                                "failed to determine relationship between local branch {} and \
                                 upstream branch {} ({})",
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
                        if branch.local().is_head() {
                            let mut status_options = StatusOptions::new();
                            status_options.show(StatusShow::IndexAndWorkdir);
                            status_options.exclude_submodules(true);
                            status_options.renames_head_to_index(true);
                            status_options.renames_index_to_workdir(true);
                            status_options.renames_from_rewrites(true);
                            status_options.include_untracked(true);
                            let error_message = &format!(
                                "failed to fast-forward {} to {}",
                                local_name, upstream_name
                            );
                            match git.statuses(Some(&mut status_options)) {
                                Ok(statuses) => {
                                    if !statuses.is_empty() {
                                        summary.push_note(Note::new(
                                            BRANCH_FAILURE_GROUP,
                                            Kind::Failure,
                                            &format!("{} (worktree is dirty)", error_message),
                                        ));
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    summary.push_note(Note::new(
                                        BRANCH_FAILURE_GROUP,
                                        Kind::Failure,
                                        &format!(
                                            "{} (could not get worktree status) ({})",
                                            error_message, e
                                        ),
                                    ));
                                    continue;
                                }
                            }
                        }
                        let ref_name = &format!("refs/heads/{}", local_name);
                        let mut local_reference = git
                            .find_reference(ref_name)
                            .expect("failed to get reference for local branch");
                        if let Err(e) =
                            local_reference.set_target(upstream_oid, "mgit: fast-forward")
                        {
                            summary.push_note(Note::new(
                                BRANCH_STATUS_GROUP,
                                Kind::Failure,
                                &format!(
                                    "failed to fast-forward {} to {} ({})",
                                    local_name, upstream_name, e
                                ),
                            ));
                        } else {
                            if branch.local().is_head() {
                                if let Err(e) = git.reset(
                                    &branch
                                        .upstream()
                                        .get()
                                        .peel(ObjectType::Any)
                                        .expect("failed to get upstream object"),
                                    ResetType::Hard,
                                    None,
                                ) {
                                    summary.push_note(Note::new(
                                        BRANCH_STATUS_GROUP,
                                        Kind::Failure,
                                        &format!("failed to hard reset worktree ({})", e),
                                    ));
                                    continue;
                                }
                            }
                            summary.push_note(Note::new(
                                BRANCH_STATUS_GROUP,
                                Kind::Success,
                                &format!("fast-forwarded {} to {}", local_name, upstream_name),
                            ));
                        }
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
    }
    summary
}

// ----- State --------------------------------------------------------------------------------------------------------

/// Represents the state of the fetch/fast-forward for a remote.
#[derive(Clone, Debug)]
enum State {
    /// Fetch has not yet started.
    Pending,
    /// Fetch has been canceled by the user.
    Canceled,
    /// Fetch is in progress.
    Fetching,
    /// Fetch was successful, no tracking branches were ahead or behind.
    NoChange,
    /// Fetch was successful, one or more tracking branches was successfully
    /// fast-forwarded.
    Success,
    /// Fetch was successful, one or more tracking branches is ahead of its
    /// upstream.
    Warning,
    /// Fetch was unsuccessful or one or more fast-forwards failed (due to HEAD
    /// being dirty, or diverging local/upstream branches).
    Failure,
}

// ----- UI -----------------------------------------------------------------------------------------------------------

/// Manages the user interface during fetch and fast-forward.
struct UI<'a, W: 'a + Write> {
    /// Maps `&Repo` to another `HashMap`, which maps remote names to their
    /// current `State`.
    state: HashMap<&'a Repo, HashMap<String, State>>,
    /// Queue of updates to be made next time `process_updates` is called.
    /// Format is `(<repo>, <remote-name>, <state>)`.
    updates: Vec<(&'a Repo, String, State)>,
    /// Indicates whether the user has terminated the program.
    canceled: bool,
    /// `RawTerminal` instance on which all drawing commands are done.
    t: &'a mut RawTerminal<W>,
    /// Width and height of the drawn UI.
    drawn: (u16, u16),
    /// Holds the terminal resize debounce state.
    ///
    /// Every iteration of the main loop (inside the call to `update`), we
    /// check the terminal size. When terminal size changes from the `drawn`
    /// size, this gets set to `Some(<new-width>, <new-height>,
    /// Instant::now())`.
    ///
    /// Subsequent updates will continue to check terminal size. If it changes
    /// again, a new `Some(w, h, Instant::now())` value is generated. If
    /// `DEBOUNCE_MILLIS` goes by without a change, mgit assumes the user is
    /// done resizing and redraws the UI based on the new terminal size.
    /// (Debounce is then set to `None` as we are done debouncing.)
    debounce: Option<(u16, u16, Instant)>,
    /// Cache of all strings drawn to the screen as well as their location,
    /// keyed by `&Repo` and optionally remote name (a `String`).
    ///
    /// The values are of the form `(x, y, <string>)` where x and y are termion
    /// coordinates and `<string>` is the string that was drawn to the screen
    /// for the key.
    ///
    /// A key of `(&Repo, None)` is the location of the overall repo status
    /// display. Otherwise the key will be `(&Repo, Some(String))` where the
    /// string represents the name of the remote.
    locations: HashMap<(&'a Repo, Option<String>), (u16, u16, String)>,
}

impl<'a, W: Write> UI<'a, W> {
    /// Creates and returns a new `UI` instance.
    fn new(terminal: &'a mut RawTerminal<W>) -> Self {
        Self {
            state: HashMap::new(),
            updates: Vec::new(),
            canceled: false,
            t: terminal,
            drawn: (0, 0),
            debounce: None,
            locations: HashMap::new(),
        }
    }

    /// Adds remote named `remote` for repository `repo` to the UI.
    fn push_remote(&mut self, repo: &'a Repo, remote: &str) {
        self.state.entry(repo).or_insert_with(HashMap::new);
        self.state
            .get_mut(repo)
            .expect("failed to get state value for repo")
            .insert(remote.to_owned(), State::Pending);
    }

    /// Notifies the UI of an update to the state of a remote.
    ///
    /// Note that updates are queued, and are not reflected in the UI until the
    /// `update()` method is called.
    fn update_state(&mut self, repo: &'a Repo, remote: &str, state: State) {
        self.updates.push((repo, remote.to_owned(), state));
    }

    /// Instructs the user interface to update the terminal.
    fn update(&mut self, results: &Results) {
        let (w, h) = termion::terminal_size().expect("failed to get terminal size");
        let debounce = Some((w, h, Instant::now()));
        let (drawn_w, drawn_h) = self.drawn;
        if drawn_w == 0 && drawn_h == 0 {
            self.draw(w, h, results);
        } else if let Some((new_w, new_h, t)) = self.debounce {
            if w == new_w && h == new_h {
                if t.elapsed() >= Duration::from_millis(DEBOUNCE_MILLIS) {
                    self.debounce = None;
                    self.draw(w, h, results);
                }
            } else {
                self.debounce = debounce;
            }
        } else if w != drawn_w || h != drawn_h {
            self.debounce = debounce;
        } else {
            self.process_updates(results);
        }
    }

    /// Tells the user interface that the program is terminating.
    fn cancel(&mut self, results: &Results) {
        if !self.canceled {
            self.canceled = true;
            let (w, h) = termion::terminal_size().expect("failed to get terminal size");
            self.draw(w, h, results);
            self.process_updates(results);
        }
    }

    /// Cleans up the UI and resets the terminal.
    fn cleanup(&mut self) {
        writeln!(self.t, "{}{}", clear::All, cursor::Show)
            .expect("failed to write content to the terminal");
        self.t
            .flush()
            .expect("failed to flush content to the terminal");
    }

    /// Draws the UI to `self.t`, with a width of `w` and height `h`, based on
    /// results `results`.
    ///
    /// **This is an internal method and should not be called outside the
    /// impl.**
    #[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
    fn draw(&mut self, w: u16, h: u16, results: &Results) {
        // We do some calculations where we need width and height as a usize, so we
        // just assign them some variables.
        let (w_usize, h_usize) = (w as usize, h as usize);

        // Clear the screen, and the current state of what's drawn where.
        self.locations.clear();
        write!(self.t, "{}", clear::All).expect("failed to write content to the terminal");

        // We take a lot of references when drawing the screen and setting up internal
        // state. Scope all the messy work so we can safely mutate a few things at the
        // end.
        {
            // Get the full list of repos, sorted by name. Sorting is required to make the
            // UI output deterministic.
            let mut repos: Vec<&&Repo> = self.state.keys().collect();
            repos.sort_by_key(|repo| (repo.name_or_default(), repo.path()));

            // Determine the longest name. This is how "wide" the left column of repo names
            // will be.
            let column_w = repos
                .iter()
                .max_by_key(|repo| repo.name_or_default().len())
                .expect("failed to compute column width")
                .name_or_default()
                .len();

            // If number of repos is more than the number of lines we have to display them,
            // overflow_h contains the number of repos "past the bottom" of the terminal
            // window. Count the "cancelling..." message as a repo since it takes up a line
            // of output.
            let mut rows_needed = repos.len();
            if self.canceled {
                rows_needed += 1;
            }
            let overflow_h = if h_usize < rows_needed {
                rows_needed - h_usize
            } else {
                0
            };

            let mut y: u16 = 0;
            for (i, repo) in repos.iter().enumerate() {
                // 1-based "row" we're working on (termion is 1-based)
                y = (i as u16) + 1;

                if overflow_h > 0 && y == h {
                    // This is the last line available in the terminal. If we are canceled, break
                    // the loop and allow the code below to use the last line to show the
                    // "cancelling..." message. Otherwise, use the last line to tell the user how
                    // many repositories are not displayed.
                    if !self.canceled {
                        // Number not displayed is overflow + 1, because we are also not displaying
                        // *this* repo.
                        let mut message = format!("\u{2026}{} more not shown", overflow_h + 1);
                        // Our message might be longer than the available width. If so, truncate it
                        // and add an ellipsis at the end.
                        if message.len() > w_usize {
                            message.truncate(w_usize - 1);
                            message.push_str("\u{2026}");
                        }
                        write!(self.t, "{}{}", cursor::Goto(1, y), message)
                            .expect("failed to write content to the terminal");
                    }
                    break;
                }

                // `remaining` keeps track of how many columns/characters we have left to draw
                // into.
                let mut remaining = w_usize;

                // `line` is what we're drawing into.
                let mut line = String::from("");

                // Left pad the line, so repo names end up right-aligned.
                let name = repo.name_or_default();
                let n = name.len();
                for _ in 0..column_w - n {
                    line.push_str(" ");
                    remaining -= 1;
                }

                // We need at least two characters to draw a repo name (the first character
                // plus an ellipsis). If we don't have two, draw an ellipsis at the far right
                // and bail out of this loop iteration.
                if remaining < 2 {
                    write!(self.t, "{}\u{2026}", cursor::Goto(w, y))
                        .expect("failed to write content to the terminal");
                    continue;
                }

                // Keeps track of whether we need to put an ellipsis at the end of the line.
                let mut needs_ellipsis = false;

                // If the repo name "runs past the right of the terminal," truncate it to the
                // terminal width minus one (where the one is reserved for an ellipsis).
                let (name, n) = if n >= remaining {
                    needs_ellipsis = true;
                    let s = &name[..remaining - 1];
                    (s, s.len())
                } else {
                    (name, n)
                };

                // Append the repo name (painted based on current overall status) to the string.
                let kind = results
                    .get(*repo)
                    .expect("failed to get summary for repo")
                    .kind();
                let style = style_for_kind(&kind).bold();
                line.push_str(&format!("{}", style.paint(name)));

                // Store the location and string we just painted.
                self.locations.insert(
                    (repo, None),
                    (w - (remaining as u16) + 1, y, name.to_owned()),
                );

                // Reduce the remaining characters by the number of characters that we just
                // drew into the line.
                remaining -= n;

                // Get a sorted list of remotes. Sorting is required to make the UI output
                // deterministic.
                let mut remote_names: Vec<&String> = self
                    .state
                    .get(*repo)
                    .expect("failed to get state value for repo")
                    .keys()
                    .collect();
                remote_names.sort();

                for full_name in remote_names {
                    // We need three characters to draw the remote (one for the space, one for the
                    // first character, one for the ellipsis). If we don't have three, bail.
                    if remaining < 3 {
                        needs_ellipsis = true;
                        break;
                    }

                    let n = full_name.len();

                    // If the remote name plus one (the space to the left) "runs past the right of
                    // the terminal," truncate it to the terminal width minus two (where one
                    // character is reserved for the space and the other for the ellipsis).
                    let (name, n) = if n + 1 >= remaining {
                        needs_ellipsis = true;
                        let s = &full_name[..remaining - 2];
                        (s, s.len())
                    } else {
                        (full_name.as_str(), n)
                    };

                    // Add the stylized remote name to the output string.
                    let state = self
                        .state
                        .get(*repo)
                        .expect("failed to get repo value from state")
                        .get(full_name)
                        .expect("failed to get state for remote");
                    line.push_str(&format!(" {}", self.style_for_state(state).paint(name),));

                    // Store the location and string we just painted.
                    let x = w - ((remaining - 2) as u16);
                    self.locations.insert(
                        (repo, Some((*full_name).to_owned())),
                        (x, y, name.to_owned()),
                    );

                    // Reduce the remaining characters by the number of characters that we just
                    // drew into the line.
                    remaining -= n + 1;
                }

                if needs_ellipsis {
                    write!(self.t, "{}\u{2026}", cursor::Goto(w, y))
                        .expect("failed to write content to the terminal");
                }

                // Finally! Write the line to the terminal.
                write!(self.t, "{}{}", cursor::Goto(1, y), line)
                    .expect("failed to write content to the terminal");
            }

            if self.canceled {
                let mut message = "pending fetches canceled; allowing in-flight fetches to finish \
                                   (hit Ctrl-c again to terminate unsafely)"
                    .to_string();
                if message.len() > w_usize {
                    message.truncate(w_usize - 1);
                    message.push_str("\u{2026}");
                }
                write!(
                    self.t,
                    "{}{}",
                    cursor::Goto(1, y + 1),
                    Color::Red.bold().paint(message)
                )
                .expect("failed to write content to the terminal");
            }
        }
        self.drawn = (w, h);
        self.process_updates(results);
    }

    /// Processes updates in the queue, updating internal state and the UI as
    /// necessary.
    ///
    /// **This is an internal method and should not be called outside the
    /// impl.**
    fn process_updates(&mut self, results: &Results) {
        for &(repo, ref remote, ref state) in &self.updates {
            if let Some(&(x, y, ref s)) = self.locations.get(&(repo, Some(remote.to_owned()))) {
                let style = self.style_for_state(state);
                write!(self.t, "{}{}", cursor::Goto(x, y), style.paint(s.as_str()))
                    .expect("failed to write content to the terminal");
            }
            if let Some(&(x, y, ref s)) = self.locations.get(&(repo, None)) {
                let summary = results
                    .get(&repo)
                    .expect("failed to get repo from results cache");
                let style = style_for_kind(&summary.kind()).bold();
                write!(self.t, "{}{}", cursor::Goto(x, y), style.paint(s.as_str()))
                    .expect("failed to write content to the terminal");
            }
            self.state
                .get_mut(repo)
                .expect("failed to get repo value from state")
                .insert(remote.to_owned(), state.clone());
        }
        self.updates.clear();
        write!(self.t, "{}", cursor::Hide).expect("failed to write content to the terminal");
        self.t
            .flush()
            .expect("failed to flush content to the terminal");
    }

    /// Returns the appropriate style for the given `state`.
    ///
    /// **This is an internal method and should not be called outside the impl.**
    fn style_for_state(&self, state: &State) -> Style {
        match *state {
            State::Pending => Color::Blue.normal(),
            State::Canceled => Style::new().dimmed(),
            State::Fetching => Color::Cyan.normal(),
            State::NoChange => Style::new(),
            State::Success => Color::Green.normal(),
            State::Warning => Color::Yellow.normal(),
            State::Failure => Color::Red.normal(),
        }
    }
}
