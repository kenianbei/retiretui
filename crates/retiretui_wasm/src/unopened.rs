//! Why a document did not open, as the page shows it.

use retiretui_client::unopened::{position, said, text};
use retiretui_engine::plan::resolve::ResolveError;
use serde::Serialize;

/// A document that did not open: why, in a line, and the file that failed
/// where the failure is in its text.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OpenFailure {
    /// The failure in a line: the file by name, where, and why.
    pub headline: String,
    /// The file that failed, where the failure is in its text.
    pub written: Option<Written>,
}

/// A file as written, and the line a failure is on.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Written {
    /// The file, as the page's files are named: the one opened or a base
    /// in its chain.
    pub file: String,
    /// Its text.
    pub text: String,
    /// The line the failure is on, from 1, where the error knows.
    pub line: Option<usize>,
}

impl From<&ResolveError> for OpenFailure {
    fn from(error: &ResolveError) -> Self {
        let written = text(error).map(|text| Written {
            file: error.file.to_string_lossy().into_owned(),
            text: text.to_owned(),
            line: position(error).map(|at| at.line),
        });
        Self {
            headline: said(error),
            written,
        }
    }
}
