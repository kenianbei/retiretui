//! The Spending Estimate tool for a page: a bank's exports held for the
//! session, what they say the draft spends as proposals for its expenses,
//! each proposal or unnamed payee edited in its form, and the proposals
//! taken into the draft or written as a scenario.

use retiretui_client::estimate::{
    ABOUT, COLUMNS, EXTENSIONS, Exports, Listed, NOTHING_READ, PROPOSAL_FIELDS, ProposalAnswers,
    UNKNOWN_FIELDS, UnknownAnswers, category_from, listed, overlay, proposal_from, proposal_table,
    summary, take_question, taken,
};
use retiretui_client::forms::Form;
use retiretui_client::forms::details::ReadRow;
use retiretui_client::forms::edit::Entry;
use retiretui_client::present::MoneyForm;
use retiretui_engine::estimate::{Estimate, Patterns, apply};
use retiretui_engine::plan::Plan;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::edits::JsEditor;
use crate::{JsDocument, refused, to_js};

static PROPOSAL_FORM: Form = Form::tool::<ProposalAnswers>("Proposal", PROPOSAL_FIELDS);
static PAYEE_FORM: Form = Form::tool::<UnknownAnswers>("Payee", UNKNOWN_FIELDS);

/// One row of the estimate: a proposal, or a payee nothing named.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateRow {
    /// Whether the row is a proposal rather than an unnamed payee.
    pub is_proposal: bool,
    /// The row under the columns.
    pub cells: [String; 4],
    /// The row read out, in the form's words.
    pub read: Vec<ReadRow>,
}

/// What the exports say the draft spends.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateView {
    /// Each export's name and how many rows it holds.
    pub files: Vec<(String, usize)>,
    /// The window read, its income and transfers, or what to add.
    pub summary: String,
    /// The columns the rows are tabled under.
    pub columns: [&'static str; 4],
    /// The proposals, then the unnamed payees.
    pub rows: Vec<EstimateRow>,
    /// What is asked before the proposals are taken; none where there are
    /// none.
    pub question: Option<String>,
}

/// What the Spending Estimate tool says of itself.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateWords {
    /// What the tool is for, in a line.
    pub about: &'static str,
    /// What the table says before any export is read.
    pub nothing_read: &'static str,
    /// What the file dialog accepts: `.csv,.ofx,.qfx`.
    pub accept: String,
}

/// The exports a page holds for its session, and what they say the draft
/// spends.
#[derive(Debug)]
pub struct Estimator {
    exports: Exports,
    patterns: Patterns,
    estimate: Estimate,
    /// The plan last estimated over; none since the exports or patterns
    /// moved.
    over: Option<Plan>,
}

const NO_ROW: &str = "nothing highlighted";

impl Default for Estimator {
    fn default() -> Self {
        Self::new()
    }
}

impl Estimator {
    /// Holding nothing yet, with the embedded patterns.
    #[must_use]
    pub fn new() -> Self {
        Self {
            exports: Exports::default(),
            patterns: Patterns::embedded(),
            estimate: Estimate::default(),
            over: None,
        }
    }

    /// Reads the export `text`, named `name`, in place of one of the same
    /// name, answering how many rows it holds.
    ///
    /// # Errors
    ///
    /// Where the export is of an unknown kind or does not read.
    pub fn add_export(&mut self, name: &str, text: &str) -> Result<usize, String> {
        self.over = None;
        self.exports
            .add(name, text)
            .map_err(|error| error.to_string())
    }

    /// Drops the export named `name`.
    pub fn remove_export(&mut self, name: &str) {
        self.over = None;
        self.exports.remove(name);
    }

    /// What the exports say `plan` spends, estimated again where the plan,
    /// the exports or the patterns moved since; edits to the proposals
    /// last until then.
    pub fn estimate(&mut self, plan: &Plan) -> EstimateView {
        if self.over.as_ref() != Some(plan) {
            self.estimate = self.exports.estimate(plan, &self.patterns);
            self.over = Some(plan.clone());
        }
        self.view(plan)
    }

    /// The row at `at` open in its form - a proposal's, or a payee's -
    /// its answers held beside `document`'s draft.
    ///
    /// # Errors
    ///
    /// Where there is no row at `at`.
    pub fn open_row(&self, document: &mut Document, at: usize) -> Result<Entry, String> {
        let form = if let Some(proposal) = self.estimate.proposals.get(at) {
            document.hold_answers::<ProposalAnswers>(proposal_table(proposal));
            PROPOSAL_FORM
        } else if at < self.estimate.proposals.len() + self.estimate.unknown.len() {
            document.hold_answers::<UnknownAnswers>(toml::Table::new());
            PAYEE_FORM
        } else {
            return Err(NO_ROW.to_owned());
        };
        Ok(Entry::open(form, document.draft(), Some(0)))
    }

