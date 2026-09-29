//! The Roth Conversions tool for a page: every bracket's ladder searched
//! under the constraints the draft holds, in both dollar bases, and the
//! highlighted one taken into the draft or written as a scenario.

use std::path::Path;

use retiretui_client::files::{OVERLAY_SAVE_FIRST, relative_path};
use retiretui_client::forms::{Form, details};
use retiretui_client::searches::ladders::{
    CONVERSION_COLUMNS, CONVERTS_NOTHING, Constraints, DESTINATION, FIELDS, NO_BRACKET,
    OPTION_COLUMNS, PICK_DESTINATION, Swept, constraints_in, held_answers, only_roth, option_cells,
    rate_label, search, take_question, taken, taxed_in,
};
use retiretui_client::searches::overview::ladder_said;
use retiretui_client::searches::{AGAINST_PLAN, CURRENT_PLAN, FIGURES, run_refusal};
use retiretui_client::store::normal;
use retiretui_client::table::{account_name, basis_amount};
use retiretui_engine::market::Progress;
use retiretui_engine::optimize::{
    LadderStep, OptimizeOptions, SweptBracket, apply_ladder, ladder_overlay,
};
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::Summary;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::editor::Editor;
use crate::edits::JsEditor;
use crate::searches::gated;
use crate::{Bases, JsDocument, from_js, option_figures, refused, reply, tables, to_js};

/// A scenario written over a file it resolves through would name itself.
const OVER_ITS_BASE: &str = "a scenario cannot be written over a file it is made from";

static FORM: Form = Form::tool::<Constraints>("Constraints", FIELDS);

/// What the tool says of itself, beside what a search replies.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LadderWords {
    /// While no Roth account is named to convert to.
    pub pick_destination: &'static str,
    /// Where the constraints leave no bracket to fill.
    pub no_bracket: &'static str,
    /// In place of a ladder that converts nothing.
    pub converts_nothing: &'static str,
    /// The column a phone's row shows beside the bracket.
    pub against_plan: &'static str,
}

/// One year of a ladder: what it converts, and from where.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LadderYear {
    /// The year converted in.
    pub year: i16,
    /// The account converted from, by id.
    pub source: String,
    /// The account converted from, by name.
    pub from: String,
    /// Nominal dollars converted.
    pub amount: Dollars,
    /// The ordinary income taxed that year with the ladder, nominal.
    pub taxable: Dollars,
    /// What it converts, in today's dollars.
    pub amount_today: Dollars,
    /// The ordinary income taxed that year, in today's dollars.
    pub taxable_today: Dollars,
}

/// One bracket's ladder.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LadderOption {
    /// The bracket's rate, such as 0.22.
    pub rate: f64,
    /// The rate as a percent: `22%`.
    pub label: String,
    /// The plan's figures with the ladder.
    pub figures: Bases<Vec<String>>,
    /// Its conversions, year by year.
    pub steps: Vec<LadderYear>,
    /// What is asked before it is taken into the plan searched.
    pub question: String,
}

/// Every bracket's ladder, best first, beside the plan as it stands.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaddersReply {
    /// What an option's columns are called: the bracket, what it ends with
    /// against the plan, what the plan converts over its life, then each of
    /// its figures.
    pub columns: Vec<&'static str>,
    /// What a ladder's conversions are tabled under.
    pub conversion_columns: Vec<&'static str>,
    /// What the plan's own row is called.
    pub current: &'static str,
    /// The Roth account the ladders fill.
    pub destination: String,
    /// The plan's figures as it stands.
    pub baseline: Bases<Vec<String>>,
    /// One option per bracket searched, best first.
    pub brackets: Vec<LadderOption>,
    /// What the best ladder does better than the plan, in either basis.
    pub better: Bases<String>,
}

impl LaddersReply {
    fn new(plan: &Plan, swept: &Swept) -> Self {
        let Swept { sweep, options } = swept;
        let baseline = &sweep.baseline;
        let summaries = Bases::of(|nominal| baseline.summary(!nominal));
        Self {
            columns: OPTION_COLUMNS.into_iter().chain(FIGURES).collect(),
            conversion_columns: CONVERSION_COLUMNS.to_vec(),
            current: CURRENT_PLAN,
            destination: options.destination.clone(),
            baseline: option_figures(option_cells, &sweep.baseline, None),
            brackets: sweep
                .brackets
                .iter()
                .map(|bracket| option_of(plan, bracket, &summaries))
                .collect(),
            better: Bases::of(|nominal| ladder_said(Some(swept), baseline, nominal)),
        }
    }
}

