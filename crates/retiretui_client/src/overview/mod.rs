//! What the Overview lists beside its verdict, as every surface lists it:
//! what needs attention and the plan's milestones, each row led by the
//! year it is about or by the item behind it.

mod attention;
mod milestones;

#[cfg(test)]
mod tests;

use crate::forms::DomainId;

pub use attention::{NOTHING, attention, failing_start, issue_rows};
pub use milestones::milestones;

/// What the Overview titles its lists.
pub const ATTENTION: &str = "Needs attention";
/// See [`ATTENTION`].
pub const MILESTONES: &str = "Milestones";

/// A domain, and the item of its table where it has one.
pub type Place = (DomainId, Option<usize>);

/// A row: what it says, the year it is about, and the item behind it.
#[derive(Clone, PartialEq, Debug)]
pub struct Row {
    /// What the row says, without its year.
    pub text: String,
    /// The year it is about, where it is about one.
    pub year: Option<i16>,
    /// The item behind it.
    pub place: Option<Place>,
}

impl Row {
    fn plain(text: String) -> Self {
        Self {
            text,
            year: None,
            place: None,
        }
    }

    fn dated(year: i16, text: String, place: Place) -> Self {
        Self {
            year: Some(year),
            place: Some(place),
            ..Self::plain(text)
        }
    }

    /// The row as a line, led by its year where it has one.
    #[must_use]
    pub fn line(&self) -> String {
        match self.year {
            Some(year) => format!("{year} {}", self.text),
            None => self.text.clone(),
        }
    }
}
