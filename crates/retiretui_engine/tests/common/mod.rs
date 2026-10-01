//! Helpers the engine's test crates share.
#![allow(dead_code, reason = "each test crate uses its own share of these")]

use retiretui_engine::optimize::{OptimizeOptions, SweptBracket, optimize_conversions};
use retiretui_engine::params::{BenefitParams, Inflation, TaxParams, TaxTables};
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::{Action, ContributionNote, Projection, project};

pub const FULL: &str = include_str!("../fixtures/full.toml");

/// The embedded benefit formula's parameters as a 2026 plan reads them.
pub fn benefit_params() -> BenefitParams {
    TaxTables::embedded()
        .params_for(2026, &Inflation::constant(0.025))
        .social_security
        .benefit
        .unwrap()
}

/// The embedded 2026 tables as a zero-inflation plan reads them.
pub fn params_2026() -> TaxParams {
    TaxTables::embedded().params_for(2026, &Inflation::constant(0.0))
}

/// A ladder from `k` into `r`, held to nothing.
pub fn ladder_options() -> OptimizeOptions {
    OptimizeOptions {
        sources: vec!["k".to_owned()],
        destination: "r".to_owned(),
        start_year: None,
        end_year: None,
        annual_max: None,
        total_max: None,
        headroom: 0,
        irmaa_tier: None,
        max_magi: None,
        gains_rate: None,
    }
}

pub fn searched_ladder(plan: &Plan, options: &OptimizeOptions, rate: f64) -> SweptBracket {
    optimize_conversions(plan, &TaxTables::embedded(), options, rate)
        .unwrap()
        .ladder
}

pub fn plan_from(text: &str) -> Plan {
    let plan = Plan::from_toml_str(text).unwrap();
    assert!(plan.validate().is_empty(), "{:?}", plan.validate());
    plan
}

pub fn issues(text: &str) -> Vec<Issue> {
    Plan::from_toml_str(text).unwrap().validate()
}

pub fn run(text: &str) -> Projection {
    project(&plan_from(text), &TaxTables::embedded())
}

#[track_caller]
pub fn assert_issue(found: &[Issue], path: &str, message: &str) {
    assert!(
        found
            .iter()
            .any(|issue| issue.path == path && issue.message.contains(message)),
        "no issue at `{path}` containing `{message}` in {found:?}"
    );
}

pub fn head(text: &str) -> String {
    format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 70
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1980-06-15
{text}
"#
    )
}

/// The year's contribution into `account`: employee and employer amounts
/// and its notes, or nothing paid.
pub fn contribution<'a>(
    projection: &'a Projection,
    year: usize,
    account: &str,
) -> (i64, i64, &'a [ContributionNote]) {
    projection.years[year]
        .actions
        .iter()
        .find_map(|action| match action {
            Action::Contribution {
                account: paid,
                employee,
                employer,
                notes,
            } if paid == account => Some((*employee, *employer, notes.as_slice())),
            _ => None,
        })
        .unwrap_or((0, 0, &[]))
}
