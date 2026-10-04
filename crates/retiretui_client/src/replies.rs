//! The shapes a search or a year's actions are replied in, one for every
//! interface that replies in data: the command line's JSON and the agent
//! server's tools.

use std::collections::BTreeMap;

use retiretui_engine::market::{BAND_PERCENTILES, Band, MonteCarlo, Run, RunName, Runs};
use retiretui_engine::optimize::{
    BracketSweep, Claim, ClaimSearch, LadderStep, OptimizedLadder, OrderSearch, ScaledExpense,
};
use retiretui_engine::plan::{Dollars, Plan, TreatmentClass};
use retiretui_engine::project::{Action, Projection, Summary, YearRow};
use schemars::JsonSchema;
use serde::Serialize;

use crate::searches::spending::{self, Listed};
use crate::table::percentile_label;

/// One year's to-dos, as `actions` and `plan_actions` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ActionsReply {
    /// The reported year.
    pub year: i16,
    /// Age each person reaches during the year, by person id.
    pub ages: BTreeMap<String, u8>,
    /// Executed instructions in execution order, each tagged by `kind`
    /// (`transfer`, `rmd`, `contribution`, `conversion`, `withdrawal`)
    /// with account ids and nominal amounts - they are instructions, so
    /// no deflated variant exists.
    pub actions: Vec<Action>,
    /// Human-readable warnings: unfunded spending, medicare and cliff
    /// costs, and the IRMAA surcharge this year's MAGI buys two years out.
    pub warnings: Vec<String>,
}

impl ActionsReply {
    /// The reply for what was found.
    #[must_use]
    pub fn new(row: &YearRow, warnings: Vec<String>) -> Self {
        Self {
            year: row.year,
            ages: row.ages.clone(),
            actions: row.actions.clone(),
            warnings,
        }
    }
}

/// A claim search, as `optimize claims` and `optimize_claims` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimsReply {
    /// Headline figures with the plan's own claims.
    pub baseline: Summary,
    /// The income ids searched, in the order each candidate's claims hold
    /// them.
    pub incomes: Vec<String>,
    /// Every candidate, best first: least unfunded spending, then the
    /// highest final net worth in today's dollars, then earlier claims.
    pub candidates: Vec<ClaimEntry>,
}

/// One set of claims the search tried.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClaimEntry {
    /// One claim per searched income.
    pub claims: Vec<Claim>,
    /// Headline figures under those claims.
    pub summary: Summary,
}

impl ClaimsReply {
    /// The reply for what was found.
    #[must_use]
    pub fn new(search: &ClaimSearch, deflated: bool) -> Self {
        Self {
            baseline: search.baseline.summary(deflated),
            incomes: search.incomes.clone(),
            candidates: search
                .candidates
                .iter()
                .map(|candidate| ClaimEntry {
                    claims: candidate.claims.clone(),
                    summary: candidate.projection.summary(deflated),
                })
                .collect(),
        }
    }
}

/// An order search, as `optimize order` and `optimize_order` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrderReply {
    /// Headline figures under the plan's own order.
    pub baseline: Summary,
    /// One candidate for each distinct outcome, best first: least unfunded
    /// spending, then the highest final net worth in today's dollars, then
    /// the plan's own order, then the order they were tried in.
    pub candidates: Vec<OrderEntry>,
}

/// One order the search tried.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrderEntry {
    /// The classes, first drained first; of the orders that project alike,
    /// the plan's own, or else the first tried.
    pub order: Vec<TreatmentClass>,
    /// Headline figures under that order.
    pub summary: Summary,
}

impl OrderReply {
    /// The reply for what was found.
    #[must_use]
    pub fn new(search: &OrderSearch, deflated: bool) -> Self {
        Self {
            baseline: search.baseline.summary(deflated),
            candidates: search
                .candidates
                .iter()
                .map(|candidate| OrderEntry {
                    order: candidate.order.clone(),
                    summary: candidate.projection.summary(deflated),
                })
                .collect(),
        }
    }
}

