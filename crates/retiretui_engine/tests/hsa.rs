//! What an HSA pays without tax, and what it pays beyond the year's medical
//! spending.

mod common;

use std::path::Path;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{Projection, project};

use common::{born_in, plan_from, run};

const HSA_FIRST: &str = "inflation = 0.0\nwithdrawal_order = [\"hsa\", \"taxable\"]";

/// A plan for `me`, born in June of `birth_year`, holding `cash` and an HSA
/// of 50,000 drained first, who spends 9,000 on health and `other` besides.
fn spending_from_an_hsa(birth_year: i16, cash: Dollars, other: Dollars) -> String {
    let body = format!(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = {cash}

[[accounts]]
id = "hsa"
kind = "hsa"
owner = "me"
balance = 50000

[[expenses]]
id = "health"
amount = 9000
medical = true
on = {{ date = 2026-01-01 }}

[[expenses]]
id = "living"
amount = {other}
on = {{ date = 2026-01-01 }}
"#
    );
    born_in(birth_year, &body).replace("inflation = 0.0", HSA_FIRST)
}

/// What 2026 drew on the HSA, what the year taxed, and the penalty it paid.
#[track_caller]
fn drawn(projection: &Projection) -> [Dollars; 3] {
    let row = projection.row(2026).unwrap();
    let hsa = row.withdrawals.get("hsa").copied().unwrap_or(0);
    [hsa, row.taxes.magi, row.taxes.penalty]
}

#[test]
fn an_hsa_pays_the_years_medical_spending_without_tax() {
    // Born June 1966: 60 in 2026.
    let within = run(&spending_from_an_hsa(1966, 0, 0));
    assert_eq!(drawn(&within), [9_000, 0, 0]);
}

#[test]
fn what_an_hsa_pays_beyond_medical_spending_is_taxed_and_penalized_before_65() {
    // Born June 1966: 60 in 2026. The 5,000 of living and the penalty on
    // what pays it are not medical: 6,250, a fifth of which is the penalty.
    let early = run(&spending_from_an_hsa(1966, 0, 5_000));
    assert_eq!(drawn(&early), [15_250, 6_250, 1_250]);
    // Born June 1961: 65 in 2026.
    let covered = run(&spending_from_an_hsa(1961, 0, 5_000));
    assert_eq!(drawn(&covered), [14_000, 5_000, 0]);
}

#[test]
fn an_hsa_in_a_plan_that_marks_nothing_medical_is_taxed_on_every_draw() {
    let unmarked = spending_from_an_hsa(1961, 0, 0).replace("medical = true\n", "");
    assert_eq!(drawn(&run(&unmarked)), [9_000, 9_000, 0]);
}

#[test]
fn the_first_pass_takes_from_an_hsa_only_what_medical_spending_frees() {
    // Born June 1966: the HSA pays the 9,000, and cash the rest.
    let early = run(&spending_from_an_hsa(1966, 100_000, 5_000));
    assert_eq!(drawn(&early), [9_000, 0, 0]);
    assert_eq!(early.years[0].withdrawals["cash"], 5_000);
    // Born June 1961: no penalty holds the HSA back, so it pays everything
    // in its place.
    let covered = run(&spending_from_an_hsa(1961, 100_000, 5_000));
    assert_eq!(drawn(&covered), [14_000, 5_000, 0]);
}

#[test]
fn a_tax_table_that_states_no_hsa_penalty_charges_the_statutory_fifth() {
    let mut tables = TaxTables::embedded();
    tables
        .add_dir(Path::new("tests/fixtures/tax-override"))
        .unwrap();
    // Born June 1967: 60 in 2027, the year the override table is for.
    let plan = spending_from_an_hsa(1967, 0, 5_000)
        .replace("start_year = 2026", "start_year = 2027")
        .replace("2026-01-01", "2027-01-01");
    let projection = project(&plan_from(&plan), &tables);
    assert_eq!(projection.years[0].taxes.penalty, 1_250);
}

#[test]
fn a_medical_mark_is_written_back_only_where_it_is_stated() {
    let plan = plan_from(&spending_from_an_hsa(1966, 0, 5_000));
    let written = plan.to_toml_string().unwrap();
    assert_eq!(written.matches("medical = true").count(), 1, "{written}");
    assert!(!written.contains("medical = false"), "{written}");
    assert_eq!(Plan::from_toml_str(&written).unwrap(), plan);
}
