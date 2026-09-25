//! Each plan's Monte Carlo success, under its own `[market]` settings: one
//! run at a time, the document first, on a thread of its own while a page
//! that shows it is - a run already spreads across every core, so running
//! the plans side by side would only crowd them - each answer kept for as
//! long as its plan is the document or compared with it.

use std::sync::Arc;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{IntoScheduleConfigs, Res, ResMut, Resource};
use retiretui_engine::market::{History, RunError, monte_carlo};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use crate::commands::tui::compare::Compared;
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::present::{self, SAME};
use crate::commands::tui::session::{Projected, Session};
use crate::commands::tui::theme::Repainted;
use crate::commands::tui::tools::markets::MarketHistory;
use crate::commands::tui::tools::{Searches, Worker, running_text};

pub fn plugin(app: &mut App) {
    app.init_resource::<Successes>();
    app.add_systems(Update, work_through.before(Repainted));
}

const WAITING: &str = "waiting";
const FAILED: &str = "—";
const PERCENT: f64 = 100.0;
/// A difference under this many points rounds to none at the one place
/// it is shown to, and reads as the same.
const SAME_POINTS: f64 = 0.05;

/// What the page knows of each plan's success.
#[derive(Resource, Default)]
pub struct Successes {
    /// Each plan run, and the share of its runs that succeeded, `None`
    /// where the run was refused or panicked.
    answered: Vec<(Arc<Plan>, Option<f64>)>,
    running: Option<Running>,
}

struct Running {
    plan: Arc<Plan>,
    worker: Worker<Result<f64, RunError>>,
    total: usize,
    /// How many runs the table last said were done.
    shown: usize,
}

impl Running {
    fn start(plan: &Plan, tables: TaxTables, history: History) -> Self {
        let plan = Arc::new(plan.clone());
        let ran = Arc::clone(&plan);
        let worker = Worker::spawn(move |progress| {
            let found = monte_carlo(&ran, &tables, &history, progress)?;
            Ok(found.runs.success_rate())
        });
        let total = usize::try_from(plan.market().trials()).unwrap_or_default();
        Self {
            plan,
            worker,
            total,
            shown: 0,
        }
    }
}

/// A plan's success as the table has it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Success {
    Waiting,
    Running { done: usize, total: usize },
    Rate(f64),
    Failed,
}

impl Success {
    pub(crate) fn text(self) -> String {
        match self {
            Self::Waiting => WAITING.to_owned(),
            Self::Running { done, total } => running_text(done, total),
            Self::Rate(rate) => present::rate(rate),
            Self::Failed => FAILED.to_owned(),
        }
    }

    /// Points more or fewer than `base`'s, where both have answered.
    pub(crate) fn against(self, base: Self) -> String {
        let (Self::Rate(own), Self::Rate(base)) = (self, base) else {
            return self.text();
        };
        let points = (own - base) * PERCENT;
        if points.abs() < SAME_POINTS {
            SAME.to_owned()
        } else {
            format!("{points:+.1} pts")
        }
    }
}

impl Successes {
    pub(crate) fn of(&self, plan: &Plan) -> Success {
        if let Some((_, answer)) = self.answered.iter().find(|(held, _)| **held == *plan) {
            return answer.map_or(Success::Failed, Success::Rate);
        }
        match &self.running {
            Some(running) if *running.plan == *plan => Success::Running {
                done: running.shown,
                total: running.total,
            },
            _ => Success::Waiting,
        }
    }

    #[cfg(test)]
    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    fn has_answered(&self) -> bool {
        let running = self.running.as_ref();
        running.is_some_and(|running| running.worker.is_finished())
    }

    /// Keeps the finished run's answer; a cancelled one has none.
    fn receive(&mut self) {
        let Some(running) = self.running.take() else {
            return;
        };
        let answer = match running.worker.join() {
            Some(Ok(rate)) => Some(rate),
            Some(Err(RunError::Cancelled)) => return,
            Some(Err(RunError::Refused(_))) | None => None,
        };
        self.answered.push((running.plan, answer));
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
        if self.running.as_ref().map(|running| running.plan.as_ref()) == next {
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
        let done = running.worker.progress.done();
        std::mem::replace(&mut running.shown, done) != done
    });
    if is_counted {
        successes.set_changed();
    }
}

/// How many of the document and the compared plans, in that order, the
/// page on show wants run: every one on Compare, the document on the
/// Overview while it runs its searches.
fn wanted(active: Page, searches: Searches, plans: usize) -> usize {
    match active {
        Page::Compare => plans,
        Page::Overview if searches.0 => 1,
        _ => 0,
    }
}

/// Works through the plans the page on show wants, starting over from the
/// first unanswered one whenever a plan changes; a page that wants none
/// stops the run under way, so nothing runs behind another page.
pub(crate) fn work_through(
    (projected, compared, active): (Res<Projected>, Res<Compared>, Res<ActivePage>),
    (session, history, searches): (Res<Session>, Res<MarketHistory>, Res<Searches>),
    mut successes: ResMut<Successes>,
) {
    let has_answered = successes.has_answered();
    if has_answered {
        successes.receive();
    }
    let is_moved = projected.is_changed()
        || compared.is_changed()
        || active.is_changed()
        || searches.is_changed();
    if !has_answered && !is_moved {
        count_runs(&mut successes);
        return;
    }
    let document = std::iter::once(&projected.plan);
    let kept: Vec<&Plan> = document.chain(compared.plans()).collect();
    let wanted = wanted(active.0, *searches, kept.len());
    successes.queue((&kept, &kept[..wanted]), &session.tables, &history.0);
}
