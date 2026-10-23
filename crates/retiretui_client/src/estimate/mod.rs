//! The spending estimate as every surface says it: the exports a session
//! holds, each proposed expense with the categories it gathers and each
//! payee nothing names as a row and read out, the forms each kind of row
//! and the plan's `[estimate]` table are edited in, what is asked and said
//! when the proposals are taken, and the shapes they and the records behind
//! them are replied in.

mod act;
mod behind;
mod forms;
mod records;
mod rows;
mod rules;
#[cfg(test)]
mod tests;

use retiretui_engine::estimate::{
    BankError, Estimate, Patterns, Row, apply, estimate, merge, read,
};
use retiretui_engine::plan::{self, Plan, Scenario};
use schemars::JsonSchema;
use serde::Serialize;
use toml::Table;

pub use act::{Action, reached, record_menu, starting_words};
pub use behind::Behind;
pub use forms::{RowForm, SETTINGS_FORM, assign};
pub use records::{RecordReply, RecordsReply};
pub use rows::{Listed, listed, summary, take_question, taken};
pub use rules::{Change, Record, To};

use crate::draft::Draft;
use crate::present::MoneyForm;

/// What the tool is for, in a line.
pub const ABOUT: &str = "The plan's expenses from what a bank's exports say the household spent.";
/// What the table says before any export is read.
pub const NOTHING_READ: &str =
    "Add a bank's CSV, OFX or QFX export and the plan's spending is estimated from it.";
/// The columns the rows are tabled under.
pub const COLUMNS: [&str; 5] = ["Expense", "Now", "Proposed", "Change", "Why"];
/// The extensions an export may have.
pub const EXTENSIONS: [&str; 3] = ["csv", "ofx", "qfx"];
/// The name of the user's own spending patterns file.
pub const PATTERNS_FILE: &str = "patterns.toml";
/// What a payee's category reads as while nothing names it.
pub const UNKNOWN: &str = "Unknown";
const NO_ROW: &str = "nothing to edit on this row";

/// The exports a session holds, each by its name, read once, and their
/// rows merged as each is added or dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exports {
    files: Vec<(String, Vec<Row>)>,
    merged: Vec<Row>,
}

impl Exports {
    /// Reads `text` as the export `name`, in place of one of the same name.
    ///
    /// # Errors
    ///
    /// As the file fails to read, by name.
    pub fn add(&mut self, name: &str, text: &str) -> Result<usize, BankError> {
        let rows = read(name, text)?;
        let count = rows.len();
        self.files.retain(|(held, _)| held != name);
        self.files.push((name.to_owned(), rows));
        self.merge();
        Ok(count)
    }

    /// Drops the export `name`.
    pub fn remove(&mut self, name: &str) {
        self.files.retain(|(held, _)| held != name);
        self.merge();
    }

    fn merge(&mut self) {
        self.merged.clear();
        for (_, file) in &self.files {
            merge(&mut self.merged, file.clone());
        }
    }

    /// Each export's name and how many rows it holds, in the order added.
    pub fn listed(&self) -> impl Iterator<Item = (&str, usize)> {
        self.files
            .iter()
            .map(|(name, rows)| (name.as_str(), rows.len()))
    }

    /// Whether no export is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The exports' rows, each file's merged over the ones before it.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.merged
    }

    /// What the exports say `plan` spends.
    #[must_use]
    pub fn estimate(&self, plan: &Plan, patterns: &Patterns) -> Estimate {
        estimate(plan, &self.merged, patterns)
    }
}

/// What applying a row's form did, where a surface has more to do: a
/// category moved, which is the plan's to keep, or a payee named, which a
/// surface may remember.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// Nothing more: the estimate holds the edit, or there was none.
    Held,
    /// The user's rules changed, in the order made, which a surface may
    /// remember.
    Changed(Vec<Change>),
    /// The category goes to `expense`, or back to where it is placed.
    Moved {
        /// The category moved.
        category: retiretui_engine::estimate::Category,
        /// The expense, or none to drop the assignment.
        expense: Option<String>,
    },
}

/// What a session holds for the tool: the exports, the patterns that name
/// their payees, and what they say the plan last estimated over spends,
/// with whatever its proposals were edited to since.
#[derive(Debug, Clone)]
pub struct Estimator {
    exports: Exports,
    patterns: Patterns,
    estimate: Estimate,
    /// The plan last estimated over; none since the exports or the
    /// patterns moved.
    over: Option<Plan>,
}

