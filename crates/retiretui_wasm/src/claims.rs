//! The SSA Benefits tool for a page: every claim age ranked for the
//! household, each person's record and estimated benefit with what can be
//! done for them, the chosen claims taken into the draft or written as a
//! scenario; and what the Overview's Could do better card asks of the
//! document.

use retiretui_client::searches::claims::{
    self, NO_CLAIM, NOBODY, NOTHING_SEARCHED, PEOPLE_COLUMNS, PersonAction, age_cell, claimants,
    person_row, take_question, taken,
};
use retiretui_client::searches::ladders::{Constraints, DESTINATION};
use retiretui_client::searches::overview::{
    COULD_DO_BETTER, NOTHING_TO_SEARCH, REFUSED, claims_said, roth_owners,
};
use retiretui_client::searches::{CURRENT_PLAN, FIGURES, figure_amounts, run_refusal};
use retiretui_engine::market::Progress;
use retiretui_engine::optimize::{
    Claim, ClaimCandidate, ClaimSearch, apply_claims, benefit_estimates, claims_overlay,
    computed_income, optimize_claims,
};
use retiretui_engine::plan::{Dollars, Income, Item, Plan};
use retiretui_engine::project::Projection;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::ladders::Better;
use crate::searches::gated;
use crate::{JsDocument, from_js, refused, reply, tables, to_js};

/// An option's figures in either dollar basis.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimFigures {
    /// In today's dollars.
    pub today: [Dollars; 4],
    /// In the dollars of each year.
    pub nominal: [Dollars; 4],
}

impl ClaimFigures {
    fn of(projection: &Projection) -> Self {
        Self {
            today: figure_amounts(&projection.summary(true)),
            nominal: figure_amounts(&projection.summary(false)),
        }
    }
}

/// One set of claims the search tried.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimOption {
    /// Its ages joined, as the address holds it: `70-67`.
    pub key: String,
    /// The age each person claims at, in the reply's column order.
    pub ages: Vec<u8>,
    /// What is taken or written.
    pub claims: Vec<Claim>,
    /// The plan's figures under them.
    pub figures: ClaimFigures,
    /// What is asked before they are taken into the plan searched.
    pub question: String,
}

/// Every claim age for the household, best first, beside the plan as it
/// stands.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimsOptions {
    /// Each person claimed for, by name, then each figure.
    pub columns: Vec<String>,
    /// What the plan's own row is called.
    pub current: &'static str,
    /// The age each claim is at in the plan as it stands, or what an
    /// unpaid one says.
    pub current_ages: Vec<String>,
    /// The plan's figures as it stands.
    pub baseline: ClaimFigures,
    /// Every set of claims, best first.
    pub options: Vec<ClaimOption>,
    /// What the best claims do better than the plan, in either basis.
    pub better: Better,
}

fn option_of(plan: &Plan, candidate: &ClaimCandidate) -> ClaimOption {
    let ages: Vec<u8> = candidate.claims.iter().map(|claim| claim.age).collect();
    let key: Vec<String> = ages.iter().map(u8::to_string).collect();
    ClaimOption {
        key: key.join("-"),
        ages,
        claims: candidate.claims.clone(),
        figures: ClaimFigures::of(&candidate.projection),
        question: take_question(plan, &candidate.claims),
    }
}

impl ClaimsOptions {
    fn new(plan: &Plan, search: &ClaimSearch) -> Self {
        let baseline = &search.baseline;
        Self {
            columns: claimants(plan, search)
                .into_iter()
                .chain(FIGURES.map(str::to_owned))
                .collect(),
            current: CURRENT_PLAN,
            current_ages: search.current.iter().copied().map(age_cell).collect(),
            baseline: ClaimFigures::of(baseline),
            options: (search.candidates.iter())
                .map(|candidate| option_of(plan, candidate))
                .collect(),
            better: Better::of(|nominal| claims_said(plan, search, baseline, nominal)),
        }
    }
}

/// Every claim age in `text`, a plan, for the household but the people
/// `held`, whose claims stay as the plan states them.
///
/// # Errors
///
/// Where the plan does not pass the gate, or nothing can be searched.
pub fn claims(text: &str, held: &[String]) -> Result<ClaimsOptions, String> {
    let plan = gated(text)?;
    let search =
        optimize_claims(&plan, tables(), &[], held, &Progress::default()).map_err(run_refusal)?;
    Ok(ClaimsOptions::new(&plan, &search))
}

/// The incomes `claims` need that `plan` does not hold: each made up as
/// the search made it.
fn made_up(plan: &Plan, claims: &[Claim]) -> Vec<Income> {
    let missing = claims
        .iter()
        .filter(|claim| plan.income_source(&claim.income).is_none());
    missing.map(|claim| computed_income(&claim.owner)).collect()
}

