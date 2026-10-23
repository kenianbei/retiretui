//! The records a selection of the estimate holds, as the command line and
//! the agents' tools reply them: each with where it was sorted and what
//! decided it.

use retiretui_engine::estimate::{
    By, Cents, Of, Outcome, Patterns, Sorted, sorted_records, spent_a_year,
};
use retiretui_engine::plan::{Dollars, Plan};
use schemars::JsonSchema;
use serde::Serialize;

use super::Exports;
use super::behind::{records_title, spent_cents};
use crate::present::{MoneyForm, counted};

const UNNAMED: &str = "unnamed";
const IGNORED: &str = "ignored";
const BUCKET: &str = "bucket";

/// The records a selection holds.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RecordsReply {
    /// What they are: `Records: Groceries`.
    pub title: String,
    /// How many there are.
    pub count: usize,
    /// What they spend a year in the plan's start-year dollars, a credit
    /// negative.
    pub per_year: Dollars,
    /// Each, newest first.
    pub records: Vec<RecordReply>,
}

/// One record.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RecordReply {
    /// Its date.
    pub on: String,
    /// Its payee, as the bank wrote it.
    pub payee: String,
    /// Its payee, as the rules name it.
    pub named: String,
    /// What it spent, in dollars, a credit negative.
    pub spent: f64,
    /// Where it was sorted: a category's slug, `bucket`, `ignored`, or
    /// `unnamed` where nothing sorted it.
    pub sorted: String,
    /// What decided it; none where nothing did.
    pub by: Option<By>,
    /// The export it was read from.
    pub export: String,
    /// The bank's own category, where the export has one.
    pub bank_category: Option<String>,
}

impl Exports {
    /// The records `of` selects within the estimate's window, sorted by
    /// `patterns`, what they spend a year in `plan`'s dollars.
    #[must_use]
    pub fn records(&self, plan: &Plan, patterns: &Patterns, of: &Of) -> RecordsReply {
        let found = sorted_records(self.rows(), patterns, of);
        RecordsReply {
            title: records_title(of),
            count: found.len(),
            per_year: spent_a_year(plan, self.rows(), &found),
            records: found.into_iter().map(record).collect(),
        }
    }
}

fn record(sorted: Sorted) -> RecordReply {
    let row = sorted.row;
    RecordReply {
        on: row.on.0.to_string(),
        payee: row.payee.clone(),
        named: sorted.name,
        spent: row.spent().dollars(),
        sorted: match sorted.outcome {
            Some(Outcome::Category(category)) => category.slug(),
            Some(Outcome::Ignore) => IGNORED,
            Some(Outcome::Bucket) => BUCKET,
            None => UNNAMED,
        }
        .to_owned(),
        by: sorted.by,
        export: row.export.clone(),
        bank_category: row.category.clone(),
    }
}

impl RecordsReply {
    /// Its heading: `Records: Groceries - 41 records, $6,120 a year`.
    #[must_use]
    pub fn heading(&self) -> String {
        format!(
            "{} - {}, {} a year",
            self.title,
            counted(self.count, "record", "records"),
            MoneyForm::Full.money(self.per_year)
        )
    }
}

impl RecordReply {
    /// Its date, payee as the bank wrote it, what it spent, where it was
    /// sorted and what decided it, as a table's cells.
    #[must_use]
    pub fn cells(&self) -> [String; 5] {
        [
            self.on.clone(),
            self.payee.clone(),
            spent_cents(Cents::from_dollars(self.spent)),
            self.sorted.clone(),
            said_by(self.by.as_ref()),
        ]
    }
}

/// What decided a sort, in words: `rule 12 in patterns.toml`, `the bank's
/// Groceries`, `the transaction type`, `embedded "ATT "`.
fn said_by(by: Option<&By>) -> String {
    match by {
        Some(By::Rule { file, at }) => format!("rule {at} in {file}"),
        Some(By::Bank(category)) => format!("the bank's {category}"),
        Some(By::Type) => "the transaction type".to_owned(),
        Some(By::Embedded(words)) => format!("embedded \"{words}\""),
        None => String::new(),
    }
}
