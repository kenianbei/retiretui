//! A first plan from the new-plan questions, asked a step at a time, and a
//! statement's earnings recorded on a person of the open document.

use retiretui_client::draft::Draft;
use retiretui_client::forms::{DomainId, Form};
use retiretui_client::setup::{self, FIELDS, STEPS, SetupAnswers, blank_plan};
use retiretui_engine::plan::{Item, Plan};
use serde::Serialize;
use toml::Table;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::editor::Editor;
use crate::vocabulary::slug_of;
use crate::{JsDocument, refused, tables, to_js};

static FORM: Form = Form::tool::<SetupAnswers>("New plan", FIELDS);

/// One step of the new-plan questions.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetupStep {
    /// How the step is addressed.
    pub slug: &'static str,
    /// What the step is headed.
    pub title: &'static str,
    /// The keys of the fields it asks, in the form's order.
    pub keys: Vec<&'static str>,
}

/// The plan the answers made, and the name it is offered under.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NewPlanMade {
    /// The plan as canonical TOML.
    pub text: String,
    /// A file name for it, without its extension: its first person's name.
    pub name: String,
}

/// The new-plan answers, held over a plan no one sees until they make one.
pub struct NewPlan {
    draft: Draft,
    editor: Editor,
    today: i16,
}

impl NewPlan {
    /// The questions answered as `answers` holds, as of `today`.
    ///
    /// # Errors
    ///
    /// Where the answers do not read.
    pub fn new(answers: Option<&str>, today: i16) -> Result<Self, String> {
        let host = Plan::from_toml_str(&blank_plan(today)).map_err(|error| error.to_string())?;
        let mut draft = Draft::new(host, false);
        if let Some(text) = answers {
            let table: Table = toml::from_str(text).map_err(|error| error.to_string())?;
            draft.set_answers::<SetupAnswers>(table);
        }
        let editor = Editor::open(&FORM, &draft, Some(0));
        Ok(Self {
            draft,
            editor,
            today,
        })
    }

    /// What has been answered, as TOML text to keep.
    ///
    /// # Errors
    ///
    /// Where the answers do not serialize.
    pub fn answers(&self) -> Result<String, String> {
        toml::to_string(self.editor.edit.snapshot()).map_err(|error| error.to_string())
    }

    /// The plan the answers make.
    ///
    /// # Errors
    ///
    /// Where a field does not yet hold a value, or the answers make no plan.
    pub fn create(&mut self) -> Result<NewPlanMade, String> {
        self.editor.edit.apply(&mut self.draft, None)?;
        let answers = self.draft.answers::<SetupAnswers>();
        let (plan, _) = setup::compose(answers, self.today, tables())?;
        let name = plan.household.people[0].display_name().to_owned();
        let text = plan.to_toml_string().map_err(|error| error.to_string())?;
        Ok(NewPlanMade { text, name })
    }
}

/// The new-plan questions, answered a field at a time.
#[wasm_bindgen(js_name = NewPlan)]
pub struct JsNewPlan(NewPlan);

#[wasm_bindgen(js_class = NewPlan)]
impl JsNewPlan {
    /// The questions answered as `answers` - text from `answers` - holds,
    /// as of the calendar year `today`.
    ///
    /// # Errors
    ///
    /// Where the answers do not read.
    #[wasm_bindgen(constructor)]
    pub fn new(answers: Option<String>, today: i16) -> Result<JsNewPlan, JsError> {
        NewPlan::new(answers.as_deref(), today)
            .map(Self)
            .map_err(refused)
    }

    /// Every field on show, `focused` the key of the one being typed into.
    ///
    /// # Errors
    ///
    /// Where the fields do not convert.
    #[wasm_bindgen(unchecked_return_type = "FieldView[]")]
    pub fn view(&mut self, focused: Option<String>) -> Result<JsValue, JsError> {
        let NewPlan { draft, editor, .. } = &mut self.0;
        to_js(&editor.view(draft, focused.as_deref()))
    }

    /// Writes `text` into the field `key`, the row at `place` of a list.
    ///
    /// # Errors
    ///
    /// Where there is no such field, or it is not typed or picked.
    pub fn set(&mut self, key: &str, place: Option<usize>, text: &str) -> Result<(), JsError> {
        self.0.editor.set(key, place, text).map_err(refused)
    }

    /// Ticks or clears the flag or tick `key`.
    ///
    /// # Errors
    ///
    /// Where there is no such field, or it is not ticked.
    pub fn tick(
        &mut self,
        key: &str,
        #[wasm_bindgen(js_name = isTicked)] is_ticked: bool,
    ) -> Result<(), JsError> {
        self.0.editor.tick(key, is_ticked).map_err(refused)
    }

