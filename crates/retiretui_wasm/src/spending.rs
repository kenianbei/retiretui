//! The Spending Ceiling tool for a page: the most the plan's flexible
//! spending could be in its own market and at the target the draft holds,
//! in both dollar bases, and the chosen ceiling taken into the draft or
//! written as a scenario.

use retiretui_client::forms::Form;
use retiretui_client::forms::details::{self, ReadRow};
use retiretui_client::forms::edit::Entry;
use retiretui_client::present::MoneyForm;
use retiretui_client::searches::overview::spending_said;
use retiretui_client::searches::spending::{
    ABOUT, Answers, FIELDS, Found, ITEM_COLUMNS, LEADING, Listed, NOTHING_SEARCHED, PLANNED, note,
    option_columns, search, taken, target_in,
};
use retiretui_client::searches::{CURRENT_PLAN, page_refusal};
use retiretui_engine::market::{History, Progress};
use retiretui_engine::optimize::{ScaledExpense, apply_spending, spending_overlay};
use retiretui_engine::plan::Plan;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::edits::JsEditor;
use crate::searches::gated;
use crate::{Bases, JsDocument, from_js, refused, reply, tables, to_js};

static FORM: Form = Form::tool::<Answers>("Target", FIELDS);

/// One ceiling the search found.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CeilingOption {
    /// What the address holds it by: `planned` or `target`.
    pub key: &'static str,
    /// What it was held to: `In 90% of markets`.
    pub held_to: String,
    /// Its flexible spending, change and success, then the plan's figures
    /// at it.
    pub figures: Bases<Vec<String>>,
    /// Each expense it scales: its name, what the plan states, and what
    /// the ceiling makes it.
    pub items: Vec<[String; 3]>,
    /// The amounts it makes of the plan's flexible expenses, as it is
    /// taken or written.
    pub expenses: Vec<ScaledExpense>,
    /// What is asked before it is taken into the plan searched.
    pub question: String,
    /// What is said under it: of the ceiling in the plan's own market,
    /// where the plan asks to leave nothing.
    pub note: Option<&'static str>,
}

/// Both ceilings, beside the plan as it stands.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SpendingOptions {
    /// What a ceiling was held to, its flexible spending, change and
    /// success, then each figure.
    pub columns: Vec<&'static str>,
    /// What the plan's own row is called.
    pub current: &'static str,
    /// The plan's cells as it stands.
    pub baseline: Bases<Vec<String>>,
    /// The ceiling in the plan's own market, then the one at the target.
    pub options: Vec<CeilingOption>,
    /// The key of the ceiling highlighted where the address names none.
    pub leading: &'static str,
    /// What the Overview says of the ceiling at the target against what
    /// the plan spends.
    pub better: String,
}

fn option_of(listed: &Listed, plan: &Plan) -> CeilingOption {
    CeilingOption {
        key: listed.key,
        held_to: listed.held_to.clone(),
        figures: Bases::of(|nominal| listed.cells(plan, !nominal, MoneyForm::Full)),
        items: listed.items(plan, MoneyForm::Full),
        expenses: listed.ceiling.expenses.clone(),
        question: listed.take_question(plan),
        note: note(plan).filter(|_| listed.key == PLANNED),
    }
}

impl SpendingOptions {
    fn new(plan: &Plan, found: &Found) -> Self {
        let listed = found.listed();
        Self {
            columns: option_columns(),
            current: CURRENT_PLAN,
            baseline: Bases::of(|nominal| found.plan_cells(plan, !nominal, MoneyForm::Full)),
            options: listed.iter().map(|each| option_of(each, plan)).collect(),
            leading: listed[LEADING].key,
            better: spending_said(found, plan),
        }
    }
}

/// The most the flexible spending of `text`, a plan, could be: in its own
/// market, and in `success` of its random markets.
///
/// # Errors
///
/// Where the plan does not pass the gate, or the search refuses it.
pub fn spending(text: &str, success: f64) -> Result<SpendingOptions, String> {
    let plan = gated(text)?;
    let history = History::embedded();
    let found =
        search(&plan, tables(), history, success, &Progress::default()).map_err(page_refusal)?;
    Ok(SpendingOptions::new(&plan, &found))
}

