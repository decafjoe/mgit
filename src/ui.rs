//! Common UI components.
use std::iter::Iterator;

use git2::{Branch, BranchType, Repository};

use app::Error;

// ----- Kind ---------------------------------------------------------------

/// Generic indicator for "result" or "status."
#[derive(Clone, PartialEq, PartialOrd)]
pub enum Kind {
    /// Nothing notable.
    None,
    /// Something good.
    Success,
    /// Bad but not really bad.
    Warning,
    /// Bad.
    Failure,
}

// ----- Note -----------------------------------------------------------------

/// Represents an item in a `Summary`.
pub struct Note {
    /// Group for the note, used during `Summary` sort.
    group: usize,
    /// `Kind` of note.
    kind: Kind,
    /// Message for the end user.
    message: String,
}

impl Note {
    /// Creates and returns a new `Note`.
    pub fn new(group: usize, kind: Kind, message: &str) -> Self {
        Self {
            group: group,
            kind: kind,
            message: message.to_owned(),
        }
    }

    /// Returns the group number for this note.
    fn group(&self) -> usize {
        self.group
    }

    /// Returns the kind for this note.
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns the message for this note.
    pub fn message(&self) -> &str {
        &self.message
    }
}

// ----- Iter -----------------------------------------------------------------

/// Iterator for a `Summary`.
///
/// Items are yielded in consistent order. They're first sorted by
/// `Note.group()`, then (for notes with equal groups) by
/// `Note.message()`.
pub struct Iter<'a> {
    /// Sorted vec of integer indices into the notes for the
    /// `Summary`.
    indices: Vec<usize>,
    /// Reference to the summary containing the notes to iterate
    /// through.
    summary: &'a Summary,
}

impl<'a> Iter<'a> {
    /// Creates and returns a new `Iter` instance.
    fn new(summary: &'a Summary) -> Self {
        let notes = summary.notes();
        let mut indices = (0..summary.notes().len()).collect::<Vec<usize>>();
        indices.sort_by_key(|i| (notes[*i].group(), notes[*i].message()));
        Self {
            indices: indices,
            summary: summary,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Note;

    /// Returns the next note (in order), or `None` if iteration is
    /// complete.
    fn next(&mut self) -> Option<Self::Item> {
        if self.indices.is_empty() {
            None
        } else {
            Some(&self.summary.notes()[self.indices.remove(0)])
        }
    }
}

// ----- Summary --------------------------------------------------------------

/// Represents a summary of current status or the results of an
/// operation.
pub struct Summary {
    /// Vec of notes comprising the summary.
    notes: Vec<Note>,
}

impl Summary {
    /// Creates and returns a new `Summary` instance.
    pub fn new() -> Self {
        Self { notes: Vec::new() }
    }

    /// Adds a new `Note` to this summary. Takes ownership of the
    /// `note` instance.
    pub fn push_note(&mut self, note: Note) {
        self.notes.push(note)
    }

    /// Returns a slice of `Note` references for this summary.
    fn notes(&self) -> &[Note] {
        self.notes.as_slice()
    }

    /// Returns an `Iter` for this summary, which yields notes in a
    /// stably-sorted order.
    ///
    /// See the documentation for `Iter` for more information.
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    /// Returns the "most severe" `Kind` of all the notes.
    pub fn kind(&self) -> Kind {
        let mut rv = Kind::None;
        for note in &self.notes {
            let kind = note.kind();
            if *kind > rv {
                rv = kind.clone();
            }
        }
        rv
    }
}

// ----- TrackingBranches -----------------------------------------------------

/// Convenience iterator for iterating through tracking branches.
///
/// The main feature of this struct is the validation done on
/// initialization. For each local branch this checks:
///
/// * That the branch has an upstream (if not, the branch will not be
///   yielded from the iterator)
/// * That the local branch has a (valid utf-8) name
/// * That we can get the local branch's oid
/// * That the upstream branch has a (valid utf-8) name
/// * That we can get the upstream branch's oid
///
/// As a result, for branches yielded from this iterator, it is safe
/// to unwrap the values returned by the git2 API for name and oid.
pub struct TrackingBranches<'a> {
    branches: Vec<Branch<'a>>,
}

impl<'a> TrackingBranches<'a> {
    /// Creates and returns a new `TrackingBranches` iterator for the
    /// repository `git`.
    pub fn for_repository(git: &'a Repository) -> Result<Self, Vec<Error>> {
        match TrackingBranches::get(git) {
            Ok(branches) => Ok(Self { branches: branches }),
            Err(e) => Err(e),
        }
    }

    /// Returns a vec of local `Branch` references that represent
    /// valid (per the description in the struct documentation) local
    /// branch references.
    fn get(git: &'a Repository) -> Result<Vec<Branch<'a>>, Vec<Error>> {
        let branches = match git.branches(Some(BranchType::Local)) {
            Ok(branches) => branches,
            Err(e) => {
                return Err(vec![
                    Error::new(&format!(
                        "failed to fetch local branch data ({})",
                        e
                    )),
                ]);
            }
        };
        let mut rv = Vec::new();
        let mut errors = Vec::new();
        for branch in branches {
            let local = match branch {
                Ok((local, _)) => local,
                Err(e) => {
                    errors.push(Error::new(&format!(
                        "failed to get info for local branch ({})",
                        e
                    )));
                    continue;
                }
            };
            {
                let local_name = match local.name() {
                    Ok(name) => if let Some(name) = name {
                        name
                    } else {
                        errors.push(Error::new(
                            "local branch name is not valid utf-8",
                        ));
                        continue;
                    },
                    Err(e) => {
                        errors.push(Error::new(&format!(
                            "failed to get name of local branch ({})",
                            e
                        )));
                        continue;
                    }
                };
                if local.get().target().is_none() {
                    errors.push(Error::new(&format!(
                        "failed to resolve oid for local branch {}",
                        local_name
                    )));
                    continue;
                }
                let upstream = if let Ok(upstream) = local.upstream() {
                    upstream
                } else {
                    // Assume there is no upstream branch (though
                    // technically this could be an actual error).
                    continue;
                };
                let upstream_name = match upstream.name() {
                    Ok(name) => if let Some(name) = name {
                        name
                    } else {
                        errors.push(Error::new(&format!(
                            "upstream branch name for local branch '{}' is \
                             not valid utf-8",
                            local_name
                        )));
                        continue;
                    },
                    Err(e) => {
                        errors.push(Error::new(&format!(
                            "failed to get name of upstream branch for local \
                             branch {} ({})",
                            local_name, e
                        )));
                        continue;
                    }
                };
                if upstream.get().target().is_none() {
                    errors.push(Error::new(&format!(
                        "failed to resolve oid for upstream branch {} (local \
                         branch is {})",
                        upstream_name, local_name
                    )));
                    continue;
                }
            }
            rv.push(local);
        }

        if errors.is_empty() {
            Ok(rv)
        } else {
            Err(errors)
        }
    }
}

impl<'a> Iterator for TrackingBranches<'a> {
    type Item = Branch<'a>;

    /// Returns the next local branch (if any) for this iterator.
    fn next(&mut self) -> Option<Self::Item> {
        if self.branches.is_empty() {
            None
        } else {
            Some(self.branches.remove(0))
        }
    }
}