fn option_of(plan: &Plan, bracket: &SweptBracket, summaries: &Bases<Summary>) -> LadderOption {
    let steps = bracket.steps.iter().map(|step| {
        let (taxable, deflator) = taxed_in(bracket, step.year);
        LadderYear {
            year: step.year,
            source: step.source.clone(),
            from: account_name(plan, &step.source).to_owned(),
            amount: step.amount,
            taxable,
            amount_today: basis_amount(step.amount, deflator, false),
            taxable_today: basis_amount(taxable, deflator, false),
        }
    });
    LadderOption {
        rate: bracket.rate,
        label: rate_label(bracket.rate),
        figures: option_figures(option_cells, &bracket.optimized, Some(summaries)),
        steps: steps.collect(),
        question: take_question(bracket, plan),
    }
}

/// Every bracket's ladder into `destination` in `text`, a plan, under
/// `answers`, the rest of the constraints as TOML.
///
/// # Errors
///
/// Where the plan does not pass the gate, the answers do not read, or the
/// search refuses them.
pub fn ladders(text: &str, answers: &str, destination: &str) -> Result<LaddersReply, String> {
    let plan = gated(text)?;
    let mut answers: toml::Table = toml::from_str(answers).map_err(|error| error.to_string())?;
    answers.insert(DESTINATION.to_owned(), destination.into());
    let (options, rate) = constraints_in(answers)?;
    let sweep =
        search(&plan, tables(), &options, rate, &Progress::default()).map_err(run_refusal)?;
    Ok(LaddersReply::new(&plan, &Swept { sweep, options }))
}

/// The options a ladder into `destination` is taken or written under: the
/// destination is all either reads of them.
fn into(destination: &str) -> OptimizeOptions {
    retiretui_client::ladder::LadderConstraints::default().options(&[], destination)
}

fn steps_of(years: Vec<LadderYear>) -> Vec<LadderStep> {
    let steps = years.into_iter().map(|year| LadderStep {
        year: year.year,
        source: year.source,
        amount: year.amount,
    });
    steps.collect()
}

impl Document {
    /// The constraints the draft holds, the plan's one Roth account named
    /// where they name none.
    fn aimed(&self) -> toml::Table {
        let mut answers = self.draft().answers::<Constraints>();
        if let Some(only) = only_roth(self.draft()) {
            answers.insert(DESTINATION.to_owned(), only.into());
        }
        answers
    }

    /// The constraints open in their form.
    pub fn constraints(&mut self) -> Editor {
        self.hold_answers::<Constraints>(self.aimed());
        Editor::open(&FORM, self.draft(), Some(0))
    }

    /// Holds what `editor` was given as the constraints, beside the plan
    /// and outside its history.
    ///
    /// # Errors
    ///
    /// Why the constraints were not held, in the form's words.
    pub fn apply_constraints(&mut self, editor: &mut Editor) -> Result<(), String> {
        self.hold(editor)
    }

    /// The constraints but the destination as TOML, what a search into
    /// any account is handed.
    ///
    /// # Errors
    ///
    /// Where they do not serialize.
    pub fn constraints_text(&self) -> Result<String, String> {
        toml::to_string(&held_answers(self.draft())).map_err(|error| error.to_string())
    }

    /// The account the constraints aim at, where they name one.
    #[must_use]
    pub fn destination(&self) -> Option<String> {
        let answers = self.aimed();
        Some(answers.get(DESTINATION)?.as_str()?.to_owned())
    }

    /// The constraints read out, each field's label beside what it holds.
    #[must_use]
    pub fn constraints_read(&self) -> Vec<[String; 2]> {
        details::rows(&FORM, &self.aimed(), &self.draft().plan)
    }

