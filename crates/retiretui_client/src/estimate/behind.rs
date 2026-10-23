//! What lies behind a row of the estimate: an expense's payees with what
//! each adds a year, or the records of a category, a payee, the unnamed
//! payees or the ignored, each of which can be ignored or counted again.

use retiretui_engine::estimate::{CENTS_PER_DOLLAR, Cents, Of, Patterns, Proposal, Row, records};
use serde::Serialize;

use super::Listed;
use super::rows::{IGNORED, UNNAMED};
use crate::present::{MoneyForm, money};

/// The columns an expense's payees are tabled under.
const PAYEE_COLUMNS: [&str; 2] = ["Payee", "Per year"];
/// The columns a row's records are tabled under.
const RECORD_COLUMNS: [&str; 3] = ["Date", "Payee", "Amount"];

/// What lies behind a row.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Behind {
    /// What the records are, as their pane is headed.
    pub title: String,
    /// The columns they are tabled under.
    pub columns: Vec<String>,
    /// Each record's cells.
    pub rows: Vec<Vec<String>>,
    /// Whether the records are those ignored; none where they are an
    /// expense's payees rather than records.
    pub ignored: Option<bool>,
    /// Each record's payee, as the rules name it.
    pub payees: Vec<String>,
}

impl Listed<'_> {
    /// Whose records lie behind the row; none for an expense, behind
    /// which lie its payees.
    pub(super) fn of(&self) -> Option<Of> {
        Some(match self {
            Self::Expense(_) => return None,
            Self::Category { gathered, .. } => Of::Category(gathered.category),
            Self::Unnamed { .. } => Of::Unnamed,
            Self::Payee(unknown) => Of::Payee(unknown.payee.clone()),
            Self::Ignored { .. } => Of::Ignored,
        })
    }

    /// What lies behind the row, among `rows` as `patterns` sort them.
    #[must_use]
    pub fn behind(&self, rows: &[Row], patterns: &Patterns, form: MoneyForm) -> Behind {
        if let Self::Expense(proposal) = self {
            return payees_behind(proposal, form);
        }
        let Some(of) = self.of() else {
            return Behind::default();
        };
        let what = what(&of);
        let found = records(rows, patterns, &of);
        Behind {
            ignored: Some(of == Of::Ignored),
            payees: found
                .iter()
                .map(|row| patterns.name_of(&row.payee))
                .collect(),
            ..records_behind(&what, &found)
        }
    }
}

/// Whose records `of` selects, as their heading names them.
pub(super) fn what(of: &Of) -> String {
    match of {
        Of::Category(category) => category.title(),
        Of::Payee(payee) => payee.clone(),
        Of::Containing(words) => format!("payees containing {}", words.trim()),
        Of::Unnamed => UNNAMED.to_owned(),
        Of::Ignored => IGNORED.to_lowercase(),
    }
}

/// An expense's payees, each with what it adds a year.
fn payees_behind(proposal: &Proposal, form: MoneyForm) -> Behind {
    Behind {
        title: format!("Payees: {}", proposal.name),
        columns: PAYEE_COLUMNS.map(str::to_owned).to_vec(),
        rows: proposal
            .payees
            .iter()
            .map(|(payee, amount)| vec![payee.clone(), form.money(*amount)])
            .collect(),
        ignored: None,
        payees: Vec::new(),
    }
}

/// `records` of `what`, each its date, its payee as the bank wrote it and
/// what it spent, a credit negative.
fn records_behind(what: &str, records: &[&Row]) -> Behind {
    Behind {
        title: format!("Records: {what}"),
        columns: RECORD_COLUMNS.map(str::to_owned).to_vec(),
        rows: records
            .iter()
            .map(|row| {
                vec![
                    row.on.0.to_string(),
                    row.payee.clone(),
                    spent_cents(row.spent()),
                ]
            })
            .collect(),
        ignored: None,
        payees: Vec::new(),
    }
}

/// What a row spent, in dollars and cents: `$42.10`, `-$5.00` for money in.
fn spent_cents(spent: Cents) -> String {
    let sign = if spent.0 < 0 { "-" } else { "" };
    let whole = spent.0.abs();
    format!(
        "{sign}{}.{:02}",
        money(whole / CENTS_PER_DOLLAR),
        whole % CENTS_PER_DOLLAR
    )
}
