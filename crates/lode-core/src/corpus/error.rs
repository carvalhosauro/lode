//! In-memory corpus evaluation errors (no I/O or parse failures).

use std::fmt;

/// Failure while evaluating a loaded [`super::CorpusInput`].
#[derive(Debug)]
pub enum CorpusError {
    MissingLabel { format: String, line: usize },
    UnknownTemplateGid { format: String, gid: String },
    DeterminismMismatch { detail: String },
}

impl fmt::Display for CorpusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLabel { format, line } => {
                write!(f, "format {format}: missing label for line {line}")
            }
            Self::UnknownTemplateGid { format, gid } => {
                write!(f, "format {format}: unknown template_gid {gid:?}")
            }
            Self::DeterminismMismatch { detail } => write!(f, "determinism check failed: {detail}"),
        }
    }
}

impl std::error::Error for CorpusError {}
