//! The rules a plan's residencies keep: codes from the closed sets, a state
//! exactly where the country has them, and a timeline with one beginning.

use super::places::{COUNTRIES, US, US_STATES, place_name};
use super::{Issue, Plan, push_issue};

pub(super) fn check_residency(plan: &Plan, issues: &mut Vec<Issue>) {
    for (at, residency) in plan.residency.iter().enumerate() {
        let country = residency.country.as_str();
        if place_name(COUNTRIES, country).is_none() {
            push_issue(
                issues,
                format!("residency[{at}].country"),
                format!("`{country}` is not an ISO 3166-1 two-letter country code"),
            );
            continue;
        }
        let path = || format!("residency[{at}].state");
        match residency.state.as_deref() {
            None if country == US => push_issue(issues, path(), "a U.S. residency names its state"),
            Some(state) if country != US => push_issue(
                issues,
                path(),
                format!("`{state}` is given, but only a `{US}` residency has a state"),
            ),
            Some(state) if place_name(US_STATES, state).is_none() => push_issue(
                issues,
                path(),
                format!("`{state}` is not a U.S. state code or `dc`"),
            ),
            _ => {}
        }
    }
    check_beginning(plan, issues);
}

/// Each residency ends where the next begins, so exactly one begins with
/// the plan.
fn check_beginning(plan: &Plan, issues: &mut Vec<Issue>) {
    let placed = plan.residency.iter().enumerate();
    let mut initial = placed.filter(|(_, residency)| residency.from.is_none());
    if initial.next().is_none() && !plan.residency.is_empty() {
        push_issue(
            issues,
            "residency",
            "no residency is without `from`, so the plan starts nowhere",
        );
    }
    for (at, _) in initial {
        push_issue(
            issues,
            format!("residency[{at}].from"),
            "only one residency begins with the plan; a later one says when the move is",
        );
    }
}
