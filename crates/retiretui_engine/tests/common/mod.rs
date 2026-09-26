//! Helpers the engine's test crates share.
#![allow(dead_code, reason = "each test crate uses its own share of these")]

use retiretui_engine::params::{BenefitParams, Inflation, TaxTables};
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::{Projection, project};

pub const FULL: &str = include_str!("../fixtures/full.toml");

/// The embedded benefit formula's parameters as a 2026 plan reads them.
pub fn benefit_params() -> BenefitParams {
    TaxTables::embedded()
        .params_for(2026, &Inflation::constant(0.025))
        .social_security
        .benefit
        .unwrap()
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
