//! Could do better: each Roth owner's conversion sweep, the household's
//! claim search, the order search and the plan from every historical start,
//! searched beside the frames while the Overview is shown - as the Roth
//! Conversions, SSA Benefits, Withdrawal Order and Historical pages search
//! them, under the conversion answers and the claims those pages hold - and
//! the answer kept for what it describes. The spending ceiling is searched
//! apart, after the rest and the market runs: it is many times their work.

use std::collections::BTreeSet;

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Res, ResMut, Resource};
use retiretui_engine::market::{History, RunError, Runs};
use retiretui_engine::optimize::{ClaimSearch, OrderSearch};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use super::rows::Entry;
use crate::edit::Draft;
use crate::nav::{Page, ShownSurface};
use crate::session::{Projected, Session};
use crate::success::Successes;
use crate::tools::claims::HeldClaims;
use crate::tools::ladders::{self, Swept};
use crate::tools::markets::MarketHistory;
use crate::tools::spending::Ceilings;
use crate::tools::{Keyed, Searches};
use retiretui_client::searches::overview::{
    Found, NOTHING_TO_SEARCH, Searched, claims_said, ladder_said, order_said, search, spending_said,
};
use retiretui_client::searches::spending::{self, Answers};

/// What the searches found, and what they were made over.
#[derive(Resource, Default)]
pub struct Better {
    answered: Option<(Searched, Found)>,
    running: Option<Keyed<Searched, Option<Found>>>,
    ceiling: Ceiling,
}

/// A plan and the Spending Ceiling answers it is searched under.
type Asked = (Plan, toml::Table);

/// The spending ceiling's search: its answer, none where the search
/// refused the plan, and the search under way.
#[derive(Default)]
struct Ceiling {
    answered: Option<(Asked, Option<Ceilings>)>,
    running: Option<Keyed<Asked, Result<Ceilings, RunError>>>,
}

