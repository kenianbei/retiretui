//! What can be done with a record or a payee straight from the page: a
//! record ignored or counted again, a payee ignored, said to be for a
//! category or named, and every payee containing some words sorted alike.

use retiretui_engine::estimate::{Reach, Row, normalize, reach, records};

use serde::Deserialize;

use super::rows::BUCKET_MARK;
use super::{Change, Estimator, Listed, Record, To};
use crate::present::{MoneyForm, counted};

const NO_RECORD: &str = "no record there to ignore";
const NO_PAYEE: &str = "no payee here to act on";
const NO_WORDS: &str = "say which words the payees contain";
const BUCKET: &str = "the bucket";

/// What can be done with a record or a payee.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(rename = "RecordAction"))]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// The record ignored, or counted again behind the ignored row.
    IgnoreRecord,
    /// Every record of the payee ignored.
    IgnorePayee,
    /// Every record of the payee to a category or the bucket.
    PayeeIs(To),
    /// Every record of each payee containing the words to a category or
    /// the bucket.
    Containing(String, To),
    /// The payee listed under the name.
    Name(String),
}

/// How many of a payee's words a rule on its words starts from.
const STARTING_WORDS: usize = 2;

/// How far a rule on some words reaches, in words: `4 payees, $1,230 a
/// year`.
#[must_use]
pub fn reached(reach: Reach) -> String {
    format!(
        "{}, {} a year",
        counted(reach.payees, "payee", "payees"),
        MoneyForm::Full.money(reach.spent)
    )
}

/// The words a rule on payees like `payee` starts from: its first two.
#[must_use]
pub fn starting_words(payee: &str) -> String {
    let words: Vec<&str> = payee.split_whitespace().take(STARTING_WORDS).collect();
    words.join(" ")
}

/// What a record's menu offers, in its order: the record ignored or
/// counted again, its payee for a category, payees containing its words,
/// every record of the payee ignored, and the payee named.
#[must_use]
pub fn record_menu(payee: &str, is_ignored: bool) -> [String; 5] {
    let record = if is_ignored {
        "Count this record again"
    } else {
        "Ignore this record"
    };
    [
        record.to_owned(),
        format!("{payee} is…"),
        "Payees containing…".to_owned(),
        format!("Ignore every {payee} record"),
        format!("Name {payee}…"),
    ]
}

/// The payee an action is done to: as the rules name it, and as the
/// estimate normalized it before any merge.
struct Target {
    named: String,
    normalized: String,
}

impl Estimator {
    /// Does `action` to the record at `record` behind the row at `at`, or,
    /// with none, to the payee of the row at `at`, answering the change
    /// made.
    ///
    /// # Errors
    ///
    /// Where no record or payee lies there, the words are empty, or the
    /// change cannot be made.
    pub fn act(
        &mut self,
        at: usize,
        record: Option<usize>,
        action: Action,
    ) -> Result<Change, String> {
        let change = match action {
            Action::IgnoreRecord => self.ignoring(at, record.ok_or(NO_RECORD)?)?,
            Action::IgnorePayee => Change::IgnorePayee {
                payee: self.target(at, record)?.named,
            },
            Action::PayeeIs(to) => Change::Name {
                payee: self.target(at, record)?.named,
                to,
            },
            Action::Containing(part, to) => Change::Contains {
                part: Some(part.trim())
                    .filter(|part| !part.is_empty())
                    .ok_or(NO_WORDS)?
                    .to_owned(),
                to,
            },
            Action::Name(name) => Change::Merge {
                payee: self.target(at, record)?.normalized,
                name: name.trim().to_owned(),
            },
        };
        self.make(&change)?;
        Ok(change)
    }

    /// The payee an action at `at`, or at `record` behind it, is done
    /// to, as the rules name it.
    #[must_use]
    pub fn payee(&self, at: usize, record: Option<usize>) -> Option<String> {
        self.target(at, record).ok().map(|target| target.named)
    }

    /// What the bucket is called: its expense's name, marked as the
    /// bucket, where the estimate proposes one.
    #[must_use]
    pub fn bucket_name(&self) -> String {
        let bucket = self.estimate.proposals.iter().find(|p| p.is_bucket);
        bucket.map_or_else(|| BUCKET.to_owned(), |p| format!("{}{BUCKET_MARK}", p.name))
    }

    /// How far a rule on payees containing `part` reaches.
    #[must_use]
    pub fn reach(&self, part: &str) -> Reach {
        let plan = self.over.as_ref();
        plan.map_or_else(Reach::default, |plan| {
            reach(plan, self.exports.rows(), &self.patterns, part)
        })
    }

    /// The record at `index` behind the row at `at`, and the row.
    fn picked(&self, at: usize, index: usize) -> Option<(Listed<'_>, &Row)> {
        let row = self.row(at)?;
        let found = records(self.exports.rows(), &self.patterns, &row.of()?);
        Some((row, found.get(index)?))
    }

    /// The payee of the record at `record` behind the row at `at`, else of
    /// the row itself.
    fn target(&self, at: usize, record: Option<usize>) -> Result<Target, String> {
        if let Some(index) = record {
            let (_, picked) = self.picked(at, index).ok_or(NO_PAYEE)?;
            return Ok(Target {
                named: self.patterns.name_of(&picked.payee),
                normalized: normalize(&picked.payee),
            });
        }
        match self.row(at).ok_or(NO_PAYEE)? {
            Listed::Payee(unknown) => Ok(Target {
                named: unknown.payee.clone(),
                normalized: unknown.payee.clone(),
            }),
            _ => Err(NO_PAYEE.to_owned()),
        }
    }

    /// The record at `index` behind the row at `at` ignored, or counted
    /// again behind the ignored row.
    fn ignoring(&self, at: usize, index: usize) -> Result<Change, String> {
        let (row, picked) = self.picked(at, index).ok_or(NO_RECORD)?;
        let record = Record {
            payee: self.patterns.name_of(&picked.payee),
            on: picked.on,
            spent: picked.spent(),
        };
        Ok(if matches!(row, Listed::Ignored { .. }) {
            Change::Unignore(record)
        } else {
            Change::IgnoreRecord(record)
        })
    }
}
