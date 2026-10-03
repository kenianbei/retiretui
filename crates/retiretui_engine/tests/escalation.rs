//! How amounts escalate: the deflator, own rates and frozen nominal amounts.

mod common;

use retiretui_engine::plan::Plan;
use retiretui_engine::project::Projection;

use common::{head, run};

/// A 20,000 pension first paid in 2031, five years into the plan, with
/// `rest` among its keys.
fn deferred_pension(rest: &str) -> String {
    head(&format!(
        r#"
[[events]]
id = "retire"
trigger = {{ date = 2031-01-01 }}

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "pension"
kind = "pension"
owner = "me"
amount = 20000
start = {{ event = "retire" }}
{rest}
"#
    ))
}

/// What the pension pays in `year`: nothing where the year has no row for it.
fn pension(projection: &Projection, year: i16) -> i64 {
    let row = projection.row(year).unwrap();
    row.income.get("pension").copied().unwrap_or(0)
}

#[test]
fn deflator_tracks_inflation() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 1000000

[[expenses]]
id = "living"
amount = 40000
"#,
    );
    let projection = run(&plan);
    assert!((projection.years[0].deflator - 1.0).abs() < 1e-9);
    assert!((projection.years[1].deflator - 1.025).abs() < 1e-9);
    assert_eq!(projection.years[1].expenses, 41_000);
}

#[test]
fn own_rate_escalators_diverge_from_plan_inflation() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 1000000

[[income]]
id = "income-7"
kind = "pension"
owner = "me"
amount = 10000
cola = 0.02

[[expenses]]
id = "rent"
amount = 20000
cola = false
"#,
    );
    let projection = run(&plan);
    let third = &projection.years[2];
    // Income at its own 2%: 10,000 x 1.02^2; rent frozen nominal.
    assert_eq!(third.total_income, 10_404);
    assert_eq!(third.expenses, 20_000);
}

#[test]
fn flat_nominal_conversions_stay_flat() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 500000

[[accounts]]
id = "401k"
kind = "401k"
owner = "me"
balance = 200000

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "conversion-6"
from = "401k"
to = "roth"
amount = 20000
cola = false
end = { date = 2030-12-31 }
"#,
    );
    let projection = run(&plan);
    for row in &projection.years[..5] {
        assert_eq!(row.conversions, 20_000, "year {}", row.year);
    }
}

#[test]
fn out_of_bounds_cola_rate_is_rejected() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-8"
kind = "pension"
owner = "me"
amount = 10000
cola = 0.9
"#,
    );
    let plan = Plan::from_toml_str(&plan).unwrap();
    let issues = plan.validate();
    assert!(
        issues
            .iter()
            .any(|issue| issue.path == "income[0].cola" && issue.message.contains("rate")),
        "{issues:?}"
    );
}

#[test]
fn a_rate_from_the_start_leaves_the_amount_as_stated_in_its_first_year() {
    let from_plan = run(&deferred_pension("cola = 0.02"));
    let from_start = run(&deferred_pension("cola = 0.02\ncola_from = \"start\""));
    // From the plan's start the five years before it is paid compound too:
    // 20,000 x 1.02^5.
    assert_eq!(pension(&from_plan, 2031), 22_082);
    assert_eq!(pension(&from_start, 2030), 0);
    assert_eq!(pension(&from_start, 2031), 20_000);
    assert_eq!(pension(&from_start, 2032), 20_400);
    assert_eq!(pension(&from_start, 2033), 20_808);
}

#[test]
fn inflation_from_the_start_leaves_the_amount_as_stated_in_its_first_year() {
    let from_start = run(&deferred_pension("cola_from = \"start\""));
    assert_eq!(pension(&from_start, 2031), 20_000);
    assert_eq!(pension(&from_start, 2032), 20_500);
    // 20,000 x 1.025^3.
    assert_eq!(pension(&from_start, 2034), 21_538);
}

#[test]
fn a_frozen_amount_pays_the_same_from_either_anchor() {
    let from_plan = run(&deferred_pension("cola = false"));
    let from_start = run(&deferred_pension("cola = false\ncola_from = \"start\""));
    assert_eq!(pension(&from_start, 2040), 20_000);
    assert_eq!(from_start, from_plan);
}

#[test]
fn an_income_already_paid_at_the_plan_start_escalates_from_the_plan_start() {
    let paid_since = |rest: &str| {
        let text = deferred_pension(rest).replace("date = 2031-01-01", "date = 2020-01-01");
        run(&text)
    };
    let from_start = paid_since("cola = 0.02\ncola_from = \"start\"");
    // What it pays now, not 2020's amount grown six years.
    assert_eq!(pension(&from_start, 2026), 20_000);
    assert_eq!(pension(&from_start, 2027), 20_400);
    assert_eq!(from_start, paid_since("cola = 0.02"));
}

#[test]
fn a_one_time_income_from_its_start_is_paid_as_stated() {
    let windfall = |rest: &str| {
        let text = deferred_pension(rest)
            .replace("kind = \"pension\"", "kind = \"windfall\"")
            .replace("start = {", "on = {");
        run(&text)
    };
    // In today's dollars it is 20,000 x 1.025^5 by 2031.
    assert_eq!(pension(&windfall(""), 2031), 22_628);
    assert_eq!(pension(&windfall("cola_from = \"start\""), 2031), 20_000);
}
