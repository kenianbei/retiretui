use crate::params::{TaxParams, TaxTables};
use crate::plan::{Dollars, FilingStatus, Plan};
use crate::project::{
    covered_count, horizon_year, inflation_factor, min_active_cliff_threshold, plan_inflation,
    scale,
};
use crate::tax;

use super::{OptimizeOptions, RATE_EPSILON};

pub(super) fn conversion_window(
    plan: &Plan,
    options: &OptimizeOptions,
) -> std::ops::RangeInclusive<i16> {
    let start = options
        .start_year
        .unwrap_or(plan.plan.start_year)
        .max(plan.plan.start_year);
    let end = options
        .end_year
        .unwrap_or_else(|| default_end_year(plan, options))
        .min(horizon_year(plan));
    start..=end
}

/// The year before the destination owner's RMDs begin: once RMDs force
/// distributions, bracket space stops being discretionary.
fn default_end_year(plan: &Plan, options: &OptimizeOptions) -> i16 {
    plan.account(&options.destination)
        .and_then(|account| plan.person(&account.owner))
        .map_or(plan.plan.start_year, |owner| {
            let birth = owner.birth.year();
            birth + i16::from(tax::rmd_start_age(birth)) - 1
        })
}

/// One window year's bracket target and MAGI ceiling under one params
/// resolution; `None` when the year's params carry no such bracket rate.
pub(super) fn year_targets(
    plan: &Plan,
    tables: &TaxTables,
    year: i16,
    bracket_rate: f64,
    options: &OptimizeOptions,
) -> Option<(Dollars, Option<Dollars>)> {
    let params = tables.params_for(year, &plan_inflation(plan));
    let target = bracket_target(
        &params,
        plan.household.filing,
        bracket_rate,
        options.headroom,
    )?;
    Some((target, magi_ceiling(plan, &params, year, options)))
}

/// The strictest MAGI ceiling for one year: the requested IRMAA tier
/// (only while a lookback-later premium still lands on a covered person
/// inside the horizon), the explicit cap, and the lowest active cliff.
fn magi_ceiling(
    plan: &Plan,
    params: &TaxParams,
    year: i16,
    options: &OptimizeOptions,
) -> Option<Dollars> {
    let tier = options
        .irmaa_tier
        .filter(|_| irmaa_lookback_matters(plan, year))
        .and_then(|tier| tax::irmaa_threshold(params, plan.household.filing, tier));
    let explicit = options
        .max_magi
        .map(|max| scale(max, inflation_factor(plan, year)));
    let cliff = min_active_cliff_threshold(plan, year);
    [tier, explicit, cliff].into_iter().flatten().min()
}

/// Whether a conversion in `year` can still buy a surcharge: someone is
/// covered when its premiums land, inside the horizon.
fn irmaa_lookback_matters(plan: &Plan, year: i16) -> bool {
    let premium_year = year + tax::IRMAA_LOOKBACK_YEARS;
    premium_year <= horizon_year(plan) && covered_count(plan, premium_year) > 0
}

/// The fill ceiling for one year: the chosen bracket's top minus the
/// headroom cushion, or `None` when that year's params carry no such rate.
fn bracket_target(
    params: &TaxParams,
    filing: FilingStatus,
    rate: f64,
    headroom: Dollars,
) -> Option<Dollars> {
    let brackets = params.brackets.for_status(filing);
    let index = brackets
        .iter()
        .position(|bracket| (bracket.rate - rate).abs() < RATE_EPSILON)?;
    let top = brackets.get(index + 1)?.over;
    Some(top - headroom)
}
