//! A plan run through many markets, and what is kept of each run: whether
//! the money lasted, what it ended with, and its net worth year by year -
//! each in today's dollars by that run's own inflation.

use std::cmp::Reverse;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use serde::Serialize;

use crate::optimize::rank_key;
use crate::params::TaxTables;
use crate::plan::{Dollars, Issue, Plan, push_issue};
use crate::project::{MarketPath, Projection, deflate, project_on};

use super::{History, draw, start_in};

/// The percentiles the bands are drawn at, lowest first, and a Monte
/// Carlo search singles a market out at, highest first.
pub const BAND_PERCENTILES: [u8; 5] = [10, 25, 50, 75, 90];

/// Which market a run went through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunName {
    /// The market the plan states.
    Planned,
    /// A Monte Carlo trial, by index.
    Trial(u32),
    /// History from a start year.
    Start(i16),
}

/// What is kept of one run, every figure in today's dollars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The market it went through.
    pub name: RunName,
    /// Net worth at the horizon.
    pub ending: Dollars,
    /// Spending the accounts could not cover, over the whole run.
    pub unfunded: Dollars,
    /// The first year spending went uncovered.
    pub first_short: Option<i16>,
    /// Net worth at each year's end, from the start year.
    pub net_worth: Vec<Dollars>,
    /// Never short, and ending with at least what the plan asks to leave.
    pub is_success: bool,
    /// What the run ranks by, as the optimizers rank plans.
    rank: (Dollars, Reverse<Dollars>),
}

impl Run {
    fn of(name: RunName, projection: &Projection, leave_at_least: Option<Dollars>) -> Self {
        let summary = projection.summary(true);
        let first_short = summary.first_unfunded_year;
        let ending = summary.final_net_worth;
        Self {
            name,
            ending,
            unfunded: summary.lifetime_unfunded,
            first_short,
            net_worth: projection
                .years
                .iter()
                .map(|row| deflate(row.net_worth, row.deflator))
                .collect(),
            is_success: first_short.is_none() && leave_at_least.is_none_or(|floor| ending >= floor),
            rank: rank_key(projection),
        }
    }
}

/// One projected year across every run.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Band {
    /// The calendar year.
    pub year: i16,
    /// Net worth at [`BAND_PERCENTILES`], lowest first.
    pub net_worth: [Dollars; BAND_PERCENTILES.len()],
    /// The share of runs not yet short by the year's end.
    pub funded: f64,
}

/// Every run through the markets, and the plan in its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Runs {
    /// The plan in the market it states.
    pub planned: Run,
    /// Every run, in the order the markets were made.
    pub runs: Vec<Run>,
    /// How many runs succeeded.
    pub successes: usize,
    /// Each projected year across the runs.
    pub bands: Vec<Band>,
}

impl Runs {
    fn of(planned: Run, runs: Vec<Run>, start_year: i16) -> Self {
        let years = planned.net_worth.len();
        let bands = (0..years)
            .map(|at| {
                let year = start_year + i16::try_from(at).unwrap_or(i16::MAX);
                let mut worths: Vec<Dollars> = runs.iter().map(|run| run.net_worth[at]).collect();
                worths.sort_unstable();
                let funded = runs
                    .iter()
                    .filter(|run| run.first_short.is_none_or(|short| short > year))
                    .count();
                Band {
                    year,
                    net_worth: BAND_PERCENTILES
                        .map(|percentile| at_percentile(&worths, percentile).unwrap_or_default()),
                    funded: funded as f64 / runs.len().max(1) as f64,
                }
            })
            .collect();
        Self {
            successes: runs.iter().filter(|run| run.is_success).count(),
            planned,
            runs,
            bands,
        }
    }

    /// The share of runs that succeeded.
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        self.successes as f64 / self.runs.len().max(1) as f64
    }

    /// Run indices, worst first; a tie goes to the market made first.
    #[must_use]
    pub fn worst_first(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.runs.len()).collect();
        order.sort_by_key(|&at| (Reverse(self.runs[at].rank), at));
        order
    }
}

/// The nearest-rank value at `percentile` of `sorted`, lowest first; none
/// of nothing.
fn at_percentile<T: Copy>(sorted: &[T], percentile: u8) -> Option<T> {
    let last = sorted.len().checked_sub(1)?;
    let at = (f64::from(percentile) / 100.0 * last as f64).round() as usize;
    Some(sorted[at.min(last)])
}

/// How far a search has got, and a way to stop it. Shared with the thread
/// that runs it.
#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicUsize,
    is_cancelled: AtomicBool,
}

impl Progress {
    /// How many runs have finished.
    #[must_use]
    pub fn done(&self) -> usize {
        self.done.load(Ordering::Relaxed)
    }

    /// Asks the search to stop after the runs under way.
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::Relaxed);
    }

    /// Whether the search has been asked to stop, which work of its own
    /// between steps may check.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Relaxed)
    }
}

/// Why a search answered nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum RunError {
    /// It was cancelled.
    Cancelled,
    /// The plan's settings cannot be run, and why.
    Refused(Vec<Issue>),
}