    /// Writes `text` into `part` of the trigger `key`.
    ///
    /// # Errors
    ///
    /// Where there is no such trigger, or a trigger no such part.
    #[wasm_bindgen(js_name = setTrigger)]
    pub fn set_trigger(&mut self, key: &str, part: &str, text: &str) -> Result<(), JsError> {
        self.0.editor.set_trigger(key, part, text).map_err(refused)
    }

    /// What has been answered, as text a new `NewPlan` reads back.
    ///
    /// # Errors
    ///
    /// Where the answers do not serialize.
    #[wasm_bindgen(getter)]
    pub fn answers(&self) -> Result<String, JsError> {
        self.0.answers().map_err(refused)
    }

    /// The plan the answers make, and a name to offer it under.
    ///
    /// # Errors
    ///
    /// Where a field does not yet hold a value, or the answers make no plan.
    #[wasm_bindgen(unchecked_return_type = "NewPlanMade")]
    pub fn create(&mut self) -> Result<JsValue, JsError> {
        to_js(&self.0.create().map_err(refused)?)
    }
}

/// The steps the new-plan questions are asked in.
///
/// # Errors
///
/// Where they do not convert.
#[wasm_bindgen(js_name = setupSteps, unchecked_return_type = "SetupStep[]")]
pub fn setup_steps() -> Result<JsValue, JsError> {
    let steps = STEPS.iter().map(|step| SetupStep {
        slug: step.slug,
        title: step.title,
        keys: step.keys.to_vec(),
    });
    to_js(&steps.collect::<Vec<_>>())
}

/// The address of the page whose people a statement is recorded on.
#[wasm_bindgen(js_name = statementPage)]
#[must_use]
pub fn statement_page() -> String {
    slug_of(DomainId::People)
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// Records the statement `xml` on the person at `index` of the People
    /// page as one step of history, answering what it recorded.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, no person is at `index`, the statement
    /// does not parse, or its birth date is not theirs.
    #[wasm_bindgen(js_name = importEarnings)]
    pub fn import_earnings(&mut self, index: usize, xml: &str) -> Result<String, JsError> {
        self.0.import_earnings(index, xml).map_err(refused)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use retiretui_engine::project::validate_plan;

    use super::*;
    use crate::document::Document;

    const STATEMENT: &str = include_str!("../../retiretui_engine/tests/fixtures/statement.xml");

    const TODAY: i16 = 2026;

    #[test]
    fn answers_kept_as_text_read_back_as_they_were() {
        let mut first = NewPlan::new(None, TODAY).expect("blank");
        first.editor.set("name", None, "Jordan").expect("a field");
        first
            .editor
            .set("birth_year", None, "1975")
            .expect("a field");
        let kept = first.answers().expect("serializes");
        let second = NewPlan::new(Some(&kept), TODAY).expect("reads");
        assert_eq!(second.answers().expect("serializes"), kept);
    }

    #[test]
    fn the_answers_make_a_plan_that_passes_the_gate() {
        let mut new_plan = NewPlan::new(None, TODAY).expect("blank");
        new_plan
            .editor
            .set("name", None, "Jordan")
            .expect("a field");
        let made = new_plan.create().expect("a plan");
        assert_eq!(made.name, "Jordan");
        let plan = Plan::from_toml_str(&made.text).expect("parses");
        assert!(validate_plan(&plan, tables()).is_empty());
    }

    fn born_as_the_statement_says(text: &str) -> Document {
        let text = text.replace("1970-01-01", "1975-06-14");
        Document::open("/plan.toml", &mut |_: &Path| Ok(text.clone())).expect("opens")
    }

    #[test]
    fn a_statement_is_recorded_as_one_step_of_history() {
        let mut document = born_as_the_statement_says(&blank_plan(TODAY));
        let said = document.import_earnings(0, STATEMENT).expect("recorded");
        assert!(said.starts_with("recorded 3 year(s) of earnings for"));
        assert_eq!(document.draft().plan.household.people[0].earnings.len(), 3);
        assert!(document.undo());
        assert!(
            document.draft().plan.household.people[0]
                .earnings
                .is_empty()
        );
        assert!(document.import_earnings(1, STATEMENT).is_err());
    }

    #[test]
    fn a_scenario_records_no_statement() {
        let scenario = "schema = 1\nbase = \"/base.toml\"\n";
        let base = blank_plan(TODAY).replace("1970-01-01", "1975-06-14");
        let mut read = |path: &Path| {
            Ok(if path == Path::new("/base.toml") {
                base.clone()
            } else {
                scenario.to_owned()
            })
        };
        let mut document = Document::open("/early.toml", &mut read).expect("opens");
        assert!(document.import_earnings(0, STATEMENT).is_err());
    }
}
