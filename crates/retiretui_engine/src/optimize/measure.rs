//! What a solver holds a candidate plan to: lasting in the plan's own
//! market, or in a share of its Monte Carlo markets.

use crate::market::{History, Run, RunName, monte_carlo};
use crate::params::TaxTables;
use crate::plan::Plan;
use crate::project::{Projection, project};
use crate::search::{Progress, RunError};

/// What a solver holds a candidate plan to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Measure {
    /// Succeeds in the plan's own market.
    Planned,
    /// Succeeds in at least this share of its Monte Carlo markets.
    Success(f64),
}

impl Measure {
    /// Refuses a target that is no share.
    ///
    /// # Errors
    ///
    /// [`RunError::Refused`] for a target share outside 0 to 1.
    pub(crate) fn check(self) -> Result<(), RunError> {
        match self {
            Self::Success(target) if !(target > 0.0 && target <= 1.0) => Err(RunError::refused(
                "success",
                "must be a share above 0%, and no more than 100%",
            )),
            _ => Ok(()),
        }
    }
}

/// A candidate plan judged against a [`Measure`].
#[derive(Debug, Clone, PartialEq)]
pub struct Judged {
    /// Whether it met the measure.
    pub is_met: bool,
    /// The candidate projected in its own market.
    pub projection: Projection,
    /// The share of its Monte Carlo markets it succeeded in, where the
    /// measure ran them.
    pub success_rate: Option<f64>,
}

/// Judges `plan` against `measure`, [checked](Measure::check) by whoever
/// asks, by the success the market tools count.
///
/// # Errors
///
/// [`RunError::Cancelled`] when `progress` was cancelled first.
pub(crate) fn judge(
    plan: &Plan,
    tables: &TaxTables,
    history: &History,
    measure: Measure,
    progress: &Progress,
) -> Result<Judged, RunError> {
    if progress.is_cancelled() {
        return Err(RunError::Cancelled);
    }
    let projection = project(plan, tables);
    let (is_met, success_rate) = match measure {
        Measure::Planned => {
            let floor = plan.market().leave_at_least();
            let run = Run::of(RunName::Planned, &projection, floor);
            (run.is_success, None)
        }
        Measure::Success(target) => {
            let rate = monte_carlo(plan, tables, history, progress)?
                .runs
                .success_rate();
            (rate >= target, Some(rate))
        }
    };
    Ok(Judged {
        is_met,
        projection,
        success_rate,
    })
}
