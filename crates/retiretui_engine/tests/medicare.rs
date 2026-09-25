//! Medicare surcharges and MAGI cliffs in the projection.

mod common;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use common::{head, run};

/// A 64-year-old with a fat conversion in year one and `[medicare]` on:
/// the surcharge lands exactly two years later, when covered.
fn medicare_plan(extra: &str) -> String {
    format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 75
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1962-06-15

[medicare]
{extra}

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 400000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 400000

[[accounts]]
id = "r"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "conversion-4"
from = "k"
to = "r"
amount = 200000
on = {{ date = 2026-06-01 }}
cola = false

[[expenses]]
id = "living"
amount = 30000
cola = false
"#
    )
}

#[test]
fn irmaa_surcharge_lands_two_years_after_the_magi() {
    let projection = run(&medicare_plan(""));
    // 2026: MAGI ~200k (the conversion). Ages 64/65/66 in 2026/27/28.
    assert_eq!(projection.years[0].medicare, 0, "not yet covered");
    assert_eq!(projection.years[1].medicare, 0, "lookback below tiers");
    let surcharge_2028 = projection.years[2].medicare;
    assert!(surcharge_2028 > 0, "2028 pays for 2026's conversion");
    assert_eq!(surcharge_2028, 3_552 + 700, "tier three at 200k MAGI");
    assert!(
        projection.years[3].medicare < surcharge_2028,
        "2029 falls back to the quiet-year tier"
    );
    assert!(projection.years[0].taxes.magi >= 200_000);
}

#[test]
fn prior_magi_seeds_and_part_d_toggles() {
    let seeded = run(&medicare_plan("prior_magi = [120000, 250000]"));
    // 2027 (first covered year) looks back to 2025 = 250k -> tier four;
    // 2026 is uncovered at 64.
    assert_eq!(seeded.years[0].medicare, 0);
    assert_eq!(seeded.years[1].medicare, 4_884 + 967);
    let no_d = run(&medicare_plan(
        "prior_magi = [120000, 250000]\npart_d = false",
    ));
    assert_eq!(no_d.years[1].medicare, 4_884);
}

#[test]
fn cliffs_spend_when_magi_crosses_and_lapse_at_medicare_age() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 500000

[[income]]
id = "income-3"
kind = "salary"
owner = "me"
amount = 100000
cola = false
end = { date = 2028-12-31 }

[[expenses]]
id = "living"
amount = 40000
cola = false

[[cliffs]]
id = "aca"
magi_over = 90000
cost = 12000
cola = false
"#,
    );
    let projection = run(&plan);
    // Salary years: MAGI 100k > 90k -> the cliff costs 12k.
    assert_eq!(projection.years[0].medicare, 12_000, "2026 crossed");
    assert_eq!(projection.years[2].medicare, 12_000, "2028 crossed");
    assert_eq!(projection.years[3].medicare, 0, "2029: salary gone, under");
    assert!(projection.years[0].taxes.magi > 90_000);
}

#[test]
fn irmaa_purchase_prices_the_lookahead() {
    use retiretui_engine::project::irmaa_purchase;
    let plan = Plan::from_toml_str(&medicare_plan("part_d = true")).unwrap();
    let tables = TaxTables::embedded();
    // 200k MAGI in 2026 lands on a covered 66-year-old in 2028: tier three.
    assert_eq!(irmaa_purchase(&plan, &tables, 2026, 200_000), 3_552 + 700);
    assert_eq!(irmaa_purchase(&plan, &tables, 2026, 50_000), 0);
    assert_eq!(
        irmaa_purchase(&plan, &tables, 2036, 200_000),
        0,
        "premiums past the horizon are never charged"
    );
}
