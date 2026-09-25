//! How amounts escalate: the deflator, own rates and frozen nominal amounts.

mod common;

use retiretui_engine::plan::Plan;

use common::{head, run};

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
