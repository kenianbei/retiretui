//! Benefit estimates and the career fill, over a zero-inflation plan so
//! the figures hand-check against the formula.

mod common;

use retiretui_engine::optimize::{benefit_estimates, career_at_salary};
use retiretui_engine::params::{Inflation, TaxTables};
use retiretui_engine::plan::Plan;
use retiretui_engine::tax::{earnings_at_wage, social_security_benefit};

use common::plan_from;

const BASE: &str = r#"
schema = 1

[plan]
name = "estimate-base"
start_year = 2026
horizon_age = 90
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1964-06-15
earnings = { 2000 = 60000, 2010 = 80000, 2020 = 100000 }

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 600000

[[expenses]]
id = "living"
amount = 40000
cola = false
"#;

const BENEFIT: &str = "[[income]]\nid = \"ss\"\nkind = \"social-security\"\nowner = \"me\"\n";

fn with_income(income: &str) -> Plan {
    plan_from(&BASE.replace("[[expenses]]", &format!("{income}\n[[expenses]]")))
}

#[test]
fn the_estimates_are_the_formula_monthly_and_rise_with_the_claim() {
    let plan = plan_from(BASE);
    let estimates = benefit_estimates(&plan, &TaxTables::embedded(), "me");
    let params = TaxTables::embedded()
        .params_for(2026, &Inflation::constant(0.0))
        .social_security
        .benefit
        .unwrap();
    let earnings = &plan.person("me").unwrap().earnings;
    let expected =
        [62, 67, 70].map(|age| Some(social_security_benefit(&params, 1964, age, earnings) / 12));
    assert_eq!(estimates, expected);
    let [early, full, late] = estimates.map(Option::unwrap);
    assert!(early < full && full < late, "{estimates:?}");
}

#[test]
fn an_income_held_computed_or_typed_estimates_as_one_made_up() {
    let made_up = benefit_estimates(&plan_from(BASE), &TaxTables::embedded(), "me");
    let computed = with_income(&format!(
        "{BENEFIT}start = {{ age = 64, owner = \"me\" }}\n"
    ));
    let typed = with_income(&format!(
        "{BENEFIT}amount = 1\nstart = {{ date = 2030-01-01 }}\n"
    ));
    assert_eq!(
        benefit_estimates(&computed, &TaxTables::embedded(), "me"),
        made_up
    );
    assert_eq!(
        benefit_estimates(&typed, &TaxTables::embedded(), "me"),
        made_up
    );
}

#[test]
fn no_record_or_no_room_estimates_nothing() {
    let no_record = plan_from(&BASE.replace("earnings = {", "# earnings = {"));
    assert_eq!(
        benefit_estimates(&no_record, &TaxTables::embedded(), "me"),
        [None; 3]
    );
    assert_eq!(
        benefit_estimates(&plan_from(BASE), &TaxTables::embedded(), "you"),
        [None; 3]
    );
    let short = plan_from(&BASE.replace("horizon_age = 90", "horizon_age = 70"));
    let [early, full, late] = benefit_estimates(&short, &TaxTables::embedded(), "me");
    assert!(early.is_some() && full.is_some() && late.is_none());
}

#[test]
fn a_career_is_the_wage_indexed_salary_before_the_plan() {
    let salaried = with_income(
        "[[income]]\nid = \"pay\"\nkind = \"salary\"\nowner = \"me\"\namount = 100000\n",
    );
    let params = TaxTables::embedded()
        .params_for(2026, &Inflation::constant(0.0))
        .social_security
        .benefit
        .unwrap();
    assert_eq!(
        career_at_salary(&salaried, &TaxTables::embedded(), "me").unwrap(),
        earnings_at_wage(&params, 100_000, 2026, 1986..=2025)
    );
    let unpaid = career_at_salary(&plan_from(BASE), &TaxTables::embedded(), "me").unwrap_err();
    assert!(unpaid.contains("import a statement"), "{unpaid}");
    assert_eq!(
        career_at_salary(&plan_from(BASE), &TaxTables::embedded(), "you").unwrap_err(),
        "no person `you`"
    );
}
