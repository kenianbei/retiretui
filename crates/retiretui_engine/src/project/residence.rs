use crate::params::TaxTables;
use crate::plan::{Issue, Plan, Residency, US};

use super::resolve::Resolver;
use super::{horizon_year, plan_inflation};

/// Where the household lives for the whole of `year`: the residency begun
/// latest at or before it, the one listed later where two begin in the same
/// year.
fn residence_in<'plan>(
    plan: &'plan Plan,
    resolver: &Resolver,
    year: i16,
) -> Option<(usize, &'plan Residency)> {
    let began = |residency: &Residency| match &residency.from {
        None => Some(i16::MIN),
        Some(from) => resolver.trigger_year(plan, from),
    };
    let lived = plan.residency.iter().enumerate();
    lived
        .filter_map(|(at, residency)| Some((began(residency)?, at, residency)))
        .filter(|(began, ..)| *began <= year)
        .max_by_key(|(began, ..)| *began)
        .map(|(_, at, residency)| (at, residency))
}

/// The U.S. state taxing `year`. `None` abroad and where the plan states no
/// residency, which owe no state tax.
pub(super) fn state_in<'plan>(
    plan: &'plan Plan,
    resolver: &Resolver,
    year: i16,
) -> Option<&'plan str> {
    taxing_state(residence_in(plan, resolver, year)?.1)
}

fn taxing_state(residency: &Residency) -> Option<&str> {
    residency
        .state
        .as_deref()
        .filter(|_| residency.country == US)
}

/// Refuses a U.S. state that a year it is lived in has no table for, which
/// would otherwise be projected as though it taxed nothing. Reports the
/// first such year of each residency.
#[must_use]
pub(super) fn check_modeled_states(plan: &Plan, tables: &TaxTables) -> Vec<Issue> {
    let resolver = Resolver::new(plan);
    let inflation = plan_inflation(plan);
    let mut issues: Vec<Issue> = Vec::new();
    for year in plan.plan.start_year..=horizon_year(plan) {
        let Some((at, residency)) = residence_in(plan, &resolver, year) else {
            continue;
        };
        let Some(state) = taxing_state(residency) else {
            continue;
        };
        let params = tables.params_for(year, &inflation);
        let path = format!("residency[{at}].state");
        if params.states.contains_key(state) || issues.iter().any(|issue| issue.path == path) {
            continue;
        }
        let message = format!(
            "state income tax is not modeled for \"{state}\" in {year}; without a residency \
             the plan projects federal tax only"
        );
        issues.push(Issue { path, message });
    }
    issues
}
