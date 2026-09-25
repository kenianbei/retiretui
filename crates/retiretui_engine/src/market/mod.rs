//! Markets for a plan to be projected through: drawn at random from its
//! assumptions or from history, or replayed from a historical start year.

mod history;
mod random;
mod runs;
mod statistical;

pub use history::{HistoricalYear, History, HistoryError};
pub use runs::{
    BAND_PERCENTILES, Band, MonteCarlo, Progress, Run, RunError, RunName, Runs, historical,
    monte_carlo, replay,
};

use crate::params::Inflation;
use crate::plan::{ClassReturns, Draw, Plan};
use crate::project::{MarketPath, horizon_len};

use random::Random;
use statistical::Assumptions;

/// A path of each year's `returns` from the plan's start year, and
/// `inflation`, each year's carrying prices into the next.
fn through(plan: &Plan, returns: Vec<ClassReturns>, inflation: Vec<f64>) -> MarketPath {
    let inflation = Inflation::yearly(plan.plan.start_year + 1, inflation, plan.plan.inflation);
    MarketPath::new(plan, returns, inflation)
}

/// A path through history's `years`, as they came.
fn through_years(plan: &Plan, years: &[HistoricalYear]) -> MarketPath {
    let returns = years.iter().map(HistoricalYear::returns).collect();
    let inflation = years.iter().map(|year| year.inflation).collect();
    through(plan, returns, inflation)
}

/// Monte Carlo trial `trial` of `plan`: drawn from its assumptions, or from
/// `history`'s years in blocks, as the plan's `[market]` says, from its
/// seed.
#[must_use]
pub fn draw(plan: &Plan, history: &History, trial: u32) -> MarketPath {
    let market = plan.market();
    let mut random = Random::new(market.seed(), trial);
    let length = horizon_len(plan);
    match market.draw() {
        Draw::Assumptions => {
            let assumptions = Assumptions::of(plan);
            let mut deviation = 0.0;
            let (returns, inflation) = (0..length)
                .map(|_| assumptions.year(&mut random, &mut deviation))
                .unzip();
            through(plan, returns, inflation)
        }
        Draw::History => {
            let block = usize::from(market.block_years()).clamp(1, history.len());
            let starts = history.len() - block + 1;
            let mut years = Vec::with_capacity(length + block);
            while years.len() < length {
                let at = random.below(starts);
                years.extend_from_slice(&history.years()[at..at + block]);
            }
            years.truncate(length);
            through_years(plan, &years)
        }
    }
}

/// `plan` retired into history's `start`: its years as they came from
/// then, going on from the record's first past its last where `wrap`; none
/// where that runs out.
#[must_use]
pub fn start_in(plan: &Plan, history: &History, start: i16, wrap: bool) -> Option<MarketPath> {
    history
        .sequence(start, horizon_len(plan), wrap)
        .map(|years| through_years(plan, &years))
}