    /// Applies what `editor` holds to the row at `at`: a proposal's
    /// expense, name, amount and phases, or the category a payee's rows
    /// are for, which estimates again.
    ///
    /// # Errors
    ///
    /// Why it was not applied, in the form's words.
    pub fn apply_row(
        &mut self,
        document: &mut Document,
        editor: &mut Entry,
        at: usize,
    ) -> Result<(), String> {
        document.hold(editor)?;
        let draft = document.draft();
        if let Some(proposal) = self.estimate.proposals.get_mut(at) {
            return proposal_from(draft.answers::<ProposalAnswers>(), proposal);
        }
        let unknown = at.saturating_sub(self.estimate.proposals.len());
        let payee = self.estimate.unknown.get(unknown).ok_or(NO_ROW)?;
        if let Some(category) = category_from(draft.answers::<UnknownAnswers>())? {
            self.patterns.assign(&payee.payee, category);
            self.over = None;
        }
        Ok(())
    }

    /// Takes the proposals into `document`'s draft as one step of history,
    /// answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take(&self, document: &mut Document) -> Result<String, String> {
        let estimate = &self.estimate;
        document.step(|plan| {
            apply(plan, estimate);
            Ok(taken(estimate))
        })
    }

    /// The proposals applied as a scenario to be written at `out`, its
    /// `base` the document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, or the scenario does not serialize.
    pub fn scenario(&self, document: &Document, out: &str) -> Result<String, String> {
        let base = document.scenario_base(out)?;
        overlay(&base, &document.draft().plan, &self.estimate)
    }

    fn view(&self, plan: &Plan) -> EstimateView {
        let estimate = &self.estimate;
        let rows = listed(estimate)
            .iter()
            .map(|row| EstimateRow {
                is_proposal: matches!(row, Listed::Proposal(_)),
                cells: row.cells(estimate.months, MoneyForm::Full),
                read: row
                    .read_out(plan, estimate.months, MoneyForm::Full)
                    .into_iter()
                    .map(|(label, text)| ReadRow {
                        label,
                        text,
                        is_unstated: false,
                        group: None,
                    })
                    .collect(),
            })
            .collect();
        EstimateView {
            files: self
                .exports
                .listed()
                .map(|(name, count)| (name.to_owned(), count))
                .collect(),
            summary: if self.exports.is_empty() {
                NOTHING_READ.to_owned()
            } else {
                summary(estimate, MoneyForm::Full)
            },
            columns: COLUMNS,
            rows,
            question: (!estimate.proposals.is_empty()).then(|| take_question(estimate)),
        }
    }
}

/// The exports a page holds for its session, and what they say the draft
/// spends.
#[wasm_bindgen(js_name = Estimator)]
#[derive(Debug, Default)]
pub struct JsEstimator(Estimator);

#[wasm_bindgen(js_class = Estimator)]
impl JsEstimator {
    /// Holding nothing yet, with the embedded patterns.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self(Estimator::new())
    }

    /// Reads the export `text`, named `name`, in place of one of the same
    /// name, answering how many rows it holds.
    ///
    /// # Errors
    ///
    /// Where the export is of an unknown kind or does not read.
    #[wasm_bindgen(js_name = addExport)]
    pub fn add_export(&mut self, name: &str, text: &str) -> Result<usize, JsError> {
        self.0.add_export(name, text).map_err(refused)
    }

    /// Drops the export named `name`.
    #[wasm_bindgen(js_name = removeExport)]
    pub fn remove_export(&mut self, name: &str) {
        self.0.remove_export(name);
    }

    /// What the exports say `document`'s draft spends, estimated again
    /// where the draft, the exports or the patterns moved since.
    ///
    /// # Errors
    ///
    /// Where the view does not convert.
    #[wasm_bindgen(unchecked_return_type = "EstimateView")]
    pub fn estimate(&mut self, document: &JsDocument) -> Result<JsValue, JsError> {
        to_js(&self.0.estimate(&document.0.draft().plan))
    }

    /// The row at `at` open in its form - a proposal's, or a payee's -
    /// its answers held beside `document`'s draft.
    ///
    /// # Errors
    ///
    /// Where there is no row at `at`.
    #[wasm_bindgen(js_name = openRow)]
    pub fn open_row(&self, document: &mut JsDocument, at: usize) -> Result<JsEditor, JsError> {
        self.0
            .open_row(&mut document.0, at)
            .map(JsEditor)
            .map_err(refused)
    }

    /// Applies what `editor` holds to the row at `at`: a proposal's
    /// expense, name, amount and phases, or the category a payee's rows
    /// are for.
    ///
    /// # Errors
    ///
    /// Why it was not applied, in the form's words.
    #[wasm_bindgen(js_name = applyRow)]
    pub fn apply_row(
        &mut self,
        document: &mut JsDocument,
        editor: &mut JsEditor,
        at: usize,
    ) -> Result<(), JsError> {
        self.0
            .apply_row(&mut document.0, &mut editor.0, at)
            .map_err(refused)
    }

    /// Takes the proposals into `document`'s draft as one step of history,
    /// answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take(&self, document: &mut JsDocument) -> Result<String, JsError> {
        self.0.take(&mut document.0).map_err(refused)
    }

    /// The proposals applied as a scenario to be written at `out`, its
    /// `base` the document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, or the scenario does not serialize.
    pub fn scenario(&self, document: &JsDocument, out: &str) -> Result<String, JsError> {
        self.0.scenario(&document.0, out).map_err(refused)
    }
}