/// A Monte Carlo search's runs, and the markets it singles out.
#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarlo {
    /// Every trial.
    pub runs: Runs,
    /// The run at each of [`BAND_PERCENTILES`], highest first, then the
    /// worst, by index.
    pub singled_out: Vec<usize>,
}

/// The runs at `range`, stopping early once `progress` is cancelled.
fn run_range(
    range: std::ops::Range<usize>,
    progress: &Progress,
    one: &(impl Fn(usize) -> Option<Run> + Sync),
) -> Vec<Option<Run>> {
    range
        .take_while(|_| !progress.is_cancelled())
        .map(|at| {
            let run = one(at);
            progress.done.fetch_add(1, Ordering::Relaxed);
            run
        })
        .collect()
}

/// Runs `count` markets across the machine's threads; each index's run is
/// its own whatever thread takes it, and one that has no market is left
/// out.
fn run_all(
    count: usize,
    progress: &Progress,
    one: impl Fn(usize) -> Option<Run> + Sync,
) -> Result<Vec<Run>, RunError> {
    let threads = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(count.max(1));
    let chunk = count.div_ceil(threads).max(1);
    let one = &one;
    let finished: Vec<Vec<Option<Run>>> = thread::scope(|scope| {
        let handles: Vec<_> = (0..count)
            .step_by(chunk)
            .map(|first| {
                let range = first..(first + chunk).min(count);
                scope.spawn(move || run_range(range, progress, one))
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a run does not panic"))
            .collect()
    });
    if progress.is_cancelled() {
        return Err(RunError::Cancelled);
    }
    Ok(finished.into_iter().flatten().flatten().collect())
}

fn planned(plan: &Plan, tables: &TaxTables) -> Run {
    let projection = project_on(plan, tables, &MarketPath::expected(plan));
    Run::of(
        RunName::Planned,
        &projection,
        plan.market().leave_at_least(),
    )
}

/// Runs `plan` through its Monte Carlo markets.
///
/// # Errors
///
/// [`RunError::Cancelled`] when `progress` was cancelled first.
pub fn monte_carlo(
    plan: &Plan,
    tables: &TaxTables,
    history: &History,
    progress: &Progress,
) -> Result<MonteCarlo, RunError> {
    let market = plan.market();
    let floor = market.leave_at_least();
    let count = usize::try_from(market.trials()).unwrap_or_default();
    let runs = run_all(count, progress, |at| {
        let trial = u32::try_from(at).unwrap_or(u32::MAX);
        let projection = project_on(plan, tables, &draw(plan, history, trial));
        Some(Run::of(RunName::Trial(trial), &projection, floor))
    })?;
    let runs = Runs::of(planned(plan, tables), runs, plan.plan.start_year);
    let worst_first = runs.worst_first();
    let mut singled_out: Vec<usize> = BAND_PERCENTILES
        .iter()
        .rev()
        .filter_map(|&percentile| at_percentile(&worst_first, percentile))
        .collect();
    singled_out.extend(worst_first.first());
    Ok(MonteCarlo { runs, singled_out })
}

/// Runs `plan` through history from each start year its `[market]` names.
///
/// # Errors
///
/// [`RunError::Refused`] when the start years fall outside `history` or
/// none has a full horizon to run; [`RunError::Cancelled`] when `progress`
/// was cancelled first.
pub fn historical(
    plan: &Plan,
    tables: &TaxTables,
    history: &History,
    progress: &Progress,
) -> Result<Runs, RunError> {
    let market = plan.market();
    let (from, to) = (market.from(), market.to());
    let covered = format!(
        "the record covers {} to {}",
        history.first_year(),
        history.last_year()
    );
    let mut issues = Vec::new();
    if from < history.first_year() {
        push_issue(&mut issues, "market.historical.from", &covered);
    }
    if to > history.last_year() {
        push_issue(&mut issues, "market.historical.to", &covered);
    }
    if !issues.is_empty() {
        return Err(RunError::Refused(issues));
    }
    let floor = market.leave_at_least();
    let count = usize::try_from(to - from + 1).unwrap_or_default();
    let runs = run_all(count, progress, |at| {
        let start = from + i16::try_from(at).unwrap_or(i16::MAX);
        let path = start_in(plan, history, start, market.wrap())?;
        Some(Run::of(
            RunName::Start(start),
            &project_on(plan, tables, &path),
            floor,
        ))
    })?;
    if runs.is_empty() {
        push_issue(
            &mut issues,
            "market.historical.wrap",
            "no start year has a whole horizon of history; wrap, or start earlier",
        );
        return Err(RunError::Refused(issues));
    }
    Ok(Runs::of(planned(plan, tables), runs, plan.plan.start_year))
}

/// `plan` projected whole through the market `name` ran in; none for a
/// start year history cannot run.
#[must_use]
pub fn replay(
    plan: &Plan,
    tables: &TaxTables,
    history: &History,
    name: RunName,
) -> Option<Projection> {
    let path = match name {
        RunName::Planned => MarketPath::expected(plan),
        RunName::Trial(trial) => draw(plan, history, trial),
        RunName::Start(start) => start_in(plan, history, start, plan.market().wrap())?,
    };
    Some(project_on(plan, tables, &path))
}
