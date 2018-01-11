//! Common UI components.
use std::iter::Iterator;

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
        self.message.as_str()
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
