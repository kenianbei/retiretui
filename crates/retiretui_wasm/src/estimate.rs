//! The Spending Estimate tool for a page: a bank's exports held for the
//! session, what they say the draft spends as proposals for its expenses,
//! each proposal or unnamed payee edited in its form, and the proposals
//! taken into the draft or written as a scenario.

use retiretui_client::estimate::{
    ABOUT, COLUMNS, EXTENSIONS, Estimator, Listed, NOTHING_READ, PROPOSAL_FIELDS, ProposalAnswers,
    UNKNOWN_FIELDS, UnknownAnswers, listed, overlay, summary, take_question, taken,
};
use retiretui_client::forms::Form;
use retiretui_client::forms::details::ReadRow;
use retiretui_client::forms::edit::Entry;
use retiretui_client::present::{MoneyForm, counted};
use retiretui_engine::estimate::{Patterns, apply};
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::edits::JsEditor;
use crate::{JsDocument, refused, to_js};

static PROPOSAL_FORM: Form = Form::tool::<ProposalAnswers>("Proposal", PROPOSAL_FIELDS);
static PAYEE_FORM: Form = Form::tool::<UnknownAnswers>("Payee", UNKNOWN_FIELDS);
const NO_ROW: &str = "nothing highlighted";

/// One row of the estimate: a proposal, or a payee nothing named.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateRow {
    /// Whether the row is a proposal rather than an unnamed payee.
    pub is_proposal: bool,
    /// The row under the columns.
    pub cells: [String; 4],
}

/// What the exports say the draft spends.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateView {
    /// Each export's name and how many rows it holds, in words.
    pub files: Vec<(String, String)>,
    /// The window read, its income and transfers, or what to add.
    pub summary: String,
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
    /// The columns the rows are tabled under.
    pub columns: [&'static str; 4],
    /// What the file dialog accepts: `.csv,.ofx,.qfx`.
    pub accept: String,
}

/// What `estimator` says `document`'s draft spends, estimated again where
/// anything moved.
fn view(estimator: &mut Estimator, document: &Document) -> EstimateView {
    estimator.refresh(&document.draft().plan);
    let found = estimator.estimate();
    let files = estimator.exports().listed();
    EstimateView {
        files: files
            .map(|(name, count)| (name.to_owned(), counted(count, "row", "rows")))
            .collect(),
        summary: summary(found, MoneyForm::Full),
        rows: listed(found)
            .iter()
            .map(|row| EstimateRow {
                is_proposal: matches!(row, Listed::Proposal(_)),
                cells: row.cells(found.months, MoneyForm::Full),
            })
            .collect(),
        question: (!found.proposals.is_empty()).then(|| take_question(found)),
    }
}

/// The row at `at` read out, its label beside what it says.
fn read(estimator: &Estimator, document: &Document, at: usize) -> Vec<ReadRow> {
    let Some(row) = estimator.row(at) else {
        return Vec::new();
    };
    let months = estimator.estimate().months;
    row.read_out(&document.draft().plan, months, MoneyForm::Full)
        .into_iter()
        .map(|(label, text)| ReadRow {
            label,
            text,
            is_unstated: false,
            group: None,
        })
        .collect()
}

/// The row at `at` open in its form - a proposal's, or a payee's - its
/// answers held beside `document`'s draft.
fn open_row(estimator: &Estimator, document: &mut Document, at: usize) -> Result<Entry, String> {
    let (row, answers) = estimator.row(at).zip(estimator.answers(at)).ok_or(NO_ROW)?;
    let form = match row {
        Listed::Proposal(_) => {
            document.hold_answers::<ProposalAnswers>(answers);
            PROPOSAL_FORM
        }
        Listed::Unknown(_) => {
            document.hold_answers::<UnknownAnswers>(answers);
            PAYEE_FORM
        }
    };
    Ok(Entry::open(form, document.draft(), Some(0)))
}

/// The exports a page holds for its session, and what they say the draft
/// spends.
#[wasm_bindgen(js_name = Estimator)]
#[derive(Debug)]
pub struct JsEstimator(Estimator);

