//! What the projection taxes: penalties, RMDs, conversions, benefits and gains.

mod common;

use retiretui_engine::plan::Dollars;

use common::{head, run};

#[test]
fn early_deferred_draw_pays_penalty_only_as_last_resort() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 30000

[[accounts]]
id = "401k"
kind = "401k"
owner = "me"
balance = 500000

[[expenses]]
id = "living"
amount = 50000
"#,
    );
    let projection = run(&plan);
    let first = &projection.years[0];
    // Cash covers 30,000 penalty-free; the remainder must come from the
    // 401k at age 46 and carries the 10% penalty.
    assert_eq!(first.withdrawals["cash"], 30_000);
    let deferred_draw = first.withdrawals["401k"];
    assert!(deferred_draw > 0);
    assert_eq!(
        first.taxes.penalty,
        (deferred_draw as f64 * 0.10).round() as i64
    );
    let exempt = plan.replace("kind = \"401k\"", "kind = \"457b\"");
    let projection = run(&exempt);
    assert_eq!(projection.years[0].taxes.penalty, 0);
}

#[test]
fn rmds_are_forced_and_taxed() {
    let plan = r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 80
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1950-01-15

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 1000000
"#;
    let projection = run(plan);
    let first = &projection.years[0];
    // Age 76 in 2026: divisor 23.7.
    let expected = (1_000_000f64 / 23.7).round() as i64;
    assert_eq!(first.rmds, expected);
    assert_eq!(first.withdrawals["ira"], expected);
    // Taxable 42,194 - 16,100 = 26,094 -> 1,240 + 12% * 13,694 = 2,883.
    assert_eq!(first.taxes.ordinary, 2_883);
    assert_eq!(first.surplus, expected - first.taxes.total);
}

#[test]
fn conversions_move_money_and_are_taxed_as_ordinary() {
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
balance = 100000

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "conversion-5"
from = "401k"
to = "roth"
amount = 40000
on = { date = 2026-06-01 }
"#,
    );
    let projection = run(&plan);
    let first = &projection.years[0];
    assert_eq!(first.conversions, 40_000);
    assert_eq!(first.balances["401k"], 60_000);
    assert_eq!(first.balances["roth"], 40_000);
    // Taxable 40,000 - 16,100 = 23,900 -> 1,240 + 12% * 11,500 = 2,620.
    assert_eq!(first.taxes.ordinary, 2_620);
}

#[test]
fn social_security_taxation_flows_through() {
    let plan = r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 75
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1958-03-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 100000

[[income]]
id = "income-5"
kind = "pension"
owner = "me"
amount = 40000

[[income]]
id = "income-6"
kind = "social-security"
owner = "me"
amount = 20000
start = { date = 2026-01-01 }
"#;
    let projection = run(plan);
    let first = &projection.years[0];
    // Provisional 50,000: capped at 85% of the 20,000 benefit.
    assert_eq!(first.taxes.taxable_social_security, 17_000);
}

#[test]
fn taxable_gains_use_basis_fraction() {
    let drawn = |living: Dollars| {
        let plan = head(&format!(
            r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "brokerage"
kind = "brokerage"
owner = "me"
balance = 100000
basis = 40000

[[expenses]]
id = "living"
amount = {living}
"#
        ));
        run(&plan).years[0].clone()
    };
    let half = drawn(50_000);
    // 60% of every withdrawn dollar is gain; taxable income stays inside
    // the zero LTCG band, so no tax and no gross-up.
    assert_eq!(half.withdrawals["brokerage"], 50_000);
    assert_eq!(half.taxes.magi, 30_000);
    assert_eq!(half.taxes.ltcg, 0);
    assert_eq!(half.taxes.total, 0);
    let all = drawn(100_000);
    assert_eq!(all.withdrawals["brokerage"], 100_000);
    assert_eq!(all.taxes.magi, 60_000);
}
