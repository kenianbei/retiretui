//! What can be done with a record or a payee straight from the page: a
//! record ignored or counted again, a payee ignored, said to be for a
//! category or named, and every payee containing some words sorted alike.

use retiretui_engine::estimate::{Category, Reach, normalize, reach, records};

use super::{Change, Estimator, Listed, Record};

const NO_RECORD: &str = "no record there to ignore";
const NO_PAYEE: &str = "no payee here to act on";
const NO_WORDS: &str = "say which words the payees contain";

/// What can be done with a record or a payee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The record ignored, or counted again behind the ignored row.
    IgnoreRecord,
    /// Every record of the payee ignored.
    IgnorePayee,
    /// Every record of the payee for the category.
    PayeeIs(Category),
    /// Every record of each payee containing the words for the category.
    Containing(String, Category),
    /// The payee listed under the name.
    Name(String),
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
            Action::PayeeIs(category) => Change::Name {
                payee: self.target(at, record)?.named,
                category,
            },
            Action::Containing(part, category) => Change::Contains {
                part: Some(part.trim())
                    .filter(|part| !part.is_empty())
                    .ok_or(NO_WORDS)?
                    .to_owned(),
                category,
            },
            Action::Name(name) => Change::Merge {
                payee: self.target(at, record)?.normalized,
                name: name.trim().to_owned(),
            },
        };
        self.make(&change)?;
        Ok(change)
    }

    /// How far a rule on payees containing `part` reaches.
    #[must_use]
    pub fn reach(&self, part: &str) -> Reach {
        let plan = self.over.as_ref();
        plan.map_or_else(Reach::default, |plan| {
            reach(plan, self.exports.rows(), &self.patterns, part)
        })
    }

    /// The payee of the record at `record` behind the row at `at`, else of
    /// the row itself.
    fn target(&self, at: usize, record: Option<usize>) -> Result<Target, String> {
        let row = self.row(at).ok_or(NO_PAYEE)?;
        if let Some(index) = record {
            let of = row.of().ok_or(NO_PAYEE)?;
            let found = records(self.exports.rows(), &self.patterns, &of);
            let picked = found.get(index).ok_or(NO_PAYEE)?;
            return Ok(Target {
                named: self.patterns.name_of(&picked.payee),
                normalized: normalize(&picked.payee),
            });
        }
        match row {
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
        let row = self.row(at).ok_or(NO_RECORD)?;
        let of = row.of().ok_or(NO_RECORD)?;
        let found = records(self.exports.rows(), &self.patterns, &of);
        let picked = found.get(index).ok_or(NO_RECORD)?;
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