/// A person as the People table shows them.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PersonRow {
    /// Their id, what `?held` names them by.
    pub id: String,
    /// Their name, which an edit checks they still have.
    pub name: String,
    /// The cells under the table's columns.
    pub cells: Vec<String>,
    /// What can be done for them.
    pub actions: Vec<OfferedAction>,
}

/// An action offered on a person.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OfferedAction {
    /// Which.
    pub action: PersonAction,
    /// What it is called.
    pub label: &'static str,
}

/// A Roth owner, and the account the Overview searches a ladder into.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RothOwner {
    /// Their name.
    pub name: String,
    /// Their first Roth account, by id.
    pub destination: String,
}

/// What the SSA Benefits tool and the Could do better card say of
/// themselves.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimWords {
    /// The People table's columns.
    pub people_columns: [&'static str; 6],
    /// What a claim the plan does not pay says.
    pub no_claim: &'static str,
    /// Before anything is searched.
    pub nothing_searched: &'static str,
    /// Where the household has no one.
    pub nobody: &'static str,
    /// The card's title.
    pub could_do_better: &'static str,
    /// A Roth owner's row where the search is refused.
    pub refused: &'static str,
    /// The card where there is nothing to search.
    pub nothing_to_search: &'static str,
}

impl Document {
    /// Each person's row, their estimates from the last plan without
    /// issues, the people `held` marked so.
    #[must_use]
    pub fn people(&self, held: &[String]) -> Vec<PersonRow> {
        let plan = &self.draft().plan;
        let last_good = self.projected().ok().map(|projected| &projected.plan);
        let people = plan.household.people.iter();
        people
            .map(|person| {
                let is_held = held.contains(&person.id);
                let estimates = last_good.map_or([None; 3], |good| {
                    benefit_estimates(good, tables(), &person.id)
                });
                let actions = PersonAction::ALL
                    .into_iter()
                    .filter(|action| action.is_offered(plan, person, is_held))
                    .map(|action| OfferedAction {
                        action,
                        label: action.label(),
                    });
                PersonRow {
                    id: person.id.clone(),
                    name: person.display_name().to_owned(),
                    cells: person_row(plan, person, is_held, estimates),
                    actions: actions.collect(),
                }
            })
            .collect()
    }

    /// Takes `claims` into the draft as one step of history, adding the
    /// incomes they need, answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take_claims(&mut self, claims: &[Claim]) -> Result<String, String> {
        self.step(|plan| {
            let added = made_up(plan, claims);
            apply_claims(plan, &added, claims);
            Ok(taken(plan, claims))
        })
    }

    /// `claims` as a scenario to be written at `out`, its `base` the
    /// document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// resolved from, or the scenario does not serialize.
    pub fn claims_scenario(&self, out: &str, claims: &[Claim]) -> Result<String, String> {
        let base = self.scenario_base(out)?;
        let added = made_up(&self.draft().plan, claims);
        claims_overlay(&base, &added, claims).map_err(|error| error.to_string())
    }

    /// Records a career at their salary on the person at `index`, still
    /// the one called `name`, as one step of history.
    ///
    /// # Errors
    ///
    /// Where the draft has issues or is read-only, the person is no longer
    /// them, or they have a record.
    pub fn fill_career(&mut self, index: usize, name: &str) -> Result<String, String> {
        if let Some(issue) = self.draft().issues().first() {
            return Err(format!("not filled: {issue}"));
        }
        self.person_step(index, name, |plan, id| {
            claims::fill_career(plan, tables(), id)
        })
    }

    /// The `DESTINATION` the conversion constraints aim at, where they
    /// name one.
    #[must_use]
    pub fn destination(&self) -> Option<String> {
        let answers = self.aimed();
        let destination = answers.get(DESTINATION)?.as_str()?;
        Some(destination.to_owned())
    }

    /// Aims the conversion constraints at `destination`, beside the plan
    /// and outside its history.
    pub fn aim_at(&mut self, destination: &str) {
        let mut answers = self.draft().answers::<Constraints>();
        answers.insert(DESTINATION.to_owned(), destination.into());
        self.hold_answers::<Constraints>(answers);
    }
}

