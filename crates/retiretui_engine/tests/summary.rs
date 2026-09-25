//! The summary aggregated over a projection, and the unfunded years.

mod common;

use common::{head, run};

#[test]
fn summary_aggregates_the_ledger() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-4"
kind = "salary"
owner = "me"
amount = 100000

[[expenses]]
id = "living"
amount = 50000
"#,
    );
    let projection = run(&plan);
    let nominal = projection.summary(false);
    let last = projection.years.last().unwrap();
    assert_eq!(nominal.final_net_worth, last.net_worth);
    assert_eq!(nominal.final_deferred, last.class_totals.deferred);
    assert_eq!(nominal.peak_year, last.year);
    assert_eq!(nominal.peak_net_worth, last.net_worth);
    let taxes: i64 = projection.years.iter().map(|row| row.taxes.total).sum();
    assert_eq!(nominal.lifetime_taxes, taxes);
    assert_eq!(nominal.lifetime_conversions, 0);
    assert_eq!(nominal.lifetime_unfunded, 0);
    assert_eq!(nominal.first_unfunded_year, None);
    let deflated = projection.summary(true);
    assert!(deflated.final_net_worth < nominal.final_net_worth);
    assert!(deflated.lifetime_taxes < nominal.lifetime_taxes);
    assert_eq!(deflated.first_unfunded_year, None);
}

#[test]
fn summary_reports_the_first_unfunded_year() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000

[[expenses]]
id = "living"
amount = 40000
"#,
    );
    let projection = run(&plan);
    let summary = projection.summary(false);
    let first_shortfall = projection
        .years
        .iter()
        .find(|row| row.unfunded > 0)
        .map(|row| row.year);
    assert!(first_shortfall.is_some(), "plan should run out of money");
    assert_eq!(summary.first_unfunded_year, first_shortfall);
    let unfunded: i64 = projection.years.iter().map(|row| row.unfunded).sum();
    assert_eq!(summary.lifetime_unfunded, unfunded);
    assert_eq!(summary.peak_year, 2026);
}

#[test]
fn unfunded_years_are_reported() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 10000

[[expenses]]
id = "living"
amount = 50000
"#,
    );
    let projection = run(&plan);
    let first = &projection.years[0];
    assert_eq!(first.unfunded, 40_000);
    assert_eq!(first.net_worth, 0);
    assert_eq!(first.taxes.total, 0);
}
