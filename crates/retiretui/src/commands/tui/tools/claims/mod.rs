//! The SSA Benefits tool: every computed Social Security benefit tried at
//! each whole age it can still be claimed at, jointly for the household,
//! ranked by what the household ends with.

mod commands;
mod guide;
#[cfg(test)]
mod guide_tests;
mod options;
mod people;
#[cfg(test)]
mod people_tests;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::path::PathBuf;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{In, IntoScheduleConfigs, Local, Res, ResMut, World};
use retiretui_engine::optimize::{
    Claim, ClaimCandidate, ClaimSearch, apply_claims, claims_overlay, optimize_claims,
};
use retiretui_engine::plan::{Income, Plan};

use super::options::{CURRENT_PLAN, FIGURES, Laid, figures};
use super::{Found, NOTHING_SEARCHED_YET, Tool, ToolPage, write};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::confirm::{Answer, Confirm};
use crate::commands::tui::documents::{Browsing, Pickers};
use crate::commands::tui::edit::{Draft, DraftEditor};
use crate::commands::tui::journal;
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::session::Session;
pub(crate) use people::HeldClaims;

pub type Claims = Tool<ClaimSearch>;

pub use commands::{
    clear_record, compute_benefit, fill_career, hold_claim, import_statement, remove_benefit,
};
pub use guide::offer_actions;

pub fn plugin(app: &mut App) {
    super::install::<ClaimSearch>(app, &PAGE);
    app.add_systems(
        Update,
        search_by_itself.before(super::poll_search::<ClaimSearch>),
    );
    people::plugin(app);
    super::options::plugin::<ClaimSearch>(app);
    guide::plugin(app);
}

const PAGE: ToolPage = ToolPage {
    surface: Page::SsaBenefits,
    panes: |commands, row| {
        people::spawn_pane(commands, row);
        options::spawn_pane(commands, row);
    },
};

/// What a claim the plan does not pay says.
const NO_CLAIM: &str = "none";

impl Found for ClaimSearch {
    const NOTHING_SEARCHED: &'static str = "Every age each computed Social Security benefit can be claimed at is ranked here, jointly for the household, as soon as the plan is valid.";

    /// The header names each person, then the figures; the plan's own row
    /// gives each claim age the plan states, and each option the ages tried.
    fn laid(&self, plan: &Plan, is_nominal: bool) -> Laid {
        let deflated = !is_nominal;
        let owners: Vec<String> = self
            .candidates
            .first()
            .map(|best| {
                best.claims
                    .iter()
                    .map(|claim| plan.person_name(&claim.owner).to_owned())
                    .collect()
            })
            .unwrap_or_default();
        let header = std::iter::once(String::new())
            .chain(owners)
            .chain(FIGURES.map(str::to_owned))
            .collect();
        let ages = self
            .current
            .iter()
            .map(|age| age.map_or_else(|| NO_CLAIM.to_owned(), |age| age.to_string()));
        let current = std::iter::once(CURRENT_PLAN.to_owned())
            .chain(ages)
            .chain(figures(&self.baseline.summary(deflated)))
            .collect();
        let options = self
            .candidates
            .iter()
            .map(|candidate| {
                let ages = candidate.claims.iter().map(|claim| claim.age.to_string());
                std::iter::once(String::new())
                    .chain(ages)
                    .chain(figures(&candidate.projection.summary(deflated)))
                    .collect()
            })
            .collect();
        Laid {
            header,
            current,
            options,
        }
    }
}

impl Tool<ClaimSearch> {
    /// What was found and the candidate chosen of it: the highlighted one,
    /// or the best while the baseline is highlighted.
    fn chosen(&self) -> Option<(&ClaimSearch, &ClaimCandidate)> {
        let found = self.found()?;
        let highlighted = self.highlighted().and_then(|at| found.candidates.get(at));
        Some((found, highlighted.unwrap_or_else(|| found.best())))
    }
}

/// Searches again whenever the page is on show over a valid draft whose plan
/// differs from the last it searched, so the ranking is never asked for.
fn search_by_itself(
    (draft, held): (Res<Draft>, Res<HeldClaims>),
    session: Res<Session>,
    active: Res<ActivePage>,
    mut searched: Local<Option<(Plan, BTreeSet<String>)>>,
    mut claims: ResMut<Claims>,
) {
    let is_moved =
        draft.is_changed() || active.is_changed() || claims.is_changed() || held.is_changed();
    let is_ready = is_moved && active.page() == Page::SsaBenefits && !claims.is_running();
    let is_same = |(plan, ids): &(Plan, BTreeSet<String>)| *plan == draft.plan && *ids == held.0;
    if !is_ready || !super::is_due(&draft, searched.as_ref(), is_same) {
        return;
    }
    *searched = Some((draft.plan.clone(), held.0.clone()));
    let tables = session.tables.clone();
    let held: Vec<String> = held.0.iter().cloned().collect();
    claims.start(draft.plan.clone(), move |plan| {
        optimize_claims(plan, &tables, &[], &held)
    });
}

/// The `take-claims` command: asks before moving each searched income's
/// `start` to the chosen candidate's claim - the highlighted one, or the
/// best while the baseline is highlighted - adding the incomes the search
/// made up. The answer carries the claims asked about, so a highlight that
/// moves under the question changes nothing.
pub fn adopt(claims: Res<Claims>, draft: Res<Draft>, mut confirm: ResMut<Confirm>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return refusal;
    }
    let Some((found, candidate)) = claims.chosen() else {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    };
    let asked = (found.added.clone(), candidate.claims.clone());
    let answers = vec![
        Answer::closing("Cancel"),
        Answer::running("Take", move |commands| {
            commands.run_system_cached_with(take, asked);
        })
        .primary(),
    ];
    confirm.ask_among(
        format!("Take these claims? {}.", said(&candidate.claims)),
        answers,
    );
    Outcome::Done
}

/// Takes `claims` into the draft, as one step of history.
fn take(In((added, claims)): In<(Vec<Income>, Vec<Claim>)>, mut editor: DraftEditor) {
    apply_claims(&mut editor.draft.plan, &added, &claims);
    editor.commit();
    journal::say(format!("claimed {}", said(&claims)));
}

/// Claims as a sentence says them: `ss-me at 70, ss-you at 67`.
fn said(claims: &[Claim]) -> String {
    let each: Vec<String> = claims
        .iter()
        .map(|claim| format!("{} at {}", claim.income, claim.age))
        .collect();
    each.join(", ")
}

/// The `write-claims` command: asks where to write the highlighted
/// candidate's claims.
pub fn write_picker(
    pickers: Res<Pickers>,
    claims: Res<Claims>,
    draft: Res<Draft>,
    mut browsing: ResMut<Browsing>,
) -> Outcome {
    if claims.found().is_none() {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    }
    write::open_picker(pickers.claims, &draft, &mut browsing)
}

/// Writes the highlighted claims at `path` as a scenario over the
/// document, and compares the file written.
pub fn write_overlay(In(path): In<PathBuf>, world: &mut World) {
    let text = write::base_of(world, &path).and_then(|base| {
        let claims = world.resource::<Claims>();
        let (found, candidate) = claims
            .chosen()
            .ok_or_else(|| NOTHING_SEARCHED_YET.to_owned())?;
        claims_overlay(&base, &found.added, &candidate.claims)
            .map_err(|error| format!("not written: {error}"))
    });
    write::write(world, path, text);
}
