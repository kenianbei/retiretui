//! Why a document did not open, as the page shows it.

use retiretui_client::unopened::{Position, position, said, text};
use retiretui_engine::plan::resolve::ResolveError;
use serde::Serialize;

/// A document that did not open: the file that failed, said in a line,
/// and, where the failure is in its text, where.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OpenFailure {
    /// The file that failed, as the page's files are named: the one opened
    /// or a base in its chain.
    pub file: String,
    /// The failure in a line: the file by name, where, and why.
    pub headline: String,
    /// The line it is on, from 1; `null` where it is not in the text.
    pub line: Option<usize>,
    /// The column on that line, from 1, in characters.
    pub column: Option<usize>,
    /// The failing file's text, where the failure is in it.
    pub text: Option<String>,
}

impl From<&ResolveError> for OpenFailure {
    fn from(error: &ResolveError) -> Self {
        let at = position(error);
        Self {
            file: error.file.to_string_lossy().into_owned(),
            headline: said(error),
            line: at.map(|Position { line, .. }| line),
            column: at.map(|Position { column, .. }| column),
            text: text(error).map(str::to_owned),
        }
    }
}
