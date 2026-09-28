//! The Roth Conversions tool for a page: every bracket's ladder searched
//! under the constraints the draft holds, in both dollar bases, and the
//! highlighted one taken into the draft or written as a scenario.

use std::path::Path;

use retiretui_client::files::{OVERLAY_SAVE_FIRST, relative_path};
use retiretui_client::forms::{Form, details};
use retiretui_client::searches::ladders::{
    Constraints, DESTINATION, FIELDS, aim_at, constraints_in, only_roth, rate_label, search,
    take_question, taken,
};
use retiretui_client::searches::run_refusal;
use retiretui_client::store::normal;
use retiretui_client::table::account_name;
use retiretui_engine::market::Progress;
use retiretui_engine::optimize::{
    BracketSweep, LadderStep, OptimizeOptions, SweptBracket, apply_ladder, ladder_overlay,
};
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{Projection, Summary};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::editor::Editor;
use crate::edits::JsEditor;
use crate::searches::gated;
use crate::{JsDocument, refused, reply, tables, to_js};

/// A scenario written over a file it resolves through would name itself.
const OVER_ITS_BASE: &str = "a scenario cannot be written over a file it is made from";

static FORM: Form = Form::tool::<Constraints>("Constraints", FIELDS);

/// Headline figures in either dollar basis.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Figures {
    /// In today's dollars.
    pub today: Summary,
    /// In the dollars of each year.
    pub nominal: Summary,
}

impl Figures {
    fn of(projection: &Projection) -> Self {
        Self {
            today: projection.summary(true),
            nominal: projection.summary(false),
        }
    }
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
    /// What turns the year's dollars into today's.
    pub deflator: f64,
}

/// One bracket's ladder.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LadderOption {
    /// The bracket's rate, such as 0.22.
    pub rate: f64,
    /// The rate as a percent: `22%`.
    pub label: String,
    /// What it converts over the plan, in today's dollars.
    pub converted_today: Dollars,
    /// What it converts over the plan, nominal.
    pub converted_nominal: Dollars,
    /// The plan's figures with the ladder.
    pub figures: Figures,
    /// Its conversions, year by year.
    pub steps: Vec<LadderYear>,
    /// What is asked before it is taken into the plan searched.
    pub question: String,
}

/// Every bracket's ladder, best first, beside the plan as it stands.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaddersReply {
    /// The Roth account the ladders fill.
    pub destination: String,
    /// The plan's figures as it stands.
    pub baseline: Figures,
    /// One option per bracket searched, best first.
    pub brackets: Vec<LadderOption>,
}

impl LaddersReply {
    fn new(plan: &Plan, sweep: &BracketSweep, options: &OptimizeOptions) -> Self {
        Self {
            destination: options.destination.clone(),
            baseline: Figures::of(&sweep.baseline),
            brackets: sweep
                .brackets
                .iter()
                .map(|bracket| option_of(plan, bracket))
                .collect(),
        }
    }
}

fn option_of(plan: &Plan, bracket: &SweptBracket) -> LadderOption {
    let steps = bracket.steps.iter().map(|step| {
        let row = bracket.optimized.row(step.year);
        LadderYear {
            year: step.year,
            source: step.source.clone(),
            from: account_name(plan, &step.source).to_owned(),
            amount: step.amount,
            taxable: row.map_or(0, |row| row.taxes.ordinary_taxable),
            deflator: row.map_or(1.0, |row| row.deflator),
        }
    });
    LadderOption {
        rate: bracket.rate,
        label: rate_label(bracket.rate),
        converted_today: bracket.converted(true),
        converted_nominal: bracket.converted(false),
        figures: Figures::of(&bracket.optimized),
        steps: steps.collect(),
        question: take_question(bracket, plan),
    }
}