/// What the Spending Ceiling tool says of itself.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SpendingWords {
    /// What the tool is for, in a line.
    pub about: &'static str,
    /// Before anything is searched.
    pub nothing_searched: &'static str,
    /// The columns a ceiling's expenses are tabled under.
    pub item_columns: [&'static str; 3],
}

impl Document {
    /// The target open in its form.
    #[must_use]
    pub fn target(&self) -> Entry {
        Entry::open(FORM, self.draft(), Some(0))
    }

    /// Holds what `editor` was given as the target, beside the plan and
    /// outside its history.
    ///
    /// # Errors
    ///
    /// Why the target was not held, in the form's words.
    pub fn apply_target(&mut self, editor: &mut Entry) -> Result<(), String> {
        self.hold(editor)
    }

    /// The target share a search is handed.
    #[must_use]
    pub fn target_share(&self) -> f64 {
        target_in(self.draft().answers::<Answers>())
    }

    /// The target read out, its label beside what it holds.
    #[must_use]
    pub fn target_read(&self) -> Vec<ReadRow> {
        let answers = self.draft().answers::<Answers>();
        details::rows(&FORM, &answers, &self.draft().plan)
    }

    /// Takes `expenses` into the draft as one step of history, answering
    /// what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take_spending(&mut self, expenses: &[ScaledExpense]) -> Result<String, String> {
        self.step(|plan| {
            apply_spending(plan, expenses);
            Ok(taken(expenses.iter().map(|expense| expense.amount).sum()))
        })
    }

    /// `expenses` as a scenario to be written at `out`, its `base` the
    /// document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// resolved from, or the scenario does not serialize.
    pub fn spending_scenario(
        &self,
        out: &str,
        expenses: &[ScaledExpense],
    ) -> Result<String, String> {
        let base = self.scenario_base(out)?;
        spending_overlay(&base, expenses).map_err(|error| error.to_string())
    }
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The Spending Ceiling target open in its form.
    #[must_use]
    pub fn target(&self) -> JsEditor {
        JsEditor(self.0.target())
    }

    /// Holds what `editor` was given as the target, outside the draft's
    /// history.
    ///
    /// # Errors
    ///
    /// Why it was not held, in the form's words.
    #[wasm_bindgen(js_name = applyTarget)]
    pub fn apply_target(&mut self, editor: &mut JsEditor) -> Result<(), JsError> {
        self.0.apply_target(&mut editor.0).map_err(refused)
    }

    /// The target share a search is handed.
    #[wasm_bindgen(getter, js_name = targetShare)]
    #[must_use]
    pub fn target_share(&self) -> f64 {
        self.0.target_share()
    }

    /// The target read out: its label beside what it holds.
    ///
    /// # Errors
    ///
    /// Where the rows do not convert.
    #[wasm_bindgen(js_name = targetRead, unchecked_return_type = "ReadRow[]")]
    pub fn target_read(&self) -> Result<JsValue, JsError> {
        to_js(&self.0.target_read())
    }

    /// Takes `expenses` into the draft as one step of history, answering
    /// what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, or `expenses` are not a ceiling's.
    #[wasm_bindgen(js_name = takeSpending)]
    pub fn take_spending(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "ScaledExpense[]")] expenses: JsValue,
    ) -> Result<String, JsError> {
        let expenses: Vec<ScaledExpense> = from_js(expenses)?;
        self.0.take_spending(&expenses).map_err(refused)
    }

    /// `expenses` as a scenario to be written at `out`, its `base` the
    /// document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, `expenses` are not a ceiling's, or the scenario does not
    /// serialize.
    #[wasm_bindgen(js_name = spendingScenario)]
    pub fn spending_scenario(
        &self,
        out: &str,
        #[wasm_bindgen(unchecked_param_type = "ScaledExpense[]")] expenses: JsValue,
    ) -> Result<String, JsError> {
        let expenses: Vec<ScaledExpense> = from_js(expenses)?;
        self.0.spending_scenario(out, &expenses).map_err(refused)
    }
}

/// What the Spending Ceiling tool says of itself.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = spendingWords, unchecked_return_type = "SpendingWords")]
pub fn spending_words() -> Result<JsValue, JsError> {
    to_js(&SpendingWords {
        about: ABOUT,
        nothing_searched: NOTHING_SEARCHED,
        item_columns: ITEM_COLUMNS,
    })
}