impl Default for JsEstimator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = Estimator)]
impl JsEstimator {
    /// Holding nothing yet, with the embedded patterns.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self(Estimator::new(Patterns::embedded()))
    }

    /// Reads the export `text`, named `name`, in place of one of the same
    /// name, answering how many rows it holds.
    ///
    /// # Errors
    ///
    /// Where the export is of an unknown kind or does not read.
    #[wasm_bindgen(js_name = addExport)]
    pub fn add_export(&mut self, name: &str, text: &str) -> Result<usize, JsError> {
        self.0
            .add(name, text)
            .map_err(|error| refused(error.to_string()))
    }

    /// Drops the export named `name`.
    #[wasm_bindgen(js_name = removeExport)]
    pub fn remove_export(&mut self, name: &str) {
        self.0.remove(name);
    }

    /// What the exports say `document`'s draft spends, estimated again
    /// where the draft, the exports or the patterns moved since.
    ///
    /// # Errors
    ///
    /// Where the view does not convert.
    #[wasm_bindgen(unchecked_return_type = "EstimateView")]
    pub fn estimate(&mut self, document: &JsDocument) -> Result<JsValue, JsError> {
        to_js(&view(&mut self.0, &document.0))
    }

    /// The row at `at` read out: its label beside what it says.
    ///
    /// # Errors
    ///
    /// Where the rows do not convert.
    #[wasm_bindgen(js_name = readRow, unchecked_return_type = "ReadRow[]")]
    pub fn read_row(&self, document: &JsDocument, at: usize) -> Result<JsValue, JsError> {
        to_js(&read(&self.0, &document.0, at))
    }

    /// The row at `at` open in its form - a proposal's, or a payee's -
    /// its answers held beside `document`'s draft.
    ///
    /// # Errors
    ///
    /// Where there is no row at `at`.
    #[wasm_bindgen(js_name = openRow)]
    pub fn open_row(&self, document: &mut JsDocument, at: usize) -> Result<JsEditor, JsError> {
        open_row(&self.0, &mut document.0, at)
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
        document.0.hold(&mut editor.0).map_err(refused)?;
        self.0.apply(at, document.0.draft()).map_err(refused)
    }

    /// Takes the proposals into `document`'s draft as one step of history,
    /// answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take(&self, document: &mut JsDocument) -> Result<String, JsError> {
        let found = self.0.estimate();
        document
            .0
            .step(|plan| {
                apply(plan, found);
                Ok(taken(found))
            })
            .map_err(refused)
    }

    /// The proposals applied as a scenario to be written at `out`, its
    /// `base` the document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, or the scenario does not serialize.
    pub fn scenario(&self, document: &JsDocument, out: &str) -> Result<String, JsError> {
        let base = document.0.scenario_base(out).map_err(refused)?;
        overlay(&base, &document.0.draft().plan, self.0.estimate()).map_err(refused)
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
        columns: COLUMNS,
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
    fn exports_are_viewed_read_out_and_their_rows_edited() {
        let mut document = opened();
        let mut estimator = Estimator::new(Patterns::embedded());
        assert_eq!(view(&mut estimator, &document).summary, NOTHING_READ);
        assert_eq!(estimator.add("chase.csv", CSV).expect("reads"), 5);
        let shown = view(&mut estimator, &document);
        assert_eq!(shown.files, [("chase.csv".to_owned(), "5 rows".to_owned())]);
        assert_eq!(shown.rows.len(), 1, "{shown:?}");
        assert_eq!(shown.rows[0].cells[0], "Groceries");
        assert!(shown.rows[0].is_proposal && shown.question.is_some());
        assert_eq!(read(&estimator, &document, 0)[0].label, "Category");
        assert!(read(&estimator, &document, 1).is_empty());
        let mut editor = open_row(&estimator, &mut document, 0).expect("opens");
        editor.set("amount", None, "$2,400").expect("a field");
        document.hold(&mut editor).expect("held");
        estimator.apply(0, document.draft()).expect("applied");
        view(&mut estimator, &document);
        assert_eq!(estimator.estimate().proposals[0].amount, 2400, "kept");
        assert!(!document.draft().is_dirty());
        assert!(open_row(&estimator, &mut document, 1).is_err());
    }
}