    /// Takes the ladder `years` into `destination` in place of any ladder
    /// the plan holds, as one step of history, answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take_ladder(
        &mut self,
        destination: &str,
        years: Vec<LadderYear>,
    ) -> Result<String, String> {
        let steps = steps_of(years);
        self.step(|plan| {
            apply_ladder(plan, &into(destination), &steps);
            Ok(taken(&steps))
        })
    }

    /// The ladder `years` into `destination` as a scenario to be written
    /// at `out`, its `base` the document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, which the file on disk does not
    /// hold, `out` is a file the document was resolved from, or the
    /// scenario does not serialize.
    pub fn ladder_scenario(
        &self,
        out: &str,
        destination: &str,
        years: Vec<LadderYear>,
    ) -> Result<String, String> {
        let base = self.scenario_base(out)?;
        let steps = steps_of(years);
        ladder_overlay(&base, &self.draft().plan, &into(destination), &steps)
            .map_err(|error| error.to_string())
    }

    /// The `base` a scenario written at `out` names: the document's file,
    /// relative to `out`.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, which the file on disk does not
    /// hold, or `out` is a file the document was resolved from.
    pub(crate) fn scenario_base(&self, out: &str) -> Result<String, String> {
        if self.draft().is_dirty() {
            return Err(OVERLAY_SAVE_FIRST.to_owned());
        }
        let out = normal(Path::new(out));
        if self.files().contains(&out) {
            return Err(OVER_ITS_BASE.to_owned());
        }
        let within = out.parent().unwrap_or(Path::new("/"));
        Ok(relative_path(within, &self.files()[0]))
    }
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The Roth Conversions constraints open in their form.
    #[must_use]
    pub fn constraints(&mut self) -> JsEditor {
        JsEditor(self.0.constraints())
    }

    /// Holds what `editor` was given as the constraints, outside the
    /// draft's history.
    ///
    /// # Errors
    ///
    /// Why they were not held, in the form's words.
    #[wasm_bindgen(js_name = applyConstraints)]
    pub fn apply_constraints(&mut self, editor: &mut JsEditor) -> Result<(), JsError> {
        self.0.apply_constraints(&mut editor.0).map_err(refused)
    }

    /// The constraints but the destination as TOML, what a search into
    /// any account is handed.
    ///
    /// # Errors
    ///
    /// Where they do not serialize.
    #[wasm_bindgen(getter, js_name = constraintsText)]
    pub fn constraints_text(&self) -> Result<String, JsError> {
        self.0.constraints_text().map_err(refused)
    }

    /// The constraints read out: each field's label beside what it holds.
    ///
    /// # Errors
    ///
    /// Where the rows do not convert.
    #[wasm_bindgen(js_name = constraintsRead, unchecked_return_type = "[string, string][]")]
    pub fn constraints_read(&self) -> Result<JsValue, JsError> {
        to_js(&self.0.constraints_read())
    }

    /// Takes the ladder `years` into `destination` as one step of history,
    /// answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, or `years` are not a ladder's.
    #[wasm_bindgen(js_name = takeLadder)]
    pub fn take_ladder(
        &mut self,
        destination: &str,
        #[wasm_bindgen(unchecked_param_type = "LadderYear[]")] years: JsValue,
    ) -> Result<String, JsError> {
        self.0
            .take_ladder(destination, from_js(years)?)
            .map_err(refused)
    }

    /// The ladder `years` into `destination` as a scenario to be written at
    /// `out`, its `base` the document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, `years` are not a ladder's, or the scenario does not
    /// serialize.
    #[wasm_bindgen(js_name = ladderScenario)]
    pub fn ladder_scenario(
        &self,
        out: &str,
        destination: &str,
        #[wasm_bindgen(unchecked_param_type = "LadderYear[]")] years: JsValue,
    ) -> Result<String, JsError> {
        self.0
            .ladder_scenario(out, destination, from_js(years)?)
            .map_err(refused)
    }
}

/// What the Roth Conversions tool says of itself.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = ladderWords, unchecked_return_type = "LadderWords")]
pub fn ladder_words() -> Result<JsValue, JsError> {
    to_js(&LadderWords {
        pick_destination: PICK_DESTINATION,
        no_bracket: NO_BRACKET,
        converts_nothing: CONVERTS_NOTHING,
        against_plan: AGAINST_PLAN,
    })
}

/// Every bracket's ladder into `destination` in `plan` under
/// `constraints`, the rest of the constraints as TOML, best first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or the search refuses the
/// constraints.
#[wasm_bindgen(js_name = ladders, unchecked_return_type = "LaddersReply")]
pub fn js_ladders(plan: &str, constraints: &str, destination: &str) -> Result<JsValue, JsError> {
    reply(ladders(plan, constraints, destination))
}

#[cfg(test)]
mod tests {
    use retiretui_client::present::parse_money;
    use retiretui_client::setup::EXAMPLES;
    use retiretui_engine::optimize::is_ladder;

    use super::*;

    const EARLY_RETIREE: &str = "early-retiree.toml";
    const ROTH: &str = "roth-ira-morgan";

