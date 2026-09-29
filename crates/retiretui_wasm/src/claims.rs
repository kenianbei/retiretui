//! The SSA Benefits tool for a page: every claim age ranked for the
//! household, each person's record and estimated benefit with what can be
//! done for them, the chosen claims taken into the draft or written as a
//! scenario; and what the Overview's Could do better card asks of the
//! document.

use retiretui_client::searches::claims::{
    NOBODY, NOTHING_SEARCHED, PEOPLE_COLUMNS, PersonAction, SPELLED_OUT, age_cell, option_columns,
    person_row, take_question, taken,
};
use retiretui_client::searches::overview::{
    COULD_DO_BETTER, NOTHING_TO_SEARCH, REFUSED, claims_said, roth_owners,
};
use retiretui_client::searches::{AGAINST_PLAN, CURRENT_PLAN, option_cells, page_refusal};
use retiretui_engine::market::Progress;
use retiretui_engine::optimize::{
    Claim, ClaimCandidate, ClaimSearch, apply_claims, claims_overlay, optimize_claims,
};
use retiretui_engine::plan::{Income, Item, Plan};
use retiretui_engine::project::Summary;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::searches::gated;
use crate::{Bases, JsDocument, from_js, option_figures, refused, reply, tables, to_js};

/// One set of claims the search tried.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimOption {
    /// Its ages joined, as the address holds it: `70-67`.
    pub key: String,
    /// One claim per claimant, in the reply's column order.
    pub claims: Vec<Claim>,
    /// The plan's figures under them.
    pub figures: Bases<Vec<String>>,
    /// What is asked before they are taken into the plan searched.
    pub question: String,
}

/// Every claim age for the household, best first, beside the plan as it
/// stands.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimsOptions {
    /// Each person claimed for, by name, then what an option ends with
    /// against the plan, then each figure.
    pub columns: Vec<String>,
    /// What the plan's own row is called.
    pub current: &'static str,
    /// The age each claim is at in the plan as it stands, or what an
    /// unpaid one says.
    pub current_ages: Vec<String>,
    /// The plan's figures as it stands.
    pub baseline: Bases<Vec<String>>,
    /// Every set of claims, best first.
    pub options: Vec<ClaimOption>,
    /// The incomes the search made up for people with a record and none,
    /// handed back with the claims taken or written.
    #[cfg_attr(feature = "ts", ts(type = "unknown[]"))]
    pub added: Vec<Income>,
    /// What the best claims do better than the plan, in either basis.
    pub better: Bases<String>,
}

fn option_of(plan: &Plan, candidate: &ClaimCandidate, summaries: &Bases<Summary>) -> ClaimOption {
    let ages = candidate.claims.iter().map(|claim| claim.age.to_string());
    ClaimOption {
        key: ages.collect::<Vec<_>>().join("-"),
        claims: candidate.claims.clone(),
        figures: option_figures(option_cells, &candidate.projection, Some(summaries)),
        question: take_question(plan, &candidate.claims),
    }
}