/// Every bracket's ladder in `text`, a plan, under `answers`, the
/// constraints as TOML.
///
/// # Errors
///
/// Where the plan does not pass the gate, the answers do not read or name
/// no destination, or the search refuses them.
pub fn ladders(text: &str, answers: &str) -> Result<LaddersReply, String> {
    let plan = gated(text)?;
    let answers = toml::from_str(answers).map_err(|error| error.to_string())?;
    let (options, rate) = constraints_in(answers)?;
    let sweep =
        search(&plan, tables(), &options, rate, &Progress::default()).map_err(run_refusal)?;
    Ok(LaddersReply::new(&plan, &sweep, &options))
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
        if let Some(only) = only_roth(self.draft()) {
            aim_at(self.draft_mut(), &only);
        }
        Editor::open(&FORM, self.draft(), Some(0))
    }

    /// Holds what `editor` was given as the constraints, beside the plan
    /// and outside its history.
    ///
    /// # Errors
    ///
    /// Why the constraints were not held, in the form's words.
    pub fn apply_constraints(&mut self, editor: &mut Editor) -> Result<(), String> {
        editor.edit.apply(self.draft_mut(), None).map(drop)
    }

    /// The constraints as TOML, what a search is handed.
    ///
    /// # Errors
    ///
    /// Where they do not serialize.
    pub fn constraints_text(&self) -> Result<String, String> {
        toml::to_string(&self.aimed()).map_err(|error| error.to_string())
    }

    /// Whether the constraints name the Roth account to convert to, as a
    /// search needs them to.
    #[must_use]
    pub fn is_aimed(&self) -> bool {
        self.aimed().contains_key(DESTINATION)
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
        self.step(|plan| apply_ladder(plan, &into(destination), &steps))?;
        Ok(taken(&steps))
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
        if self.draft().is_dirty() {
            return Err(OVERLAY_SAVE_FIRST.to_owned());
        }
        let out = normal(Path::new(out));
        if self.files().contains(&out) {
            return Err(OVER_ITS_BASE.to_owned());
        }
        let within = out.parent().unwrap_or(Path::new("/"));
        let base = relative_path(within, &self.files()[0]);
        let steps = steps_of(years);
        ladder_overlay(&base, &self.draft().plan, &into(destination), &steps)
            .map_err(|error| error.to_string())
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

    /// The constraints as TOML, what a search is handed.
    ///
    /// # Errors
    ///
    /// Where they do not serialize.
    #[wasm_bindgen(getter, js_name = constraintsText)]
    pub fn constraints_text(&self) -> Result<String, JsError> {
        self.0.constraints_text().map_err(refused)
    }

    /// Whether the constraints name the Roth account to convert to, as a
    /// search needs them to.
    #[wasm_bindgen(getter, js_name = isAimed)]
    #[must_use]
    pub fn is_aimed(&self) -> bool {
        self.0.is_aimed()
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
            .take_ladder(destination, years_of(years)?)
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
            .ladder_scenario(out, destination, years_of(years)?)
            .map_err(refused)
    }
}

fn years_of(years: JsValue) -> Result<Vec<LadderYear>, JsError> {
    from_value(years).map_err(|error| JsError::new(&error.to_string()))
}

/// Every bracket's ladder in `plan` under `constraints`, the constraints as
/// TOML, best first.
///
/// # Errors
///
/// Where the plan does not pass the gate, the constraints name no
/// destination, or the search refuses them.
#[wasm_bindgen(js_name = ladders, unchecked_return_type = "LaddersReply")]
pub fn js_ladders(plan: &str, constraints: &str) -> Result<JsValue, JsError> {
    reply(ladders(plan, constraints))
}

#[cfg(test)]
mod tests {
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
        assert!(document.is_aimed());
        let answers = document.constraints_text().expect("serializes");
        assert!(answers.contains(ROTH), "{answers}");
        let reply = ladders(example(), &answers).expect("searches");
        assert_eq!(reply.destination, ROTH);
        let best = reply.brackets.first().expect("a bracket");
        assert!(!best.steps.is_empty());
        assert!(best.converted_today <= best.converted_nominal);
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
        let reply = ladders(example(), &answers).expect("searches");
        let rates: Vec<f64> = reply.brackets.iter().map(|b| b.rate).collect();
        assert_eq!(rates, [0.22]);
        assert!(
            ladders(example(), "bracket = 22").is_err(),
            "no destination"
        );
    }

    #[test]
    fn a_ladder_is_taken_as_one_step_and_written_over_the_draft() {
        let mut document = opened();
        let answers = document.constraints_text().expect("serializes");
        let reply = ladders(example(), &answers).expect("searches");
        let years = || reply.brackets[0].steps.clone();
        let count = reply.brackets[0].steps.len();
        let said = document.take_ladder(ROTH, years()).expect("taken");
        assert_eq!(said, format!("took {count} conversion(s) into the plan"));
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