/// A spending ceiling search, as `optimize spending` and
/// `optimize_spending` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SpendingReply {
    /// Headline figures of the plan as stated.
    pub baseline: Summary,
    /// What the plan spends a year on what it could cut: every recurring
    /// expense not marked `essential`, in today's dollars.
    pub flexible: Dollars,
    /// The share of its Monte Carlo markets the plan lasts in as stated.
    pub success: f64,
    /// The most flexible spending that lasts in the plan's own market.
    pub planned: CeilingEntry,
    /// The most that lasts in the target share of its Monte Carlo markets.
    pub at_target: CeilingEntry,
}

/// One spending ceiling.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CeilingEntry {
    /// What every flexible amount is multiplied by; 1.0 is the plan.
    pub factor: f64,
    /// Whether the search stopped at its highest factor with the plan still
    /// lasting, so the ceiling is at least this.
    pub is_capped: bool,
    /// Flexible spending a year at the ceiling, in today's dollars.
    pub flexible: Dollars,
    /// The share of its Monte Carlo markets the plan lasts in at the
    /// ceiling.
    pub success: f64,
    /// Headline figures at the ceiling, in the plan's own market.
    pub summary: Summary,
    /// Each flexible expense's annual amount at the ceiling, in today's
    /// dollars.
    pub expenses: Vec<ScaledExpense>,
}

impl SpendingReply {
    /// The reply for what was found over `plan`.
    #[must_use]
    pub fn new(found: &spending::Found, plan: &Plan, deflated: bool) -> Self {
        let entry = |listed: &Listed| CeilingEntry {
            factor: listed.ceiling.factor,
            is_capped: listed.ceiling.is_capped,
            flexible: listed.flexible(),
            success: listed.success,
            summary: listed.ceiling.judged.projection.summary(deflated),
            expenses: listed.ceiling.expenses.clone(),
        };
        let [planned, at_target] = found.listed();
        Self {
            baseline: found.planned.baseline.projection.summary(deflated),
            flexible: plan.flexible_spending(),
            success: found.plan_success(),
            planned: entry(&planned),
            at_target: entry(&at_target),
        }
    }
}

/// A bracket sweep, as `optimize conversions` and
/// `sweep_conversion_brackets` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SweepReply {
    /// The plan without any ladder.
    pub baseline: Summary,
    /// One entry per fillable bracket, best first: the least left unfunded,
    /// then the most left at the end.
    pub brackets: Vec<SweepEntry>,
}

/// One bracket of a sweep.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SweepEntry {
    /// The bracket's rate (e.g. 0.22).
    pub bracket_rate: f64,
    /// The ladder's total, on the reply's dollar basis.
    pub total_converted: Dollars,
    /// Headline figures with the ladder applied.
    pub optimized: Summary,
}

impl SweepReply {
    /// The reply for what was found.
    #[must_use]
    pub fn new(sweep: &BracketSweep, deflated: bool) -> Self {
        Self {
            baseline: sweep.baseline.summary(deflated),
            brackets: sweep
                .brackets
                .iter()
                .map(|bracket| SweepEntry {
                    bracket_rate: bracket.rate,
                    total_converted: bracket.converted(deflated),
                    optimized: bracket.optimized.summary(deflated),
                })
                .collect(),
        }
    }
}

/// A searched ladder, as `optimize conversions --bracket` and
/// `optimize_conversions` reply.
#[derive(Serialize, JsonSchema)]
pub struct LadderReply {
    /// The per-year conversions, in year order.
    pub steps: Vec<LadderStep>,
    /// The ladder's total, on the reply's dollar basis.
    pub total_converted: Dollars,
    /// Headline figures without the ladder.
    pub baseline: Summary,
    /// Headline figures with the ladder applied.
    pub optimized: Summary,
}

impl LadderReply {
    /// The reply for what was found.
    #[must_use]
    pub fn new(ladder: &OptimizedLadder, deflated: bool) -> Self {
        Self {
            steps: ladder.ladder.steps.clone(),
            total_converted: ladder.ladder.converted(deflated),
            baseline: ladder.baseline.summary(deflated),
            optimized: ladder.ladder.optimized.summary(deflated),
        }
    }
}

