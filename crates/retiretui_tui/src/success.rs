//! Each plan's Monte Carlo success, under its own `[market]` settings: one
//! run at a time, the document first, beside the frames while a page
//! that shows it is - a run already spreads across every core, so running
//! the plans side by side would only crowd them - each answer kept for as
//! long as its plan is the document or compared with it.

use std::sync::Arc;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{IntoScheduleConfigs, Res, ResMut, Resource};
use retiretui_engine::market::{Band, History, RunError, monte_carlo};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

pub(crate) use retiretui_client::compare::Success;

use crate::compare::Compared;
use crate::nav::{Page, ShownSurface};
use crate::session::{Projected, Session};
use crate::theme::Repainted;
use crate::tools::markets::MarketHistory;
use crate::tools::{Keyed, Searches};

pub fn plugin(app: &mut App) {
    app.init_resource::<Successes>();
    app.add_systems(Update, work_through.before(Repainted));
}

/// What the page knows of each plan's success.
#[derive(Resource, Default)]
pub struct Successes {
    /// Each plan run, and what its runs came to, `None` where the run was
    /// refused or panicked.
    answered: Vec<(Arc<Plan>, Option<Ran>)>,
    running: Option<Running>,
}

/// What is kept of a plan's runs: the share that succeeded, and the bands
/// most of them fell in year by year.
struct Ran {
    rate: f64,
    bands: Vec<Band>,
}

struct Running {
    run: Keyed<Arc<Plan>, Result<Ran, RunError>>,
    total: usize,
    /// How many runs the table last said were done.
    shown: usize,
}

impl Running {
    fn start(plan: &Plan, tables: TaxTables, history: History) -> Self {
        let total = usize::try_from(plan.market().trials()).unwrap_or_default();
        let plan = Arc::new(plan.clone());
        let ran = Arc::clone(&plan);
        let run = Keyed::spawn(plan, move |progress| {
            let runs = monte_carlo(&ran, &tables, &history, progress)?.runs;
            Ok(Ran {
                rate: runs.success_rate(),
                bands: runs.bands,
            })
        });
        Self {
            run,
            total,
            shown: 0,
        }
    }
}

impl Successes {
    pub(crate) fn of(&self, plan: &Plan) -> Success {
        if let Some((_, answer)) = self.answered.iter().find(|(held, _)| **held == *plan) {
            let rate = answer.as_ref().map(|ran| ran.rate);
            return rate.map_or(Success::Failed, Success::Rate);
        }
        match &self.running {
            Some(running) if *running.run.key == *plan => Success::Running {
                done: running.shown,
                total: running.total,
            },
            _ => Success::Waiting,
        }
    }

    /// Whether `plan`'s runs have answered, with runs or with none.
    pub(crate) fn has_answered_for(&self, plan: &Plan) -> bool {
        self.answered.iter().any(|(held, _)| **held == *plan)
    }

    /// The bands `plan`'s net worth fell in through random markets, year by
    /// year, where they are answered.
    pub(crate) fn bands(&self, plan: &Plan) -> Option<&[Band]> {
        let (_, answer) = self.answered.iter().find(|(held, _)| **held == *plan)?;
        answer.as_ref().map(|ran| ran.bands.as_slice())
    }

    #[cfg(test)]
    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    fn has_answered(&self) -> bool {
        let running = self.running.as_ref();
        running.is_some_and(|running| running.run.is_finished())
    }

    /// Keeps the finished run's answer; a cancelled one has none.
    fn receive(&mut self) {
        let Some(running) = self.running.take() else {
            return;
        };
        let (plan, answer, _) = running.run.join();
        let answer = match answer {
            Some(Ok(ran)) => Some(ran),
            Some(Err(RunError::Cancelled)) => return,
            Some(Err(RunError::Refused(_))) | None => None,
        };
        self.answered.push((plan, answer));
    }

    /// Forgets what is not `kept`, and runs the first of `wanted` without
    /// an answer in place of whatever runs now.
    fn queue(
        &mut self,
        (kept, wanted): (&[&Plan], &[&Plan]),
        tables: &TaxTables,
        history: &History,
    ) {
        self.answered
            .retain(|(plan, _)| kept.contains(&plan.as_ref()));
        let is_answered = |plan: &Plan| self.answered.iter().any(|(held, _)| **held == *plan);
        let next = wanted.iter().copied().find(|plan| !is_answered(plan));
        if self
            .running
            .as_ref()
            .map(|running| running.run.key.as_ref())
            == next
        {
            return;
        }
        self.running = next.map(|plan| Running::start(plan, tables.clone(), history.clone()));
    }
}

/// Keeps the table's count of the run under way, changing the resource
/// only when the count moves.
fn count_runs(successes: &mut ResMut<Successes>) {
    let running = successes.bypass_change_detection().running.as_mut();
    let is_counted = running.is_some_and(|running| {
        let done = running.run.progress().done();
        std::mem::replace(&mut running.shown, done) != done
    });
    if is_counted {
        successes.set_changed();
    }
}

/// How many of the document and the compared plans, in that order, the
/// page on show wants run: every one on Compare, the document on the
/// Overview while it runs its searches.
fn wanted(shown: Option<Page>, searches: Searches, plans: usize) -> usize {
    match shown {
        Some(Page::Compare) => plans,
        Some(Page::Overview) if searches.0 => 1,
        _ => 0,
    }
}

/// Works through the plans the page on show wants, starting over from the
/// first unanswered one whenever a plan changes; a page that wants none
/// stops the run under way, so nothing runs behind another page.
pub(crate) fn work_through(
    (projected, compared, shown): (Res<Projected>, Res<Compared>, ShownSurface),
    (session, history, searches): (Res<Session>, Res<MarketHistory>, Res<Searches>),
    mut successes: ResMut<Successes>,
) {
    let has_answered = successes.has_answered();
    if has_answered {
        successes.receive();
    }
    let is_moved = projected.is_changed()
        || compared.is_changed()
        || shown.is_changed()
        || searches.is_changed();
    if !has_answered && !is_moved {
        count_runs(&mut successes);
        return;
    }
    let document = std::iter::once(&projected.plan);
    let kept: Vec<&Plan> = document.chain(compared.plans()).collect();
    let wanted = wanted(shown.surface(), *searches, kept.len());
    successes.queue((&kept, &kept[..wanted]), &session.tables, &history.0);
}