    fn example() -> &'static str {
        let found = EXAMPLES.iter().find(|(file, ..)| *file == EARLY_RETIREE);
        found.expect("the example").2
    }

    fn opened() -> Document {
        let mut read = |_: &std::path::Path| Ok(example().to_owned());
        Document::open("/plan.toml", &mut read).expect("opens")
    }

    #[test]
    fn the_lone_roth_account_is_searched_into_without_being_named() {
        let document = opened();
        assert_eq!(document.destination().as_deref(), Some(ROTH));
        let answers = document.constraints_text().expect("serializes");
        assert!(
            !answers.contains(ROTH),
            "the destination is its own: {answers}"
        );
        let reply = ladders(example(), &answers, ROTH).expect("searches");
        assert_eq!(reply.destination, ROTH);
        let best = reply.brackets.first().expect("a bracket");
        assert!(!best.steps.is_empty());
        let converted = |cells: &[String]| parse_money(&cells[1]).expect("money");
        assert!(
            converted(&best.figures.today) <= converted(&best.figures.nominal),
            "converted, deflated"
        );
        assert!(best.figures.today[0].starts_with(['+', '-', '$']));
        assert_eq!(reply.baseline.today[0], "", "the plan's own row is blank");
        assert_eq!(best.label, rate_label(best.rate));
        assert!(best.question.starts_with("Take the "), "{}", best.question);
        let read = document.constraints_read();
        assert!(
            read.iter().any(|[_, value]| value.contains("Roth")),
            "{read:?}"
        );
    }

    #[test]
    fn held_constraints_search_one_bracket_and_stay_out_of_history() {
        let mut document = opened();
        let mut editor = document.constraints();
        editor.set("bracket", None, "22").expect("typed");
        document.apply_constraints(&mut editor).expect("held");
        assert!(!document.draft().can_undo(), "no step of history");
        assert!(!document.draft().is_dirty());
        let answers = document.constraints_text().expect("serializes");
        let reply = ladders(example(), &answers, ROTH).expect("searches");
        let rates: Vec<f64> = reply.brackets.iter().map(|b| b.rate).collect();
        assert_eq!(rates, [0.22]);
        assert!(
            ladders(example(), "bracket = 22", "nowhere").is_err(),
            "no such account"
        );
    }

    #[test]
    fn a_ladder_is_taken_as_one_step_and_written_over_the_draft() {
        let mut document = opened();
        let answers = document.constraints_text().expect("serializes");
        let reply = ladders(example(), &answers, ROTH).expect("searches");
        let years = || reply.brackets[0].steps.clone();
        let count = reply.brackets[0].steps.len();
        let said = document.take_ladder(ROTH, years()).expect("taken");
        assert_eq!(said, format!("took {count} conversions into the plan"));
        let held = document
            .draft()
            .plan
            .conversions
            .iter()
            .filter(|c| is_ladder(c));
        assert_eq!(held.count(), count);
        assert_eq!(
            document.ladder_scenario("/ladder.toml", ROTH, Vec::new()),
            Err(OVERLAY_SAVE_FIRST.to_owned()),
            "the file on disk holds no ladder yet"
        );
        document.save(&mut |_| Ok(())).expect("saved");
        assert_eq!(
            document.ladder_scenario("/plan.toml", ROTH, Vec::new()),
            Err(OVER_ITS_BASE.to_owned())
        );
        let scenario = document
            .ladder_scenario("/ladder.toml", ROTH, Vec::new())
            .expect("written");
        assert!(scenario.contains("base = \"plan.toml\""), "{scenario}");
        assert_eq!(
            scenario.matches("remove = true").count(),
            count,
            "{scenario}"
        );
        assert!(document.undo());
        assert!(!document.draft().plan.conversions.iter().any(is_ladder));
    }

    #[test]
    fn a_scenario_takes_no_ladder() {
        let mut read = |path: &std::path::Path| {
            Ok(if path.ends_with("over.toml") {
                "schema = 1\nbase = \"plan.toml\"\n".to_owned()
            } else {
                example().to_owned()
            })
        };
        let mut document = Document::open("/over.toml", &mut read).expect("opens");
        assert!(document.take_ladder(ROTH, Vec::new()).is_err());
        let mut editor = document.constraints();
        editor.set("bracket", None, "22").expect("typed");
        assert!(
            document.apply_constraints(&mut editor).is_ok(),
            "a scenario is searched too"
        );
    }
}
