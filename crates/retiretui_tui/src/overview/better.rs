//! Could do better: each Roth owner's conversion sweep, the household's
//! claim search and the plan from every historical start, searched beside
//! the frames while the Overview is shown - as the Roth
//! Conversions, SSA Benefits and Historical pages search them, under the
//! conversion answers and the claims those pages hold - and the answer
//! kept for what it describes.

use std::collections::BTreeSet;

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Res, ResMut, Resource};
use retiretui_engine::market::{History, Runs};
use retiretui_engine::optimize::ClaimSearch;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use super::rows::{Entry, Tone};
use crate::edit::Draft;
use crate::nav::{Page, ShownSurface};
use crate::session::{Projected, Session};
use crate::tools::claims::HeldClaims;
use crate::tools::ladders::{self, Swept, rate_label};
use crate::tools::markets::MarketHistory;
use crate::tools::{Keyed, Searches};
use retiretui_client::searches::overview::{Found, Searched, beats, gain, search};

const NO_LADDER: &str = "no conversion ladder beats the plan";
const REFUSED: &str = "not searchable under the Roth Conversions answers";
const CLAIMS_AS_PLANNED: &str = "Claims as planned are best";
const NOTHING_TO_SEARCH: &str = "No conversion or claim to search";

/// What the searches found, and what they were made over.
#[derive(Resource, Default)]
pub struct Better {
    answered: Option<(Searched, Found)>,
    running: Option<Keyed<Searched, Option<Found>>>,
}

impl Better {
    /// What was last found, which the Overview drops once it no longer
    /// describes the plan shown.
    pub(crate) fn found(&self) -> Option<&Found> {
        self.answered.as_ref().map(|(_, found)| found)
    }

    /// What was found over `plan`, whatever else it was searched under.
    fn found_over(&self, plan: &Plan) -> Option<(&Searched, &Found)> {
        let (searched, found) = self.answered.as_ref()?;
        (searched.0 == *plan).then_some((searched, found))
    }

    /// The claim search over `plan` with the `held` claims, where it is
    /// answered.
    pub(crate) fn claims(&self, plan: &Plan, held: &BTreeSet<String>) -> Option<&ClaimSearch> {
        let (searched, found) = self.found_over(plan)?;
        if searched.1 != *held {
            return None;
        }
        found.claims.as_ref()
    }

    /// The plan from every start year, where it is answered.
    pub(crate) fn historical(&self, plan: &Plan) -> Option<&Runs> {
        self.found_over(plan)?.1.historical.as_ref()
    }

    /// The ladders into `destination` over `plan` under the `held`
    /// conversion answers, where they are answered.
    pub(crate) fn ladders(
        &self,
        plan: &Plan,
        held: &toml::Table,
        destination: &str,
    ) -> Option<&Swept> {
        let (searched, found) = self.found_over(plan)?;
        if searched.2 != *held {
            return None;
        }
        let mut ladders = found.ladders.iter();
        ladders
            .find(|ladder| ladder.destination == destination)?
            .swept
            .as_ref()
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running.is_some()
    }

    fn receive(&mut self) {
        let Some(running) = self.running.take() else {
            return;
        };
        if let (searched, Some(Some(found)), _) = running.join() {
            self.answered = Some((searched, found));
        }
    }

    /// Searches `wanted` unless it is answered or under way, cloning it
    /// only to start.
    fn queue(
        &mut self,
        (plan, held, answers): (&Plan, &BTreeSet<String>, toml::Table),
        tables: &TaxTables,
        history: &History,
    ) {
        let is_wanted = |searched: &Searched| {
            searched.0 == *plan && searched.1 == *held && searched.2 == answers
        };
        if self
            .answered
            .as_ref()
            .is_some_and(|(searched, _)| is_wanted(searched))
        {
            self.running = None;
            return;
        }
        if self
            .running
            .as_ref()
            .is_some_and(|running| is_wanted(&running.key))
        {
            return;
        }
        self.answered = None;
        let wanted: Searched = (plan.clone(), held.clone(), answers);
        let (tables, history) = (tables.clone(), history.clone());
        let searched = wanted.clone();
        self.running = Some(Keyed::spawn(wanted, move |progress| {
            search(&searched, (&tables, &history), progress)
        }));
    }
}

/// Takes the answer as it lands, and searches the plan shown while the
/// Overview is, dropping an answer that describes another.
pub(super) fn work(
    (projected, shown, searches): (Res<Projected>, ShownSurface, Res<Searches>),
    (session, history, held): (Res<Session>, Res<MarketHistory>, Res<HeldClaims>),
    (draft, mut better): (Res<Draft>, ResMut<Better>),
) {
    if better.running.as_ref().is_some_and(Keyed::is_finished) {
        better.receive();
    }
    let is_moved = projected.is_changed()
        || shown.is_changed()
        || searches.is_changed()
        || held.is_changed()
        || draft.is_changed();
    if !is_moved {
        return;
    }
    if shown.surface() != Some(Page::Overview) || !searches.0 {
        better.running = None;
        return;
    }
    let wanted = (&projected.plan, &held.0, ladders::held_answers(&draft));
    better.queue(wanted, &session.tables, &history.0);
}

/// The Could do better rows: each Roth owner's best ladder, then the best
/// claims, each against the plan as it stands.
pub(super) fn entries(better: &Better, projected: &Projected, nominal: bool) -> Vec<Entry> {
    let Some(found) = better.found() else {
        return vec![quiet(super::PENDING)];
    };
    let plan = &projected.plan;
    let current = &projected.projection;
    let mut rows: Vec<Entry> = found
        .ladders
        .iter()
        .map(|ladder| {
            let said = match ladder.swept.as_ref().and_then(Swept::best) {
                None => REFUSED.to_owned(),
                Some(best) if beats(&best.optimized, current) => {
                    let rate = rate_label(best.rate);
                    format!(
                        "convert to {rate}, {}",
                        gain(&best.optimized, current, nominal)
                    )
                }
                Some(_) => NO_LADDER.to_owned(),
            };
            let text = format!("{}: {said}", plan.person_name(&ladder.owner));
            let aim = Some(ladder.destination.clone());
            Entry::leading(text, (Page::RothConversions, aim))
        })
        .collect();
    rows.extend(found.claims.as_ref().map(|search| {
        let best = search.best();
        let text = if beats(&best.projection, current) {
            let claims: Vec<String> = (best.claims.iter())
                .map(|claim| format!("{} at {}", plan.person_name(&claim.owner), claim.age))
                .collect();
            let gain = gain(&best.projection, current, nominal);
            format!("Claim {}: {gain}", claims.join(", "))
        } else {
            CLAIMS_AS_PLANNED.to_owned()
        };
        Entry::leading(text, (Page::SsaBenefits, None))
    }));
    if rows.is_empty() {
        rows.push(quiet(NOTHING_TO_SEARCH));
    }
    rows
}

fn quiet(text: &str) -> Entry {
    Entry {
        tone: Tone::Quiet,
        ..Entry::plain(text.to_owned())
    }
}
