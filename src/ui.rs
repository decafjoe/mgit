//! Common UI components.
//!
//! "Show <summary of information> about <some or all repos>" is a
//! common UI pattern in mgit. This module has helpers for the summary
//! part.
//!
//! A summary is composed of a vec of notes. Each note has a short
//! message string and a severity. The severity indicates the
//! seriousness of the condition described by the message.
//!
//! Overall, the summary also has a "severity" (see
//! `Summary.severity()`), which is defined as the most serious of the
//! severities of its notes.

// ----- Severity -------------------------------------------------------------

/// Indicates the severity of a status note.
#[derive(Clone, PartialEq, PartialOrd)]
pub enum Severity {
    /// Informational (no action needed).
    Info,
    /// Local branch is ahead of remote (requires action, but not
    /// "bad").
    Notice,
    /// Uncommitted work or local's being behind or diverging from
    /// remote (requires action, "bad" situation).
    Warning,
}

// ----- Note -----------------------------------------------------------------

/// Represents a piece of information about repository status.
pub struct Note {
    /// Status message that will be displayed to the user.
    content: String,
    /// `Severity` of the information.
    severity: Severity,
}

impl Note {
    /// Creates and returns a new `Note`.
    pub fn new(severity: Severity, content: &str) -> Self {
        Self {
            content: content.to_owned(),
            severity: severity,
        }
    }

    /// Returns content of the note.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns severity of the note.
    pub fn severity(&self) -> &Severity {
        &self.severity
    }
}

// ----- Summary --------------------------------------------------------------

/// Represents the full status of the repository.
pub struct Summary {
    /// `Vec` of `Note` instances representing the status.
    notes: Vec<Note>,
}

impl Summary {
    /// Creates and returns a new `Summary` instance.
    pub fn new() -> Self {
        Self { notes: Vec::new() }
    }

    /// Returns a reference to the `Vec` of `Note`s for this instance.
    pub fn notes(&self) -> &Vec<Note> {
        &self.notes
    }

    /// Adds a new `Note` to this summary.
    pub fn add_note(&mut self, severity: Severity, content: &str) {
        self.notes.push(Note::new(severity, content));
    }

    /// Returns the most severe `Severity` of this summary's notes.
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
