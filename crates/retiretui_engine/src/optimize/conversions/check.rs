use crate::params::TaxTables;
use crate::plan::{Issue, Plan, TreatmentClass, push_issue};
use crate::project::plan_inflation;

use super::targets::conversion_window;
use super::{OptimizeOptions, RATE_EPSILON};

/// Checks the shared constraints, plus the bracket when one is targeted.
pub(super) fn check_options(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
    bracket_rate: Option<f64>,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    check_accounts(plan, options, &mut issues);
    if let Some(rate) = bracket_rate {
        check_bracket(plan, tables, rate, &mut issues);
    }
    if conversion_window(plan, options).is_empty() {
        push_issue(
            &mut issues,
            "options.start_year",
            "the conversion window is empty",
        );
    }
    if options.headroom < 0 {
        push_issue(&mut issues, "options.headroom", "must not be negative");
    }
    check_ceilings(plan, tables, options, &mut issues);
    for (path, cap) in [
        ("options.annual_max", options.annual_max),
        ("options.total_max", options.total_max),
    ] {
        if cap.is_some_and(|cap| cap <= 0) {
            push_issue(&mut issues, path, "must be positive when present");
        }
    }
    issues
}

fn check_accounts(plan: &Plan, options: &OptimizeOptions, issues: &mut Vec<Issue>) {
    let destination_owner = match plan.account(&options.destination) {
        None => {
            push_issue(
                issues,
                "options.destination",
                format!("unknown account `{}`", options.destination),
            );
            None
        }
        Some(account) if account.treatment() != TreatmentClass::Roth => {
            push_issue(issues, "options.destination", "must be a Roth account");
            None
        }
        Some(account) => Some(account.owner.clone()),
    };
    if options.sources.is_empty() {
        push_issue(issues, "options.sources", "at least one source is required");
    }
    for (i, source) in options.sources.iter().enumerate() {
        let path = format!("options.sources[{i}]");
        match plan.account(source) {
            None => push_issue(issues, path, format!("unknown account `{source}`")),
            Some(account) if account.treatment() != TreatmentClass::Deferred => {
                push_issue(issues, path, "must be a tax-deferred account");
            }
            Some(account) => {
                if destination_owner
                    .as_deref()
                    .is_some_and(|owner| owner != account.owner)
                {
                    push_issue(issues, path, "must share the destination's owner");
                }
            }
        }
    }
}

fn check_ceilings(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
    issues: &mut Vec<Issue>,
) {
    if let Some(tier) = options.irmaa_tier {
        if plan.medicare.is_none() {
            push_issue(
                issues,
                "options.irmaa_tier",
                "requires a [medicare] section in the plan",
            );
        }
        let tiers = tables
            .params_for(plan.plan.start_year, &plan_inflation(plan))
            .irmaa
            .len();
        if usize::from(tier) >= tiers {
            push_issue(
                issues,
                "options.irmaa_tier",
                format!("only {tiers} tiers exist"),
            );
        }
    }
    if options.max_magi.is_some_and(|max| max <= 0) {
        push_issue(issues, "options.max_magi", "must be positive when present");
    }
}

fn check_bracket(plan: &Plan, tables: &TaxTables, rate: f64, issues: &mut Vec<Issue>) {
    let params = tables.params_for(plan.plan.start_year, &plan_inflation(plan));
    let brackets = params.brackets.for_status(plan.household.filing);
    match brackets
        .iter()
        .position(|bracket| (bracket.rate - rate).abs() < RATE_EPSILON)
    {
        None => push_issue(
            issues,
            "options.bracket_rate",
            format!("no bracket with rate {rate}"),
        ),
        Some(index) if index + 1 == brackets.len() => push_issue(
            issues,
            "options.bracket_rate",
            "the top bracket has no ceiling to fill",
        ),
        Some(_) => {}
    }
}
