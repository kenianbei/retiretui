//! Which rows the estimate reads and what each is: the latest twelve whole
//! months of them, each payee merged under its name and each row sorted by
//! the user's rules, the bank's own column, its transaction type, then the
//! embedded table.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::payee::starred_as_spaces;
use super::rules::Outcome;
use super::{Category, Patterns, Row};
use crate::plan::{Dollars, Plan, PlanDate};
use crate::project::inflation_factor;
use crate::tax::month_index;

const MONTHS_PER_YEAR: i32 = 12;

/// Whose transactions [`records`] lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Of {
    /// The rows sorted into a category.
    Category(Category),
    /// The rows of one payee nothing names, by its name.
    Payee(String),
    /// The rows of every payee nothing names.
    Unnamed,
    /// The rows the user's rules ignore.
    Ignored,
}

/// The latest twelve whole months the rows reach, as month indexes.
#[derive(Debug, Clone, Copy)]
pub(super) struct Window {
    pub(super) start: i32,
    pub(super) end: i32,
}

/// The month `on` falls in, as an index.
pub(super) fn month_of(on: PlanDate) -> i32 {
    month_index(on.year(), on.0.month())
}

impl Window {
    pub(super) fn of(rows: &[Row]) -> Option<Self> {
        let end = rows.iter().map(|row| month_of(row.on)).max()?;
        let earliest = rows.iter().map(|row| month_of(row.on)).min().unwrap_or(end);
        Some(Self {
            start: earliest.max(end - (MONTHS_PER_YEAR - 1)),
            end,
        })
    }

    pub(super) const fn months(self) -> i32 {
        self.end - self.start + 1
    }

    pub(super) fn holds(self, row: &Row) -> bool {
        (self.start..=self.end).contains(&month_of(row.on))
    }

    pub(super) fn annual(self, dollars: f64) -> Dollars {
        (dollars * f64::from(MONTHS_PER_YEAR) / f64::from(self.months())).round() as Dollars
    }
}

/// A payee as a row is sorted by it: its name, merged or normalized, the
/// name uppercased as rules match it, and what the embedded table says.
struct Named {
    name: String,
    key: String,
    embedded: Option<Outcome>,
}

/// Sorts rows, each payee normalized, merged and matched against the
/// embedded table once.
pub(super) struct Sorter<'a> {
    patterns: &'a Patterns,
    named: HashMap<&'a str, Named>,
}

impl<'a> Sorter<'a> {
    pub(super) fn new(patterns: &'a Patterns) -> Self {
        Self {
            patterns,
            named: HashMap::new(),
        }
    }

    /// The row's payee by name and what it is; none where nothing says.
    pub(super) fn sort(&mut self, row: &'a Row) -> (&str, Option<Outcome>) {
        let patterns = self.patterns;
        let named = self.named.entry(&row.payee).or_insert_with(|| {
            let name = patterns.name_of(&row.payee);
            let key = name.to_ascii_uppercase();
            let embedded = patterns.embedded_outcome(&key, row);
            Named {
                name,
                key,
                embedded,
            }
        });
        let outcome = patterns
            .user(&named.key, row)
            .or_else(|| {
                let banks = row.category.as_deref().and_then(Category::from_text);
                banks.or(row.kind).map(Outcome::Category)
            })
            .or(named.embedded);
        (&named.name, outcome)
    }
}

/// The rows within the estimate's window that are `of`'s, newest first.
#[must_use]
pub fn records<'a>(rows: &'a [Row], patterns: &Patterns, of: &Of) -> Vec<&'a Row> {
    let Some(window) = Window::of(rows) else {
        return Vec::new();
    };
    let mut sorter = Sorter::new(patterns);
    let mut is_of = |row: &'a Row| match (of, sorter.sort(row)) {
        (Of::Category(wanted), (_, outcome)) => outcome == Some(Outcome::Category(*wanted)),
        (Of::Payee(wanted), (payee, None)) => payee == wanted,
        (Of::Payee(_), (_, Some(_))) => false,
        (Of::Unnamed, (_, outcome)) => outcome.is_none(),
        (Of::Ignored, (_, outcome)) => outcome == Some(Outcome::Ignore),
    };
    let mut found: Vec<&Row> = rows
        .iter()
        .filter(|row| window.holds(row) && is_of(row))
        .collect();
    found.sort_by_key(|row| std::cmp::Reverse(row.on));
    found
}

/// How far a rule matching payees that contain some words reaches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Reach {
    /// How many payees, by the names they are listed under.
    pub payees: usize,
    /// What they spent a year, in the plan's start-year dollars.
    pub spent: Dollars,
}

/// How far a `match` on `part` reaches among `rows` in the estimate's
/// window, payees named as `patterns` name them.
#[must_use]
pub fn reach(plan: &Plan, rows: &[Row], patterns: &Patterns, part: &str) -> Reach {
    let part = starred_as_spaces(&part.trim().to_ascii_uppercase());
    let Some(window) = Window::of(rows).filter(|_| !part.is_empty()) else {
        return Reach::default();
    };
    let mut names: HashMap<&str, String> = HashMap::new();
    let mut payees = HashSet::new();
    let mut spent = 0.0;
    for row in rows.iter().filter(|row| window.holds(row)) {
        let name = names
            .entry(&row.payee)
            .or_insert_with(|| patterns.name_of(&row.payee).to_ascii_uppercase());
        if name.contains(part.as_str()) {
            payees.insert(name.clone());
            spent += row.spent().dollars() / inflation_factor(plan, row.on.year());
        }
    }
    Reach {
        payees: payees.len(),
        spent: window.annual(spent),
    }
}
