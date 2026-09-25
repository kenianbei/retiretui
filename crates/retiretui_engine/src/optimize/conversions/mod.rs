//! The Roth conversion optimizer: fill-bracket ladders searched against
//! full projections.
//!
//! A ladder converts from deferred sources into one Roth so each window
//! year's ordinary taxable income reaches the chosen bracket's top (minus a
//! headroom cushion). Years are settled front to back - later conversions
//! cannot change an earlier year - and each year's amount is found by
//! re-projecting candidate plans, so every interaction the engine models
//! (Social Security taxability, withdrawals to cover the tax bill, balance
//! clamps, locks) is priced in rather than approximated.

mod check;
mod fill;
mod ladder;
mod targets;

pub use ladder::{LADDER_ID_PREFIX, apply_ladder, is_ladder, ladder_overlay};

use serde::Serialize;

use crate::params::TaxTables;
use crate::plan::{Dollars, Issue, Plan};
use crate::project::{Projection, plan_inflation, project};

use check::check_options;
use fill::search_ladder;

/// Constraints for a fill-bracket conversion ladder; the target bracket is
/// passed beside them, so a sweep reuses one set of constraints.
#[derive(Debug, Clone)]
pub struct OptimizeOptions {
    /// Deferred source account ids, drained in the given order.
    pub sources: Vec<String>,
    /// Roth destination account id; every source shares its owner.
    pub destination: String,
    /// First conversion year; `None` means plan start.
    pub start_year: Option<i16>,
    /// Last conversion year; `None` means the year before the destination
    /// owner's RMDs begin.
    pub end_year: Option<i16>,
    /// Cap on any single year's total conversion.
    pub annual_max: Option<Dollars>,
    /// Cap on the ladder's cumulative conversions.
    pub total_max: Option<Dollars>,
    /// Dollars deliberately left unfilled below the bracket top.
    pub headroom: Dollars,
    /// Highest IRMAA tier the ladder may buy: MAGI is capped at tier
    /// `N+1`'s threshold (`0` stays under every surcharge). Requires
    /// `[medicare]`, and applies only to years whose two-year-later
    /// premiums still fall on a covered person inside the horizon.
    pub irmaa_tier: Option<u8>,
    /// Explicit MAGI ceiling in today's dollars, scaled by plan inflation.
    pub max_magi: Option<Dollars>,
}

/// One emitted conversion: `amount` nominal dollars in `year` from `source`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LadderStep {
    /// The conversion year.
    pub year: i16,
    /// The source account id.
    pub source: String,
    /// Nominal dollars converted.
    pub amount: Dollars,
}

/// A searched ladder with the projection it was judged against.
#[derive(Debug, Clone)]
pub struct OptimizedLadder {
    /// The plan projected as given, any ladder it holds included.
    pub baseline: Projection,
    /// The bracket's ladder and the plan projected with it.
    pub ladder: SweptBracket,
}

/// Every fillable bracket's ladder beside the one shared baseline.
#[derive(Debug, Clone)]
pub struct BracketSweep {
    /// The plan projected as given, any ladder it holds included.
    pub baseline: Projection,
    /// One entry per fillable bracket, ascending by rate.
    pub brackets: Vec<SweptBracket>,
}

/// One bracket's ladder and the plan projected with it.
#[derive(Debug, Clone)]
pub struct SweptBracket {
    /// The bracket's rate (e.g. `0.22`).
    pub rate: f64,
    /// The per-year conversions, in year order.
    pub steps: Vec<LadderStep>,
    /// The plan projected with this bracket's ladder applied.
    pub optimized: Projection,
}

impl SweptBracket {
    /// The ladder's total on the chosen basis: the plain step sum when
    /// nominal, each step deflated by its year's deflator otherwise.
    #[must_use]
    pub fn converted(&self, deflated: bool) -> Dollars {
        if !deflated {
            return self.steps.iter().map(|step| step.amount).sum();
        }
        self.steps
            .iter()
            .map(|step| self.optimized.deflate_in(step.year, step.amount))
            .sum()
    }
}

/// Stated amounts far above any plausible balance, so a ceiling probe
/// converts everything a source can give.
const UNBOUNDED: Dollars = 1_000_000_000_000;

/// Bracket rates match within this tolerance.
const RATE_EPSILON: f64 = 1e-6;

/// Searches the fill-bracket ladder for `bracket_rate` under `options`,
/// in place of any ladder the plan already holds: the conversions with
/// [`LADDER_ID_PREFIX`] ids are left out of the search, and `baseline` is
/// the plan as given.
///
/// # Errors
///
/// Returns validation issues when the options do not fit the plan: unknown
/// or non-deferred sources, a non-Roth destination, mismatched owners, an
/// unknown or top bracket rate, or an empty window.
pub fn optimize_conversions(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
    bracket_rate: f64,
) -> Result<OptimizedLadder, Vec<Issue>> {
    let issues = check_options(plan, tables, options, Some(bracket_rate));
    if !issues.is_empty() {
        return Err(issues);
    }
    let baseline = project(plan, tables);
    let (bare, from) = without_ladder(plan, tables, &baseline);
    let (steps, optimized) = search_ladder(&bare, tables, options, bracket_rate, &from);
    Ok(OptimizedLadder {
        baseline,
        ladder: SweptBracket {
            rate: bracket_rate,
            steps,
            optimized,
        },
    })
}

/// Runs the optimizer once per fillable bracket (every rate but the top,
/// which has no ceiling), in ascending rate order, against one shared
/// baseline; each ladder replaces the plan's own, as
/// [`optimize_conversions`].
///
/// # Errors
///
/// As [`optimize_conversions`], for the shared constraint checks.
pub fn sweep_brackets(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
) -> Result<BracketSweep, Vec<Issue>> {
    let issues = check_options(plan, tables, options, None);
    if !issues.is_empty() {
        return Err(issues);
    }
    let baseline = project(plan, tables);
    let (bare, from) = without_ladder(plan, tables, &baseline);
    let params = tables.params_for(plan.plan.start_year, &plan_inflation(plan));
    let rates: Vec<f64> = params
        .brackets
        .for_status(plan.household.filing)
        .iter()
        .map(|bracket| bracket.rate)
        .collect();
    let brackets = rates
        .iter()
        .take(rates.len().saturating_sub(1))
        .map(|&rate| {
            let (steps, optimized) = search_ladder(&bare, tables, options, rate, &from);
            SweptBracket {
                rate,
                steps,
                optimized,
            }
        })
        .collect();
    Ok(BracketSweep { baseline, brackets })
}

/// `plan` without the conversions a ladder put there, and its projection,
/// which is `baseline` where it held none.
fn without_ladder(plan: &Plan, tables: &TaxTables, baseline: &Projection) -> (Plan, Projection) {
    let mut bare = plan.clone();
    bare.conversions.retain(|conversion| !is_ladder(conversion));
    let from = if bare.conversions.len() == plan.conversions.len() {
        baseline.clone()
    } else {
        project(&bare, tables)
    };
    (bare, from)
}