/// The most `plan`'s flexible spending could be: in its own market, and in
/// `success` of its random markets.
///
/// # Errors
///
/// Where the plan does not pass the gate, or the search refuses it.
#[wasm_bindgen(js_name = spending, unchecked_return_type = "SpendingOptions")]
pub fn js_spending(plan: &str, success: f64) -> Result<JsValue, JsError> {
    reply(spending(plan, success))
}

#[cfg(test)]
mod tests {
    use retiretui_client::files::OVERLAY_SAVE_FIRST;
    use retiretui_client::searches::FIGURES;
    use retiretui_client::searches::spending::{AT_TARGET, OPTION_COLUMNS, SPENDS_IT_ALL};
    use retiretui_client::setup::examples::named;

    use super::*;

    /// The retired couple run through fewer markets, which a search of
    /// them is quick over.
    fn example() -> String {
        let text = named("retired-couple.toml").expect("the example").2;
        format!("{text}\n[market.monte_carlo]\ntrials = 100\n")
    }

    fn opened() -> Document {
        let mut read = |_: &std::path::Path| Ok(example());
        Document::open("/plan.toml", &mut read).expect("opens")
    }

    #[test]
    fn each_ceiling_is_keyed_and_tabled_with_its_expenses_and_asked_about_in_words() {
        let reply = spending(&example(), 0.8).expect("searches");
        let width = OPTION_COLUMNS.len() + FIGURES.len();
        assert_eq!(reply.columns.len(), width);
        assert_eq!(reply.baseline.today.len(), width - 1);
        assert_eq!(reply.leading, AT_TARGET);
        let [planned, at_target] = &reply.options[..] else {
            panic!("two ceilings, {:?}", reply.options);
        };
        assert_eq!((planned.key, at_target.key), (PLANNED, AT_TARGET));
        assert_eq!((planned.note, at_target.note), (Some(SPENDS_IT_ALL), None));
        assert_eq!(at_target.held_to, "In 80% of markets");
        assert_eq!(at_target.figures.today.len(), width - 1);
        assert_eq!(at_target.items.len(), at_target.expenses.len());
        assert_eq!(at_target.items[1][0], "Living expenses");
        let ids: Vec<&str> = at_target.expenses.iter().map(|it| it.id.as_str()).collect();
        assert_eq!(
            ids,
            ["housing", "living", "travel", "care"],
            "the premiums are essential"
        );
        assert!(at_target.question.starts_with("Set flexible spending to $"));
        assert!(spending(&example(), 2.0).is_err());
    }

    #[test]
    fn the_target_is_held_beside_the_draft_and_read_out() {
        let mut document = opened();
        assert!((document.target_share() - 0.9).abs() < f64::EPSILON);
        let mut editor = document.target();
        editor.set("success", None, "75%").expect("a field");
        document.apply_target(&mut editor).expect("held");
        assert!((document.target_share() - 0.75).abs() < f64::EPSILON);
        assert!(!document.draft().is_dirty() && !document.draft().can_undo());
        let read = document.target_read();
        assert_eq!(
            (read[0].label.as_str(), read[0].text.as_str()),
            ("Target success", "75%")
        );
    }

    #[test]
    fn a_ceiling_is_taken_as_one_step_and_written_only_over_the_saved_file() {
        let mut document = opened();
        let reply = spending(&example(), 0.9).expect("searches");
        let chosen = &reply.options[1].expenses;
        let said = document.take_spending(chosen).expect("taken");
        assert!(said.starts_with("flexible spending is now $"), "{said}");
        let living = &document.draft().plan.expenses[1];
        assert_eq!(
            (living.id.as_str(), living.amount),
            ("living", chosen[1].amount)
        );
        assert_eq!(document.draft().plan.expenses[2].amount, 7_000);
        assert!(document.draft().can_undo());
        assert_eq!(
            document.spending_scenario("/ceiling.toml", chosen),
            Err(OVERLAY_SAVE_FIRST.to_owned())
        );
        document.save(&mut |_| Ok(())).expect("saved");
        assert!(document.spending_scenario("/plan.toml", chosen).is_err());
        let scenario = document
            .spending_scenario("/ceiling.toml", chosen)
            .expect("written");
        assert!(scenario.contains("base = \"plan.toml\""), "{scenario}");
        assert_eq!(scenario.matches("[[expenses]]").count(), 4, "{scenario}");
        assert!(document.undo());
    }
}
