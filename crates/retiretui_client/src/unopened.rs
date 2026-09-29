//! A document that did not resolve, said in words: which file, where in
//! it, and why.

use retiretui_engine::plan::PlanError;
use retiretui_engine::plan::resolve::{ResolveCause, ResolveError};

use crate::session::file_name;

/// Where in a file's text a failure is, both counted from 1; the column in
/// characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// The line.
    pub line: usize,
    /// The character on it.
    pub column: usize,
}

/// Where `error` is in its file's text, where that is a file someone wrote.
#[must_use]
pub fn position(error: &ResolveError) -> Option<Position> {
    let ResolveCause::Parse { error, text } = &error.cause else {
        return None;
    };
    let before = text.get(..error.span()?.start)?;
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    Some(Position {
        line: before.matches('\n').count() + 1,
        column: before[line_start..].chars().count() + 1,
    })
}

/// The failing file's text, where the failure is in it.
#[must_use]
pub fn text(error: &ResolveError) -> Option<&str> {
    match &error.cause {
        ResolveCause::Parse { text, .. } => Some(text),
        _ => None,
    }
}

/// `error` in a line: the file by its name, where in it, and why.
#[must_use]
pub fn said(error: &ResolveError) -> String {
    let name = file_name(&error.file);
    match &error.cause {
        ResolveCause::Parse { error: parsed, .. } => {
            let at = position(error).map_or_else(String::new, |Position { line, column }| {
                format!(", line {line}, column {column}")
            });
            format!("{name}{at}: {}", first(parsed.message()))
        }
        ResolveCause::Scenario(scenario) => format!("{name}: {scenario}"),
        ResolveCause::Merged(merged) => {
            let reason = match merged.as_ref() {
                PlanError::Parse(parsed) => first(parsed.message()).to_owned(),
                PlanError::Serialize(written) => written.to_string(),
            };
            format!("{name}, with its scenarios applied: {reason}")
        }
        ResolveCause::Invalid(issues) => {
            let listing: Vec<&str> = issues.iter().map(|issue| issue.message.as_str()).collect();
            format!("{name}: {}", listing.join("; "))
        }
        ResolveCause::Unread(_) => format!("{name} could not be read"),
        ResolveCause::Cycle => format!("{name}'s scenario chain loops back on itself"),
    }
}

fn first(message: &str) -> &str {
    message.lines().next().unwrap_or_default().trim()
}

#[cfg(test)]
mod tests;