/// `constraints`, the conversion constraints as TOML, aimed at
/// `destination` instead.
///
/// # Errors
///
/// Where they do not read or serialize.
pub fn aimed_at(constraints: &str, destination: &str) -> Result<String, String> {
    let mut answers: toml::Table =
        toml::from_str(constraints).map_err(|error| error.to_string())?;
    answers.insert(DESTINATION.to_owned(), destination.into());
    toml::to_string(&answers).map_err(|error| error.to_string())
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// Each person's row of the SSA Benefits page, the people `held` so
    /// marked.
    ///
    /// # Errors
    ///
    /// Where the rows do not convert.
    #[wasm_bindgen(unchecked_return_type = "PersonRow[]")]
    pub fn people(&self, held: Vec<String>) -> Result<JsValue, JsError> {
        to_js(&self.0.people(&held))
    }

    /// Takes `claims` into the draft as one step of history, answering what
    /// was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, or `claims` are not claims.
    #[wasm_bindgen(js_name = takeClaims)]
    pub fn take_claims(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Claim[]")] claims: JsValue,
    ) -> Result<String, JsError> {
        let claims: Vec<Claim> = from_js(claims)?;
        self.0.take_claims(&claims).map_err(refused)
    }

    /// `claims` as a scenario to be written at `out`, its `base` the
    /// document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, `claims` are not claims, or the scenario does not
    /// serialize.
    #[wasm_bindgen(js_name = claimsScenario)]
    pub fn claims_scenario(
        &self,
        out: &str,
        #[wasm_bindgen(unchecked_param_type = "Claim[]")] claims: JsValue,
    ) -> Result<String, JsError> {
        let claims: Vec<Claim> = from_js(claims)?;
        self.0.claims_scenario(out, &claims).map_err(refused)
    }

    /// Records a career at their salary on the person at `index`, still
    /// `name`, as one step of history, answering what it did.
    ///
    /// # Errors
    ///
    /// Where the draft has issues or is read-only, the person is no longer
    /// them, or they have a record.
    #[wasm_bindgen(js_name = fillCareer)]
    pub fn fill_career(&mut self, index: usize, name: &str) -> Result<String, JsError> {
        self.0.fill_career(index, name).map_err(refused)
    }

    /// Drops the typed figure of the person at `index`'s benefit, still
    /// `name`, as one step of history, answering what it did.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, the person is no longer them, or
    /// nothing is typed.
    #[wasm_bindgen(js_name = computeBenefit)]
    pub fn compute_benefit(&mut self, index: usize, name: &str) -> Result<String, JsError> {
        (self.0)
            .person_step(index, name, claims::compute_benefit)
            .map_err(refused)
    }

    /// Empties the record of the person at `index`, still `name`, as one
    /// step of history, answering what it did.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, the person is no longer them, or they
    /// have no record.
    #[wasm_bindgen(js_name = clearRecord)]
    pub fn clear_record(&mut self, index: usize, name: &str) -> Result<String, JsError> {
        (self.0)
            .person_step(index, name, claims::clear_record)
            .map_err(refused)
    }

    /// Takes the Social Security income of the person at `index`, still
    /// `name`, out of the plan as one step of history, answering what it
    /// did.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, the person is no longer them, or they
    /// have none.
    #[wasm_bindgen(js_name = removeBenefit)]
    pub fn remove_benefit(&mut self, index: usize, name: &str) -> Result<String, JsError> {
        (self.0)
            .person_step(index, name, claims::remove_benefit)
            .map_err(refused)
    }

    /// The account the conversion constraints aim at; `undefined` where
    /// they name none.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn destination(&self) -> Option<String> {
        self.0.destination()
    }

    /// Each person with a Roth account, and the first they own.
    ///
    /// # Errors
    ///
    /// Where the owners do not convert.
    #[wasm_bindgen(js_name = rothOwners, unchecked_return_type = "RothOwner[]")]
    pub fn roth_owners(&self) -> Result<JsValue, JsError> {
        let plan = &self.0.draft().plan;
        let owners = roth_owners(plan).map(|(owner, destination)| RothOwner {
            name: plan.person_name(owner).to_owned(),
            destination: destination.to_owned(),
        });
        to_js(&owners.collect::<Vec<_>>())
    }

    /// Aims the conversion constraints at `destination`, outside the
    /// draft's history.
    #[wasm_bindgen(js_name = aimAt)]
    pub fn aim_at(&mut self, destination: &str) {
        self.0.aim_at(destination);
    }
}

/// What is asked before `name`'s record is cleared.
#[wasm_bindgen(js_name = clearQuestion)]
#[must_use]
pub fn clear_question(name: &str) -> String {
    claims::clear_question(name)
}

/// What is asked before `name`'s Social Security income is removed.
#[wasm_bindgen(js_name = removeQuestion)]
#[must_use]
pub fn remove_question(name: &str) -> String {
    claims::remove_question(name)
}

