//! The Spending Estimate tool for a page: a bank's exports held for the
//! session, what they say the draft spends as an expense per row with the
//! categories it gathers beneath it, each row and the plan's `[estimate]`
//! table edited in its form, and the proposals taken into the draft or
//! written as a scenario.

use retiretui_client::estimate::{
    ABOUT, Applied, COLUMNS, EXTENSIONS, Estimator, NOTHING_READ, SETTINGS_FORM, assign, listed,
    overlay, summary, take_question, taken,
};
use retiretui_client::forms::details::{self, ReadRow};
use retiretui_client::forms::edit::Entry;
use retiretui_client::present::{MoneyForm, counted};
use retiretui_engine::estimate::{Patterns, apply};
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::edits::JsEditor;
use crate::{JsDocument, refused, to_js};

const NOTHING_TO_EDIT: &str = "nothing to edit on this row";

/// One row of the estimate.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateRow {
    /// Whether it is one of those an expense above it gathers.
    pub is_nested: bool,
    /// Whether a form edits it.
    pub is_editable: bool,
    /// What it is, as its read-out is headed.
    pub title: &'static str,
    /// The row under the columns.
    pub cells: [String; 5],
}

/// What the exports say the draft spends.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateView {
    /// Each export's name and how many rows it holds, in words.
    pub files: Vec<(String, String)>,
    /// The window read, its spending, income and transfers, or what to add.
    pub summary: String,
    /// Each expense with what it gathers beneath it, the bucket's last the
    /// payees nothing names.
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
    pub columns: [&'static str; 5],
    /// What the file dialog accepts: `.csv,.ofx,.qfx`.
    pub accept: String,
}

/// What `estimator` says `document`'s draft spends, estimated again where
/// anything moved.
fn view(estimator: &mut Estimator, document: &Document) -> EstimateView {
    estimator.refresh(&document.draft().plan);
    let found = estimator.estimate();
    let files = estimator.exports().listed();
    let rows = listed(found)
        .map(|row| EstimateRow {
            is_nested: row.is_nested(),
            is_editable: row.is_editable(),
            title: row.title(),
            cells: row.cells(MoneyForm::Full),
        })
        .collect();
    EstimateView {
        files: files
            .map(|(name, count)| (name.to_owned(), counted(count, "row", "rows")))
            .collect(),
        summary: summary(found, MoneyForm::Full),
        rows,
        question: (!found.proposals.is_empty()).then(|| take_question(found)),
    }
}

/// Label-and-text rows as the page reads them out.
fn read_rows(rows: Vec<(String, String)>) -> Vec<ReadRow> {
    rows.into_iter()
        .map(|(label, text)| ReadRow {
            label,
            text,
            is_unstated: false,
            group: None,
        })
        .collect()
}

/// The row at `at` read out, its label beside what it says.
fn read(estimator: &Estimator, at: usize) -> Vec<ReadRow> {
    let Some(row) = estimator.row(at) else {
        return Vec::new();
    };
    read_rows(row.read_out(estimator.estimate().months, MoneyForm::Full))
}

/// The row at `at` open in its form, its answers held beside `document`'s
/// draft.
fn open_row(estimator: &Estimator, document: &mut Document, at: usize) -> Result<Entry, String> {
    let (row_form, answers) = estimator.form(at).ok_or(NOTHING_TO_EDIT)?;
    let form = row_form.form();
    document.hold_form(&form, answers)?;
    Ok(Entry::open(form, document.draft(), Some(0)))
}