impl Estimator {
    /// Holding no export, naming payees by `patterns`.
    #[must_use]
    pub fn new(patterns: Patterns) -> Self {
        Self {
            exports: Exports::default(),
            patterns,
            estimate: Estimate::default(),
            over: None,
        }
    }

    /// The exports held.
    #[must_use]
    pub const fn exports(&self) -> &Exports {
        &self.exports
    }

    /// The estimate as it stands.
    #[must_use]
    pub const fn estimate(&self) -> &Estimate {
        &self.estimate
    }

    /// Reads `text` as the export `name`, as [`Exports::add`] does.
    ///
    /// # Errors
    ///
    /// As the file fails to read, by name.
    pub fn add(&mut self, name: &str, text: &str) -> Result<usize, BankError> {
        let count = self.exports.add(name, text)?;
        self.over = None;
        Ok(count)
    }

    /// Drops the export `name`.
    pub fn remove(&mut self, name: &str) {
        self.exports.remove(name);
        self.over = None;
    }

    /// Estimates again over `plan` where it, the exports or the patterns
    /// moved since the last estimate, answering whether it did; edits to
    /// the proposals last until then.
    pub fn refresh(&mut self, plan: &Plan) -> bool {
        if self.over.as_ref() == Some(plan) {
            return false;
        }
        self.estimate = self.exports.estimate(plan, &self.patterns);
        self.over = Some(plan.clone());
        true
    }

    /// The row at `at` of [`listed`].
    #[must_use]
    pub fn row(&self, at: usize) -> Option<Listed<'_>> {
        listed(&self.estimate).nth(at)
    }

    /// What lies behind the row at `at`.
    #[must_use]
    pub fn behind(&self, at: usize, form: MoneyForm) -> Option<Behind> {
        let row = self.row(at)?;
        Some(row.behind(self.exports.rows(), &self.patterns, form))
    }

    /// The form the row at `at` is edited in, and the answers it opens
    /// on; none for a row nothing edits.
    #[must_use]
    pub fn form(&self, at: usize) -> Option<(RowForm, Table)> {
        forms::opened(self.row(at)?)
    }

    /// Writes what `draft` holds as the row at `at`'s answers: into the
    /// proposal, or as a category moved or a payee named, which estimates
    /// again.
    ///
    /// # Errors
    ///
    /// Where nothing edits the row, or the schema's complaint about a
    /// value.
    pub fn apply(&mut self, at: usize, draft: &Draft) -> Result<Applied, String> {
        let row = self.row(at).ok_or(NO_ROW)?;
        if let Listed::Expense(proposal) = row {
            let id = proposal.id.clone();
            let proposal = self.estimate.proposals.iter_mut().find(|p| p.id == id);
            forms::hold(proposal.ok_or(NO_ROW)?, draft)?;
            return Ok(Applied::Held);
        }
        let applied = forms::applied(row, draft)?;
        if let Applied::Changed(changes) = &applied {
            for change in changes {
                self.make(change)?;
            }
        }
        Ok(applied)
    }

    /// Makes `change` to the rules the session holds, estimating again.
    ///
    /// # Errors
    ///
    /// Where the change cannot be made to them, in its words.
    fn make(&mut self, change: &Change) -> Result<(), String> {
        let text = change.applied(self.patterns.text())?;
        let mut patterns = Patterns::embedded();
        (patterns.add(PATTERNS_FILE, &text)).map_err(|error| error.to_string())?;
        self.patterns = patterns;
        self.over = None;
        Ok(())
    }
}

/// The overlay over `base` that makes `plan` into itself with the
/// proposals applied.
///
/// # Errors
///
/// Where the plan does not serialize.
pub fn overlay(base: &str, plan: &Plan, estimate: &Estimate) -> Result<String, String> {
    let mut applied = plan.clone();
    apply(&mut applied, estimate);
    let beneath = plan::to_table(plan).map_err(|e| e.to_string())?;
    let draft = plan::to_table(&applied).map_err(|e| e.to_string())?;
    let scenario = Scenario::over(base, &beneath, &draft, None).map_err(|issues| {
        issues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    scenario.to_toml_string().map_err(|e| e.to_string())
}

/// The estimate as `estimate` and `estimate_spending` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateReply {
    /// The estimate.
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub estimate: Estimate,
    /// The scenario written with the proposals applied, where one was.
    pub written: Option<String>,
}