/// What the Spending Estimate tool says of itself.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = estimateWords, unchecked_return_type = "EstimateWords")]
pub fn estimate_words() -> Result<JsValue, JsError> {
    let accept: Vec<String> = EXTENSIONS.iter().map(|ext| format!(".{ext}")).collect();
    to_js(&EstimateWords {
        about: ABOUT,
        nothing_read: NOTHING_READ,
        accept: accept.join(","),
    })
}

#[cfg(test)]
mod tests {
    use retiretui_client::setup::examples::named;

    use super::*;

    const CSV: &str = include_str!("../../retiretui_engine/tests/fixtures/bank/chase.csv");

    fn opened() -> Document {
        let text = named("retired-couple.toml").expect("the example").2;
        let mut read = |_: &std::path::Path| Ok(text.to_owned());
        Document::open("/plan.toml", &mut read).expect("opens")
    }

    #[test]
    fn exports_are_read_and_estimated_once_per_draft_and_the_rows_edited() {
        let mut document = opened();
        let mut estimator = Estimator::new();
        assert_eq!(
            estimator.estimate(&document.draft().plan).summary,
            NOTHING_READ
        );
        assert_eq!(estimator.add_export("chase.csv", CSV).expect("reads"), 5);
        assert!(estimator.add_export("plan.toml", "x").is_err());
        let view = estimator.estimate(&document.draft().plan);
        assert_eq!(view.files, [("chase.csv".to_owned(), 5)]);
        assert_eq!(view.rows.len(), 1, "{view:?}");
        assert_eq!(view.rows[0].cells[0], "Groceries");
        assert!(view.rows[0].is_proposal);
        assert_eq!(view.rows[0].read[0].label, "Category");
        assert!(view.question.is_some());
        let mut editor = estimator.open_row(&mut document, 0).expect("opens");
        editor.set("amount", None, "$2,400").expect("a field");
        estimator
            .apply_row(&mut document, &mut editor, 0)
            .expect("applied");
        assert_eq!(estimator.estimate.proposals[0].amount, 2400);
        assert!(!document.draft().is_dirty());
        estimator.estimate(&document.draft().plan);
        assert_eq!(
            estimator.estimate.proposals[0].amount, 2400,
            "no re-estimate"
        );
        assert!(estimator.open_row(&mut document, 1).is_err());
    }

    #[test]
    fn the_proposals_are_taken_as_one_step_and_written_over_the_saved_file() {
        let mut document = opened();
        let mut estimator = Estimator::new();
        estimator.add_export("chase.csv", CSV).expect("reads");
        estimator.estimate(&document.draft().plan);
        let written = estimator
            .scenario(&document, "/estimated.toml")
            .expect("written");
        assert!(written.contains("base = \"plan.toml\""), "{written}");
        assert!(written.contains("id = \"groceries\""), "{written}");
        let said = estimator.take(&mut document).expect("taken");
        assert!(said.contains("1 added"), "{said}");
        assert!(document.draft().is_dirty());
        assert!(estimator.scenario(&document, "/estimated.toml").is_err());
        estimator.remove_export("chase.csv");
        let view = estimator.estimate(&document.draft().plan);
        assert!(view.rows.is_empty() && view.question.is_none());
    }
}