/// Applies what `editor` holds to the row at `at`: into the proposal, a
/// moved category into the plan as a step of its history, or a payee
/// named, merged or ignored for the session.
fn apply_row(
    estimator: &mut Estimator,
    document: &mut Document,
    editor: &mut Entry,
    at: usize,
) -> Result<(), String> {
    document.hold(editor)?;
    if let Applied::Moved { category, expense } = estimator.apply(at, document.draft())? {
        document.step(|plan| {
            assign(plan, category, expense);
            Ok(())
        })?;
    }
    Ok(())
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
    pub fn read_row(&self, at: usize) -> Result<JsValue, JsError> {
        to_js(&read(&self.0, at))
    }

    /// What lies behind the row at `at`: an expense's payees, or the
    /// transactions of a category, a payee or the unnamed payees.
    ///
    /// # Errors
    ///
    /// Where the records do not convert.
    #[wasm_bindgen(js_name = behindRow, unchecked_return_type = "Behind")]
    pub fn behind_row(&self, at: usize) -> Result<JsValue, JsError> {
        to_js(&self.0.behind(at, MoneyForm::Full).unwrap_or_default())
    }

    /// Ignores the record at `index` behind the row at `at` for the
    /// session, or counts it again behind the ignored row, answering what
    /// was done.
    ///
    /// # Errors
    ///
    /// Where no record lies there, or the change cannot be made.
    #[wasm_bindgen(js_name = toggleRecord)]
    pub fn toggle_record(&mut self, at: usize, index: usize) -> Result<String, JsError> {
        self.0
            .toggle_record(at, index)
            .map(|change| change.said())
            .map_err(refused)
    }

    /// The row at `at` open in its form, its answers held beside
    /// `document`'s draft.
    ///
    /// # Errors
    ///
    /// Where nothing edits the row.
    #[wasm_bindgen(js_name = openRow)]
    pub fn open_row(&self, document: &mut JsDocument, at: usize) -> Result<JsEditor, JsError> {
        open_row(&self.0, &mut document.0, at)
            .map(JsEditor)
            .map_err(refused)
    }

    /// Applies what `editor` holds to the row at `at`.
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
        apply_row(&mut self.0, &mut document.0, &mut editor.0, at).map_err(refused)
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

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The plan's `[estimate]` table read out: each label beside what it
    /// says.
    ///
    /// # Errors
    ///
    /// Where the rows do not convert.
    #[wasm_bindgen(js_name = estimateSettingsRead, unchecked_return_type = "ReadRow[]")]
    pub fn estimate_settings_read(&self) -> Result<JsValue, JsError> {
        let draft = self.0.draft();
        let item = (SETTINGS_FORM.item)(draft, 0).unwrap_or_default();
        to_js(&details::rows(&SETTINGS_FORM, &item, &draft.plan))
    }

    /// The plan's `[estimate]` table open in its form, applied as any
    /// item is.
    #[wasm_bindgen(js_name = estimateSettings)]
    #[must_use]
    pub fn estimate_settings(&self) -> JsEditor {
        JsEditor(Entry::open(SETTINGS_FORM, self.0.draft(), Some(0)))
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
        assert_eq!(shown.rows[0].cells[0], "Groceries (new)", "{shown:?}");
        assert!(shown.rows[1].is_nested && shown.rows[1].is_editable);
        assert_eq!(
            (shown.rows[0].title, shown.rows[1].title),
            ("Expense", "Category")
        );
        assert!(shown.question.is_some());
        assert_eq!(read(&estimator, 0)[0].label, "Expense");
        let said = estimator.toggle_record(1, 0).expect("a record").said();
        assert!(said.ends_with("is ignored"), "{said}");
        view(&mut estimator, &document);
        assert_eq!(estimator.estimate().ignored_rows, 1);
        let ignored = listed(estimator.estimate()).count() - 1;
        estimator.toggle_record(ignored, 0).expect("counted again");
        view(&mut estimator, &document);
        assert_eq!(estimator.estimate().ignored_rows, 0);
        assert_eq!(
            estimator.behind(1, MoneyForm::Full).expect("a row").columns,
            ["Date", "Payee", "Amount"]
        );
        let mut editor = open_row(&estimator, &mut document, 0).expect("opens");
        editor.set("amount", None, "$2,400").expect("a field");
        apply_row(&mut estimator, &mut document, &mut editor, 0).expect("applied");
        view(&mut estimator, &document);
        assert_eq!(estimator.estimate().proposals[0].amount, 2400, "kept");
        assert!(!document.draft().is_dirty());

        let mut editor = open_row(&estimator, &mut document, 1).expect("opens");
        editor.set("expense", None, "travel").expect("a field");
        apply_row(&mut estimator, &mut document, &mut editor, 1).expect("moved");
        assert!(document.draft().is_dirty() && document.draft().can_undo());
        let shown = view(&mut estimator, &document);
        assert_eq!(shown.rows[0].cells[0], "Travel", "{shown:?}");
    }

    #[test]
    fn the_estimate_table_is_set_as_a_step() {
        let mut document = opened();
        let mut editor = Entry::open(SETTINGS_FORM, document.draft(), Some(0));
        editor.set("lean", None, "existing").expect("a field");
        document.apply(&mut editor).expect("taken");
        let settings = document.draft().plan.estimate.clone().expect("a table");
        assert_eq!(settings.lean, retiretui_engine::plan::Lean::Existing);
        assert!(document.draft().can_undo());
    }
}