/// Whether the ceiling is searched from here at all: where there are no
/// threads a search runs whole inside a frame, and this one would hold the
/// page for seconds.
const IS_CEILING_SEARCHED: bool = cfg!(not(target_arch = "wasm32"));

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

    /// The order search over `plan`, where it is answered.
    pub(crate) fn order(&self, plan: &Plan) -> Option<&OrderSearch> {
        self.found_over(plan)?.1.order.as_ref()
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

    /// The ceilings over `plan` under the `held` Spending Ceiling answers,
    /// where they are answered.
    pub(crate) fn spending(&self, plan: &Plan, held: &toml::Table) -> Option<&Ceilings> {
        let ((searched, answers), found) = self.ceiling.answered.as_ref()?;
        (searched == plan && answers == held).then_some(found.as_ref())?
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running.is_some() || self.ceiling.running.is_some()
    }

    /// Takes the ceiling's answer; a cancelled search has none.
    fn receive_ceiling(&mut self) {
        let Some(running) = self.ceiling.running.take() else {
            return;
        };
        let (asked, answer, _) = running.join();
        let found = match answer {
            Some(Err(RunError::Cancelled)) => return,
            Some(Ok(found)) => Some(found),
            Some(Err(RunError::Refused(_))) | None => None,
        };
        self.ceiling.answered = Some((asked, found));
    }

    /// Searches the ceiling over `plan` under `answers` unless it is
    /// answered or under way.
    fn queue_ceiling(
        &mut self,
        (plan, answers): (&Plan, toml::Table),
        tables: &TaxTables,
        history: &History,
    ) {
        let is_asked = |(searched, held): &Asked| searched == plan && *held == answers;
        let ceiling = &self.ceiling;
        let is_answered = ceiling
            .answered
            .as_ref()
            .is_some_and(|(asked, _)| is_asked(asked));
        let is_running = ceiling
            .running
            .as_ref()
            .is_some_and(|running| is_asked(&running.key));
        if is_answered || is_running {
            return;
        }
        let target = spending::target_in(answers.clone());
        let (tables, history) = (tables.clone(), history.clone());
        let searched = plan.clone();
        self.ceiling.answered = None;
        self.ceiling.running = Some(Keyed::spawn((plan.clone(), answers), move |progress| {
            spending::search(&searched, &tables, &history, target, progress)
        }));
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

/// Takes each answer as it lands, and searches the plan shown while the
/// Overview is, dropping an answer that describes another: the ceiling
/// once the rest and the market runs have answered, so that they do not
/// share the machine with it.
pub(super) fn work(
    (projected, shown, searches): (Res<Projected>, ShownSurface, Res<Searches>),
    (session, history, held): (Res<Session>, Res<MarketHistory>, Res<HeldClaims>),
    (draft, successes, mut better): (Res<Draft>, Res<Successes>, ResMut<Better>),
) {
    let has_answered = better.running.as_ref().is_some_and(Keyed::is_finished);
    if has_answered {
        better.receive();
    }
    let has_ceiling = (better.ceiling.running.as_ref()).is_some_and(Keyed::is_finished);
    if has_ceiling {
        better.receive_ceiling();
    }
    let is_moved = projected.is_changed()
        || shown.is_changed()
        || searches.is_changed()
        || held.is_changed()
        || draft.is_changed()
        || successes.is_changed()
        || has_answered;
    if !is_moved {
        return;
    }
    if shown.surface() != Some(Page::Overview) || !searches.0 {
        if better.running.is_some() || better.ceiling.running.is_some() {
            better.running = None;
            better.ceiling.running = None;
        }
        return;
    }
    let plan = &projected.plan;
    let (tables, history) = (&session.tables, &history.0);
    better.queue(
        (plan, &held.0, ladders::held_answers(&draft)),
        tables,
        history,
    );
    let is_clear = better.running.is_none() && successes.has_answered_for(plan);
    if IS_CEILING_SEARCHED && is_clear && draft.issues().is_empty() {
        better.queue_ceiling((plan, draft.answers::<Answers>()), tables, history);
    }
}

/// The Could do better rows: each Roth owner's best ladder, then the best
/// claims and the best order, each against the plan as it stands, and what
/// the Spending Ceiling tool finds the plan could spend.
pub(super) fn entries(better: &Better, projected: &Projected, nominal: bool) -> Vec<Entry> {
    let Some(found) = better.found() else {
        return vec![Entry::quiet(super::PENDING)];
    };
    let plan = &projected.plan;
    let current = &projected.projection;
    let mut rows: Vec<Entry> = found
        .ladders
        .iter()
        .map(|ladder| {
            let said = ladder_said(ladder.swept.as_ref(), current, nominal);
            let text = format!("{}: {said}", plan.person_name(&ladder.owner));
            let aim = Some(ladder.destination.clone());
            Entry::leading(text, (Page::RothConversions, aim))
        })
        .collect();
    rows.extend(found.claims.as_ref().map(|search| {
        let text = claims_said(plan, search, current, nominal);
        Entry::leading(text, (Page::SsaBenefits, None))
    }));
    rows.extend(found.order.as_ref().map(|search| {
        let text = order_said(search, nominal);
        Entry::leading(text, (Page::WithdrawalOrder, None))
    }));
    if rows.is_empty() {
        rows.push(Entry::quiet(NOTHING_TO_SEARCH));
    }
    rows.extend(better.ceiling.said(plan));
    rows
}

impl Ceiling {
    /// What `plan` could spend at the tool's target, leading to the tool:
    /// pending until it is answered, and no row where it is not searched
    /// or the plan has no spending to scale.
    fn said(&self, plan: &Plan) -> Option<Entry> {
        let answered = (self.answered.as_ref()).filter(|((searched, _), _)| searched == plan);
        match answered {
            Some((_, found)) => {
                let said = spending_said(found.as_ref()?, plan);
                Some(Entry::leading(said, (Page::SpendingCeiling, None)))
            }
            None if IS_CEILING_SEARCHED => Some(Entry::quiet(super::PENDING)),
            None => None,
        }
    }
}