/// What the SSA Benefits tool and the Could do better card say of
/// themselves.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = claimWords, unchecked_return_type = "ClaimWords")]
pub fn claim_words() -> Result<JsValue, JsError> {
    to_js(&ClaimWords {
        people_columns: PEOPLE_COLUMNS,
        no_claim: NO_CLAIM,
        nothing_searched: NOTHING_SEARCHED,
        nobody: NOBODY,
        could_do_better: COULD_DO_BETTER,
        refused: REFUSED,
        nothing_to_search: NOTHING_TO_SEARCH,
    })
}

/// Every claim age in `plan` for the household but the people `held`, best
/// first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or nothing can be searched.
#[wasm_bindgen(js_name = claims, unchecked_return_type = "ClaimsOptions")]
pub fn js_claims(plan: &str, held: Vec<String>) -> Result<JsValue, JsError> {
    reply(claims(plan, &held))
}

/// `constraints`, the conversion constraints as TOML, aimed at
/// `destination` instead.
///
/// # Errors
///
/// Where they do not read or serialize.
#[wasm_bindgen(js_name = aimedAt)]
pub fn js_aimed_at(constraints: &str, destination: &str) -> Result<String, JsError> {
    aimed_at(constraints, destination).map_err(refused)
}

#[cfg(test)]
mod tests {
    use retiretui_client::files::OVERLAY_SAVE_FIRST;
    use retiretui_client::setup::EXAMPLES;

    use super::*;

    fn example(file: &str) -> &'static str {
        let found = EXAMPLES.iter().find(|(name, ..)| *name == file);
        found.expect("the example").2
    }

    fn opened(file: &str) -> Document {
        let mut read = |_: &std::path::Path| Ok(example(file).to_owned());
        Document::open("/plan.toml", &mut read).expect("opens")
    }

    const STARTER: &str = "starter.toml";

    #[test]
    fn every_option_is_keyed_by_its_ages_and_asked_about_by_name() {
        let reply = claims(example(STARTER), &[]).expect("searches");
        let best = reply.options.first().expect("an option");
        assert_eq!(reply.columns.len(), best.ages.len() + FIGURES.len());
        assert_eq!(best.key, best.ages[0].to_string());
        assert!(
            best.question.starts_with("Take these claims? "),
            "{}",
            best.question
        );
        assert!(!reply.better.today.is_empty() && !reply.better.nominal.is_empty());
        let plan = Plan::from_toml_str(example(STARTER)).expect("parses");
        let held = [plan.household.people[0].id.clone()];
        assert!(
            claims(example(STARTER), &held).is_err(),
            "nothing left to search"
        );
    }

    #[test]
    fn claims_are_taken_as_one_step_and_written_only_over_the_saved_file() {
        let mut document = opened(STARTER);
        let reply = claims(example(STARTER), &[]).expect("searches");
        let chosen = &reply.options[0].claims;
        let said = document.take_claims(chosen).expect("taken");
        assert!(said.starts_with("claimed "), "{said}");
        assert!(document.draft().can_undo());
        assert_eq!(
            document.claims_scenario("/claims.toml", chosen),
            Err(OVERLAY_SAVE_FIRST.to_owned())
        );
        document.save(&mut |_| Ok(())).expect("saved");
        assert!(document.claims_scenario("/plan.toml", chosen).is_err());
        let scenario = document
            .claims_scenario("/claims.toml", chosen)
            .expect("written");
        assert!(scenario.contains("base = \"plan.toml\""), "{scenario}");
        assert!(document.undo());
    }

    #[test]
    fn a_person_is_edited_only_while_they_are_the_one_named() {
        let mut document = opened(STARTER);
        let rows = document.people(&[]);
        let person = &rows[0];
        assert_eq!(person.cells.len(), PEOPLE_COLUMNS.len());
        assert!(
            person
                .actions
                .iter()
                .any(|offered| offered.action == PersonAction::Import)
        );
        assert!(
            document
                .person_step(0, "Someone else", claims::clear_record)
                .is_err()
        );
        let held = document.people(std::slice::from_ref(&person.id));
        let is_held = held[0]
            .actions
            .iter()
            .any(|a| a.action == PersonAction::LetVary);
        assert!(is_held, "a held claim is let vary");
        let name = person.name.clone();
        let removed = document.person_step(0, &name, claims::remove_benefit);
        assert!(removed.is_ok(), "{removed:?}");
        assert!(document.draft().can_undo());
    }

    #[test]
    fn constraints_aimed_where_they_already_aim_are_the_same_text() {
        let mut document = opened("early-retiree.toml");
        let text = document.constraints_text().expect("serializes");
        let destination = document.destination().expect("the lone Roth account");
        assert_eq!(aimed_at(&text, &destination), Ok(text.clone()));
        document.aim_at(&destination);
        assert_eq!(document.constraints_text(), Ok(text));
        assert!(!document.draft().can_undo(), "no step of history");
    }
}
