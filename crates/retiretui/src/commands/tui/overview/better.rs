//! Could do better: each Roth owner's conversion sweep, the household's
//! claim search and the plan from every historical start, searched on a
//! thread of the Overview's own while it is shown - as the Roth
//! Conversions, SSA Benefits and Historical pages search them, under the
//! conversion answers and the claims those pages hold - and the answer
//! kept for what it describes.

use std::collections::BTreeSet;

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Res, ResMut, Resource};
use retiretui_engine::market::{History, Progress, RunError, Runs, historical};
use retiretui_engine::optimize::{ClaimSearch, SweptBracket, optimize_claims, rank_key};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Plan, TreatmentClass};
use retiretui_engine::project::Projection;

use super::rows::{Entry, Tone};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::present::signed_money;
use crate::commands::tui::session::{Projected, Session};
use crate::commands::tui::tools::claims::HeldClaims;
use crate::commands::tui::tools::ladders::{self, rate_label};
use crate::commands::tui::tools::markets::MarketHistory;
use crate::commands::tui::tools::{Searches, Worker};

const NO_LADDER: &str = "no conversion ladder beats the plan";
const REFUSED: &str = "not searchable under the Roth Conversions answers";
const CLAIMS_AS_PLANNED: &str = "Claims as planned are best";
const NOTHING_TO_SEARCH: &str = "No conversion or claim to search";

/// A plan, the people whose claims are held as it states them, and the
/// conversion answers held but the destination.
type Searched = (Plan, BTreeSet<String>, toml::Table);

/// What the searches found, and what they were made over.
#[derive(Resource, Default)]
pub struct Better {
    answered: Option<(Searched, Found)>,
    running: Option<(Searched, Worker<Option<Found>>)>,
}

pub(crate) struct Found {
    /// The plan from every start year, where it could be run.
    pub(super) historical: Option<Runs>,
    /// The claim search, where anything computes a benefit.
    claims: Option<ClaimSearch>,
    ladders: Vec<Ladder>,
}

/// A Roth owner's best ladder into their Roth account, `None` where the
/// search is refused under the page's answers.
struct Ladder {
    owner: String,
    destination: String,
    best: Option<SweptBracket>,
}

impl Better {
    /// What was last found, which the Overview drops once it no longer
    /// describes the plan shown.
    pub(crate) fn found(&self) -> Option<&Found> {
        self.answered.as_ref().map(|(_, found)| found)
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running.is_some()
    }

    fn receive(&mut self) {
        let Some((searched, worker)) = self.running.take() else {
            return;
        };
        if let Some(Some(found)) = worker.join() {
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
            .is_some_and(|(searched, _)| is_wanted(searched))
        {
            return;
        }
        self.answered = None;
        let wanted: Searched = (plan.clone(), held.clone(), answers);
        let (tables, history) = (tables.clone(), history.clone());
        let searched = wanted.clone();
        let worker =
            Worker::spawn(move |progress| search(&searched, (&tables, &history), progress));
        self.running = Some((wanted, worker));
    }
}

/// Every search, the cheapest first; none once cancelled, which is
/// checked between them.
fn search(
    (plan, held, answers): &Searched,
    (tables, history): (&TaxTables, &History),
    progress: &Progress,
) -> Option<Found> {
    let historical = match historical(plan, tables, history, progress) {
        Err(RunError::Cancelled) => return None,
        ran => ran.ok(),
    };
    let held: Vec<String> = held.iter().cloned().collect();
    let claims = unless_cancelled(progress, || optimize_claims(plan, tables, &[], &held).ok())?;
    let mut ladders = Vec::new();
    for owner in roth_owners(plan) {
        let best = unless_cancelled(progress, || best_ladder(plan, tables, answers, owner))?;
        ladders.push(best);
    }
    Some(Found {
        historical,
        claims,
        ladders,
    })
}

/// `work`, unless the search is cancelled before it starts.
fn unless_cancelled<T>(progress: &Progress, work: impl FnOnce() -> T) -> Option<T> {
    (!progress.is_cancelled()).then(work)
}

/// The best ladder into `owner`'s Roth account, searched as the Roth
/// Conversions page searches under the `answers` it holds.
fn best_ladder(
    plan: &Plan,
    tables: &TaxTables,
    answers: &toml::Table,
    (owner, destination): (&str, &str),
) -> Ladder {
    let best = ladders::options_into(plan, answers, destination)
        .and_then(|(options, rate)| ladders::search(plan, tables, &options, rate).ok())
        .and_then(|sweep| sweep.brackets.into_iter().next());
    Ladder {
        owner: owner.to_owned(),
        destination: destination.to_owned(),
        best,
    }
}

/// Each person with a Roth account, and the first they own.
fn roth_owners(plan: &Plan) -> impl Iterator<Item = (&str, &str)> {
    plan.household.people.iter().filter_map(|person| {
        let roth = plan.accounts.iter().find(|account| {
            account.owner == person.id && account.treatment() == TreatmentClass::Roth
        })?;
        Some((person.id.as_str(), roth.id.as_str()))
    })
}

/// Takes the answer as it lands, and searches the plan shown while the
/// Overview is, dropping an answer that describes another.
pub(super) fn work(
    (projected, active, searches): (Res<Projected>, Res<ActivePage>, Res<Searches>),
    (session, history, held): (Res<Session>, Res<MarketHistory>, Res<HeldClaims>),
    (draft, mut better): (Res<Draft>, ResMut<Better>),
) {
    if better
        .running
        .as_ref()
        .is_some_and(|(_, worker)| worker.is_finished())
    {
        better.receive();
    }
    let is_moved = projected.is_changed()
        || active.is_changed()
        || searches.is_changed()
        || held.is_changed()
        || draft.is_changed();
    if !is_moved {
        return;
    }
    if active.0 != Page::Overview || !searches.0 {
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
            let said = match &ladder.best {
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

/// Whether `option` ranks ahead of `current`, as the tools rank options.
fn beats(option: &Projection, current: &Projection) -> bool {
    rank_key(option) < rank_key(current)
}

/// What `option` ends with against `current`, and leaves unfunded where
/// that differs.
fn gain(option: &Projection, current: &Projection, nominal: bool) -> String {
    let (own, base) = (option.summary(!nominal), current.summary(!nominal));
    let ends = signed_money(own.final_net_worth - base.final_net_worth);
    let unfunded = own.lifetime_unfunded - base.lifetime_unfunded;
    if unfunded == 0 {
        format!("ends {ends}")
    } else {
        format!("ends {ends}, unfunded {}", signed_money(unfunded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::support::{TEST_PLAN, projected_from};

    /// A plan whose historical runs are refused without a look at the
    /// progress, so only the checks between the steps can stop it.
    #[test]
    fn a_cancelled_search_stops_before_the_next_step() {
        let refused = format!("{TEST_PLAN}\n[market.historical]\nfrom = 1800\n");
        let searched = (
            projected_from(&refused).plan,
            BTreeSet::new(),
            toml::Table::new(),
        );
        let tables = (&TaxTables::embedded(), History::embedded());
        let progress = Progress::default();
        let found = search(&searched, tables, &progress).expect("searched");
        assert!(found.historical.is_none(), "the runs were refused");
        progress.cancel();
        assert!(search(&searched, tables, &progress).is_none());
    }
}