/// One run as the replies show it.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunEntry {
    /// What the run is: "as planned", a percentile, "worst", or a start
    /// year.
    pub market: String,
    /// The Monte Carlo trial it was.
    pub trial: Option<u32>,
    /// The historical year it started in.
    pub start: Option<i16>,
    /// Net worth at the horizon, today's dollars.
    pub ending: Dollars,
    /// Spending left uncovered over the run, today's dollars.
    pub unfunded: Dollars,
    /// The first year spending went uncovered.
    pub first_short: Option<i16>,
    /// Never short, and leaving at least what the plan asks.
    pub success: bool,
}

/// A Monte Carlo search, as `monte-carlo` and `plan_monte_carlo` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MonteCarloReply {
    /// "assumptions" or "history".
    pub draw: String,
    /// The seed the markets were drawn from.
    pub seed: u32,
    /// How many markets ran.
    pub runs: usize,
    /// How many succeeded.
    pub successes: usize,
    /// The share that succeeded.
    pub success_rate: f64,
    /// The plan as planned, then the markets singled out, best first.
    pub markets: Vec<RunEntry>,
    /// Net worth year by year across the markets.
    pub bands: Vec<Band>,
}

/// A Historical search, as `historical` and `plan_historical` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HistoricalReply {
    /// The first start year tried.
    pub from: i16,
    /// The last start year tried.
    pub to: i16,
    /// Whether histories went on from the record's start past its end.
    pub wrap: bool,
    /// How many start years ran.
    pub runs: usize,
    /// How many succeeded.
    pub successes: usize,
    /// The share that succeeded.
    pub success_rate: f64,
    /// The plan as planned, then every start year, worst first.
    pub start_years: Vec<RunEntry>,
}

fn entry(market: String, run: &Run) -> RunEntry {
    let (trial, start) = match run.name {
        RunName::Planned => (None, None),
        RunName::Trial(trial) => (Some(trial), None),
        RunName::Start(start) => (None, Some(start)),
    };
    RunEntry {
        market,
        trial,
        start,
        ending: run.ending,
        unfunded: run.unfunded,
        first_short: run.first_short,
        success: run.is_success,
    }
}

/// What the plan's own market is called among the runs.
const PLANNED: &str = "as planned";

impl MonteCarloReply {
    /// The reply for what was found.
    #[must_use]
    pub fn new(plan: &Plan, found: &MonteCarlo) -> Self {
        let runs = &found.runs;
        let labels = BAND_PERCENTILES
            .iter()
            .rev()
            .map(|&percentile| percentile_label(percentile))
            .chain(std::iter::once("worst".to_owned()));
        let mut markets = vec![entry(PLANNED.to_owned(), &runs.planned)];
        markets.extend(
            labels
                .zip(&found.singled_out)
                .map(|(label, &at)| entry(label, &runs.runs[at])),
        );
        Self {
            draw: plan.market().draw().as_str().to_owned(),
            seed: plan.market().seed(),
            runs: runs.runs.len(),
            successes: runs.successes,
            success_rate: runs.success_rate(),
            markets,
            bands: runs.bands.clone(),
        }
    }
}

impl HistoricalReply {
    /// The reply for what was found.
    #[must_use]
    pub fn new(plan: &Plan, runs: &Runs) -> Self {
        let mut start_years = vec![entry(PLANNED.to_owned(), &runs.planned)];
        start_years.extend(runs.worst_first().into_iter().map(|at| {
            let run = &runs.runs[at];
            let label = match run.name {
                RunName::Start(start) => start.to_string(),
                RunName::Planned | RunName::Trial(_) => String::new(),
            };
            entry(label, run)
        }));
        Self {
            from: plan.market().from(),
            to: plan.market().to(),
            wrap: plan.market().wrap(),
            runs: runs.runs.len(),
            successes: runs.successes,
            success_rate: runs.success_rate(),
            start_years,
        }
    }
}

/// The `year` row; an out-of-range year errors with the valid range.
///
/// # Errors
///
/// Where the plan does not reach `year`.
pub fn year_row(projection: &Projection, year: i16) -> Result<&YearRow, String> {
    projection.row(year).ok_or_else(|| {
        let first = projection.years.first().map_or(year, |row| row.year);
        let last = projection.years.last().map_or(year, |row| row.year);
        format!("{year} is outside the projection; the plan covers {first}-{last}")
    })
}
