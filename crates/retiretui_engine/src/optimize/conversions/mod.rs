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

use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};

use super::rank_key;
use crate::params::TaxTables;
use crate::plan::{Dollars, Issue, Plan, TreatmentClass};
use crate::project::{Projection, plan_inflation, project};
use crate::search::{Progress, RunError, run_all};

use check::check_options;
use fill::search_ladder;

/// Constraints for a fill-bracket conversion ladder; the target bracket is
/// passed beside them, so a sweep reuses one set of constraints.
#[derive(Debug, Clone)]
pub struct OptimizeOptions {
    /// Deferred source account ids, drained in the given order; empty means
    /// every deferred account of the destination's owner, in plan order.
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
    /// Realized long-term gains may not be pushed past this rate; a year
    /// that realizes none is not held.
    pub gains_rate: Option<GainsRate>,
}

/// The long-term gains rate a ladder may not push realized gains past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum GainsRate {
    /// Gains stay untaxed.
    #[serde(rename = "0")]
    Zero,
    /// Gains stay out of the top rate.
    #[serde(rename = "15")]
    Fifteen,
}

impl GainsRate {
    /// Every rate, lowest first.
    pub const ALL: &'static [Self] = &[Self::Zero, Self::Fifteen];

    /// The rate as an option spells it: its percent.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::Fifteen => "15",
        }
    }
}

impl std::str::FromStr for GainsRate {
    type Err = serde::de::value::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::deserialize(text.into_deserializer())
    }
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
    /// One entry per bracket searched, best first by
    /// [`rank_key`](super::rank_key); a tie keeps the lower rate first.
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
    let options = &with_default_sources(plan, options);
    let issues = check_options(plan, tables, options, Some(bracket_rate));
    if !issues.is_empty() {
        return Err(issues);
    }
    let baseline = project(plan, tables);
    let (bare, from) = without_ladder(plan, tables, &baseline);
    let (steps, optimized) = search_ladder(
        &bare,
        tables,
        (options, bracket_rate),
        &from,
        &Progress::default(),
    );
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
/// which has no ceiling), across the machine's threads, against one shared
/// baseline, and ranks the ladders best first; each ladder replaces the
/// plan's own, as [`optimize_conversions`].
///
/// # Errors
///
/// [`RunError::Refused`] as [`optimize_conversions`], for the shared
/// constraint checks; [`RunError::Cancelled`] when `progress` is cancelled
/// before every ladder is settled.
pub fn sweep_brackets(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
    progress: &Progress,
) -> Result<BracketSweep, RunError> {
    let options = &with_default_sources(plan, options);
    let issues = check_options(plan, tables, options, None);
    if !issues.is_empty() {
        return Err(RunError::Refused(issues));
    }
    if progress.is_cancelled() {
        return Err(RunError::Cancelled);
    }
    let baseline = project(plan, tables);
    let (bare, from) = without_ladder(plan, tables, &baseline);
    let rates = fillable_rates(plan, tables);
    let mut brackets = run_all(rates.len(), progress, |at| {
        let rate = rates[at];
        let (steps, optimized) = search_ladder(&bare, tables, (options, rate), &from, progress);
        Some(SweptBracket {
            rate,
            steps,
            optimized,
        })
    })?;
    brackets.sort_by_cached_key(|bracket| rank_key(&bracket.optimized));
    Ok(BracketSweep { baseline, brackets })
}

/// Every bracket rate a ladder can fill: all but the top, ascending.
fn fillable_rates(plan: &Plan, tables: &TaxTables) -> Vec<f64> {
    let params = tables.params_for(plan.plan.start_year, &plan_inflation(plan));
    let mut rates: Vec<f64> = params
        .brackets
        .for_status(plan.household.filing)
        .iter()
        .map(|bracket| bracket.rate)
        .collect();
    rates.pop();
    rates
}

/// `options` with no sources stated read as every deferred account of the
/// destination's owner, in plan order.
fn with_default_sources(plan: &Plan, options: &OptimizeOptions) -> OptimizeOptions {
    let mut options = options.clone();
    if options.sources.is_empty() {
        let owner = plan
            .account(&options.destination)
            .map(|account| &account.owner);
        options.sources = plan
            .accounts
            .iter()
            .filter(|account| {
                account.treatment() == TreatmentClass::Deferred && Some(&account.owner) == owner
            })
            .map(|account| account.id.clone())
            .collect();
    }
    options
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
