//! Which rows the estimate reads and what each is: the latest twelve whole
//! months of them, each payee merged under its name and each row sorted by
//! the user's rules, the bank's own column, its transaction type, then the
//! embedded table.

use std::collections::{HashMap, HashSet};

use super::rules::{By, Outcome, Part, Rule};
use super::{Category, Patterns, Row};
use crate::plan::{Dollars, Plan, PlanDate};
use crate::project::inflation_factor;
use crate::tax::month_index;

const MONTHS_PER_YEAR: i32 = 12;

/// Whose transactions [`sorted_records`] lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Of {
    /// The rows sorted into a category.
    Category(Category),
    /// The rows of one payee nothing names, by its name.
    Payee(String),
    /// The rows of every payee whose name, or whose text as the bank wrote
    /// it, contains the words, either case, wherever they were sorted.
    Containing(String),
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
/// name uppercased as rules match it, and the embedded table's rule for it.
struct Named<'a> {
    name: String,
    key: String,
    embedded: Option<&'a Rule>,
}

/// What a row is sorted as, and what decided it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Decided<'a> {
    pub(super) outcome: Outcome,
    by: &'a By,
}

impl<'a> Decided<'a> {
    fn rule(rule: &'a Rule) -> Self {
        Self {
            outcome: rule.outcome,
            by: &rule.by,
        }
    }
}

/// Sorts rows, each payee normalized, merged and matched against the
/// embedded table once.
pub(super) struct Sorter<'a> {
    patterns: &'a Patterns,
    named: HashMap<&'a str, Named<'a>>,
}

impl<'a> Sorter<'a> {
    pub(super) fn new(patterns: &'a Patterns) -> Self {
        Self {
            patterns,
            named: HashMap::new(),
        }
    }

    /// The row's payee by name, what it is and what decided it; none
    /// where nothing says.
    pub(super) fn sort(&mut self, row: &'a Row) -> (&str, Option<Decided<'a>>) {
        let patterns = self.patterns;
        let named = self.named.entry(&row.payee).or_insert_with(|| {
            let name = patterns.name_of(&row.payee);
            let key = name.to_ascii_uppercase();
            let embedded = patterns.embedded_rule(&key, row);
            Named {
                name,
                key,
                embedded,
            }
        });
        let decided = (patterns.user(&named.key, row).map(Decided::rule))
            .or_else(|| {
                let banks = row.category.as_deref().and_then(Category::from_text)?;
                Some(Decided {
                    outcome: Outcome::Category(banks),
                    by: &By::Bank,
                })
            })
            .or_else(|| {
                Some(Decided {
                    outcome: Outcome::Category(row.kind?),
                    by: &By::Type,
                })
            })
            .or_else(|| named.embedded.map(Decided::rule));
        (&named.name, decided)
    }
}

/// A row within the estimate's window, as the estimate sorts it.
#[derive(Debug, Clone, PartialEq)]
pub struct Sorted<'a> {
    /// The row.
    pub row: &'a Row,
    /// Its payee, as the rules name it.
    pub name: String,
    /// What it is sorted as and what decided it; none where nothing did.
    pub decided: Option<(Outcome, By)>,
}

/// The rows within the estimate's window that are `of`'s, newest first,
/// each with what it is sorted as and what decided it.
#[must_use]
pub fn sorted_records<'a>(rows: &'a [Row], patterns: &Patterns, of: &Of) -> Vec<Sorted<'a>> {
    let Some(window) = Window::of(rows) else {
        return Vec::new();
    };
    let wanted = match of {
        Of::Containing(words) => Part::new(words),
        _ => Part::new(""),
    };
    let mut sorter = Sorter::new(patterns);
    let mut found: Vec<Sorted> = Vec::new();
    for row in rows.iter().filter(|row| window.holds(row)) {
        let (name, decided) = sorter.sort(row);
        let outcome = decided.map(|decided| decided.outcome);
        let is_unnamed = matches!(outcome, None | Some(Outcome::Bucket));
        let is_of = match of {
            Of::Category(wanted) => outcome == Some(Outcome::Category(*wanted)),
            Of::Payee(wanted) => is_unnamed && name == wanted,
            Of::Containing(_) => [name, &row.payee]
                .iter()
                .any(|text| wanted.holds(&text.to_ascii_uppercase())),
            Of::Unnamed => is_unnamed,
            Of::Ignored => outcome == Some(Outcome::Ignore),
        };
        if is_of {
            found.push(Sorted {
                row,
                name: name.to_owned(),
                decided: decided.map(|decided| (decided.outcome, decided.by.clone())),
            });
        }
    }
    found.sort_by_key(|sorted| std::cmp::Reverse(sorted.row.on));
    found
}

/// What `found` spent a year in the plan's start-year dollars, over the
/// window `rows` reach.
#[must_use]
pub fn spent_a_year(plan: &Plan, rows: &[Row], found: &[Sorted]) -> Dollars {
    let Some(window) = Window::of(rows) else {
        return 0;
    };
    let spent: f64 = (found.iter())
        .map(|sorted| sorted.row.spent().dollars() / inflation_factor(plan, sorted.row.on.year()))
        .sum();
    window.annual(spent)
}

/// How far a rule matching payees that contain some words reaches.
#[derive(Debug, Clone, Copy, Default)]
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
    let part = Part::new(part.trim());
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
        if part.holds(name) {
            payees.insert(name.clone());
            spent += row.spent().dollars() / inflation_factor(plan, row.on.year());
        }
    }
    Reach {
        payees: payees.len(),
        spent: window.annual(spent),
    }
}