impl ClaimsOptions {
    fn new(plan: &Plan, search: ClaimSearch) -> Self {
        let baseline = &search.baseline;
        let better = Bases::of(|nominal| claims_said(plan, &search, baseline, nominal));
        let summaries = Bases::of(|nominal| baseline.summary(!nominal));
        Self {
            columns: option_columns(plan, &search),
            current: CURRENT_PLAN,
            current_ages: search.current.iter().copied().map(age_cell).collect(),
            baseline: option_figures(option_cells, baseline, None),
            options: (search.candidates.iter())
                .map(|candidate| option_of(plan, candidate, &summaries))
                .collect(),
            better,
            added: search.added,
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
        optimize_claims(&plan, tables(), &[], held, &Progress::default()).map_err(page_refusal)?;
    Ok(ClaimsOptions::new(&plan, search))
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
    /// What is asked before it is done, where it drops something.
    pub question: Option<String>,
    /// The answer that does it, where it is asked about.
    pub answer: Option<&'static str>,
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
    /// What the tool is for, in a line.
    pub about: &'static str,
    /// The People table's columns.
    pub people_columns: [&'static str; 6],
    /// Its abbreviated columns, each with its words in full.
    pub spelled_out: &'static [(&'static str, &'static str)],
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
    /// The column a phone's row shows beside the ages.
    pub against_plan: &'static str,
}

impl Document {
    /// Each person's row, their estimates from the last plan without
    /// issues, the people `held` marked so.
    #[must_use]
    pub fn people(&self, held: &[String]) -> Vec<PersonRow> {
        let plan = &self.draft().plan;
        let people = plan.household.people.iter();
        people
            .map(|person| {
                let is_held = held.contains(&person.id);
                let name = person.display_name();
                let actions = PersonAction::ALL
                    .into_iter()
                    .filter(|action| action.is_offered(plan, person, is_held))
                    .map(|action| OfferedAction {
                        action,
                        label: action.label(),
                        question: action.question(name),
                        answer: action.answer(),
                    });
                PersonRow {
                    id: person.id.clone(),
                    name: name.to_owned(),
                    cells: person_row(plan, person, is_held, self.estimate(&person.id)),
                    actions: actions.collect(),
                }
            })
            .collect()
    }

    /// Takes `claims` into the draft as one step of history, adding the
    /// incomes of `added` they need, answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take_claims(&mut self, claims: &[Claim], added: &[Income]) -> Result<String, String> {
        self.step(|plan| {
            apply_claims(plan, added, claims);
            Ok(taken(plan, claims))
        })
    }

    /// `claims` as a scenario to be written at `out`, its `base` the
    /// document's file, adding the incomes of `added` they need.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// resolved from, or the scenario does not serialize.
    pub fn claims_scenario(
        &self,
        out: &str,
        claims: &[Claim],
        added: &[Income],
    ) -> Result<String, String> {
        let base = self.scenario_base(out)?;
        claims_overlay(&base, added, claims).map_err(|error| error.to_string())
    }

    /// Does `action` to the person at `index`, still the one called
    /// `name`, as one step of history, answering what it did.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, `action` is not an edit of the plan or
    /// estimates a record from a plan with issues, the person is no longer
    /// them, or they have no use for it.
    pub fn act(
        &mut self,
        action: PersonAction,
        index: usize,
        name: &str,
    ) -> Result<String, String> {
        if action == PersonAction::FillCareer
            && let Some(issue) = self.draft().issues().first()
        {
            return Err(format!("not filled: {issue}"));
        }
        self.person_step(index, name, |plan, id| {
            let edit = action.apply(plan, tables(), id);
            edit.unwrap_or_else(|| Err(format!("{} is not an edit", action.label())))
        })
    }
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

    /// Takes `claims` into the draft as one step of history, with the
    /// incomes the search `added`, answering what was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, or `claims` or `added` are not what a
    /// claim search replied.
    #[wasm_bindgen(js_name = takeClaims)]
    pub fn take_claims(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Claim[]")] claims: JsValue,
        #[wasm_bindgen(unchecked_param_type = "unknown[]")] added: JsValue,
    ) -> Result<String, JsError> {
        let (claims, added): (Vec<Claim>, Vec<Income>) = (from_js(claims)?, from_js(added)?);
        self.0.take_claims(&claims, &added).map_err(refused)
    }

    /// `claims` as a scenario to be written at `out`, its `base` the
    /// document's file, with the incomes the search `added`.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, `claims` or `added` are not what a claim search replied,
    /// or the scenario does not serialize.
    #[wasm_bindgen(js_name = claimsScenario)]
    pub fn claims_scenario(
        &self,
        out: &str,
        #[wasm_bindgen(unchecked_param_type = "Claim[]")] claims: JsValue,
        #[wasm_bindgen(unchecked_param_type = "unknown[]")] added: JsValue,
    ) -> Result<String, JsError> {
        let (claims, added): (Vec<Claim>, Vec<Income>) = (from_js(claims)?, from_js(added)?);
        self.0
            .claims_scenario(out, &claims, &added)
            .map_err(refused)
    }

    /// Does `action` to the person at `index`, still `name`, as one step of
    /// history, answering what it did.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, `action` is not an edit of the plan,
    /// the person is no longer them, or they have no use for it.
    pub fn act(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "PersonAction")] action: JsValue,
        index: usize,
        name: &str,
    ) -> Result<String, JsError> {
        self.0.act(from_js(action)?, index, name).map_err(refused)
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

/// What the SSA Benefits tool and the Could do better card say of
/// themselves.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = claimWords, unchecked_return_type = "ClaimWords")]
pub fn claim_words() -> Result<JsValue, JsError> {
    to_js(&ClaimWords {
        about: retiretui_client::searches::claims::ABOUT,
        people_columns: PEOPLE_COLUMNS,
        spelled_out: SPELLED_OUT,
        nothing_searched: NOTHING_SEARCHED,
        nobody: NOBODY,
        could_do_better: COULD_DO_BETTER,
        refused: REFUSED,
        nothing_to_search: NOTHING_TO_SEARCH,
        against_plan: AGAINST_PLAN,
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

#[cfg(test)]
mod tests {
    use retiretui_client::files::OVERLAY_SAVE_FIRST;
    use retiretui_client::searches::FIGURES;
    use retiretui_client::setup::EXAMPLES;

    use super::*;

    const STARTER: &str = "starter.toml";

    fn example(file: &str) -> &'static str {
        let found = EXAMPLES.iter().find(|(name, ..)| *name == file);
        found.expect("the example").2
    }

    fn opened(file: &str) -> Document {
        let mut read = |_: &std::path::Path| Ok(example(file).to_owned());
        Document::open("/plan.toml", &mut read).expect("opens")
    }

    #[test]
    fn every_option_is_keyed_by_its_ages_and_asked_about_by_name() {
        let reply = claims(example(STARTER), &[]).expect("searches");
        let best = reply.options.first().expect("an option");
        assert_eq!(reply.columns.len(), best.claims.len() + 1 + FIGURES.len());
        assert_eq!(reply.columns[best.claims.len()], AGAINST_PLAN);
        assert_eq!(best.figures.today.len(), 1 + FIGURES.len());
        assert_eq!(best.key, best.claims[0].age.to_string());
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
        let added = &reply.added;
        let said = document.take_claims(chosen, added).expect("taken");
        assert!(said.starts_with("claimed "), "{said}");
        assert!(document.draft().can_undo());
        assert_eq!(
            document.claims_scenario("/claims.toml", chosen, added),
            Err(OVERLAY_SAVE_FIRST.to_owned())
        );
        document.save(&mut |_| Ok(())).expect("saved");
        assert!(
            document
                .claims_scenario("/plan.toml", chosen, added)
                .is_err()
        );
        let scenario = document
            .claims_scenario("/claims.toml", chosen, added)
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
        let offers = |row: &PersonRow, wanted: PersonAction| {
            row.actions.iter().any(|offered| offered.action == wanted)
        };
        assert!(offers(person, PersonAction::Import));
        let held = document.people(std::slice::from_ref(&person.id));
        assert!(
            offers(&held[0], PersonAction::LetVary),
            "a held claim is let vary"
        );
        let removal = PersonAction::RemoveBenefit;
        assert!(document.act(removal, 0, "Someone else").is_err());
        assert!(document.act(PersonAction::Hold, 0, &person.name).is_err());
        let removed = document.act(removal, 0, &person.name);
        assert!(removed.is_ok(), "{removed:?}");
        assert!(document.draft().can_undo());
    }

    #[test]
    fn aiming_the_constraints_leaves_their_text_and_history_alone() {
        let mut document = opened("early-retiree.toml");
        let text = document.constraints_text().expect("serializes");
        let destination = document.destination().expect("the lone Roth account");
        document.aim_at(&destination);
        assert_eq!(document.constraints_text(), Ok(text));
        assert_eq!(document.destination(), Some(destination));
        assert!(!document.draft().can_undo(), "no step of history");
    }
}
