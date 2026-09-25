//! Social Security benefits computed from an earnings record.

mod common;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Projection, validate_plan};

use common::{head, run};

#[test]
fn social_security_without_an_amount_is_computed_from_earnings() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-9"
kind = "salary"
owner = "me"
amount = 100000
end = { age = 66, owner = "me" }

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }
"#,
    );
    let projection = run(&plan);
    // Born 1980-06-15: 21 nominal salary years 2026-2046 at 2.5%, nothing
    // before, indexed to 2040's wage (2024's grown 3.6% a year) and bent
    // at 2042's points [2,264, 13,646]: AIME 7,646, PIA 3,759.80, so
    // 45,108 a year in 2042 dollars, 30,386 in 2026's; claimed at 67 it
    // carries six COLAs by 2048 and pays July on in 2047.
    let income = |year| {
        let row = projection.years.iter().find(|row| row.year == year);
        row.map_or(0, |row| row.income.get("ss").copied().unwrap_or(0))
    };
    assert_eq!(income(2046), 0);
    assert_eq!(income(2047), 29_771);
    assert_eq!(income(2048), 52_312);
    assert_eq!(projection.deflate_in(2048, income(2048)), 30_386);
}

#[test]
fn a_plan_may_state_the_wage_growth_a_benefit_is_indexed_over() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-10"
kind = "salary"
owner = "me"
amount = 100000
end = { age = 66, owner = "me" }

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }
"#,
    )
    .replace("inflation = 0.025", "inflation = 0.025\nwage_growth = 0.0");
    let projection = run(&plan);
    let paid = projection
        .years
        .iter()
        .find(|row| row.year == 2048)
        .unwrap()
        .income["ss"];
    // A wage frozen at 2024's indexes nothing up: less than the 52,312 the
    // table's 3.6% pays.
    assert!(paid < 52_312 && paid > 0, "{paid}");
}

#[test]
fn a_computed_benefit_carries_colas_from_the_age_62_year() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-11"
kind = "salary"
owner = "me"
amount = 100000
end = { age = 66, owner = "me" }

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
cola = false
start = { age = 67, owner = "me" }
"#,
    );
    let projection = run(&plan);
    let paid = |year| {
        projection
            .years
            .iter()
            .find(|row| row.year == year)
            .unwrap()
            .income["ss"]
    };
    // Frozen, the benefit is the eligibility-year amount itself; the
    // escalating income above pays it grown 2.5% for the six years from
    // 2042, not the 22 from the plan's start.
    assert_eq!(paid(2048), 45_108);
    assert_eq!(paid(2049), 45_108);
    assert_eq!(paid(2047), 45_108 * 7 / 12);
}

#[test]
fn a_benefit_started_on_an_age_pays_its_first_year_from_the_birthday_month() {
    let stated = |start| {
        head(&format!(
            r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
amount = 12000
cola = false
start = {start}
"#
        ))
    };
    let paid = |projection: &Projection, year| {
        projection
            .years
            .iter()
            .find(|row| row.year == year)
            .unwrap()
            .income["ss"]
    };
    // Born 15 June: 67 is attained on the 14th, and June on is paid.
    let at_age = run(&stated(r#"{ age = 67, owner = "me" }"#));
    assert_eq!(paid(&at_age, 2047), 7_000);
    assert_eq!(paid(&at_age, 2048), 12_000);
    let on_date = run(&stated("{ date = 2047-01-01 }"));
    assert_eq!(paid(&on_date, 2047), 12_000, "a date says January");
}

#[test]
fn a_stated_social_security_amount_wins_over_the_record() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
amount = 40000
start = { age = 62, owner = "me" }
"#,
    )
    .replace(
        "birth = 1980-06-15",
        "birth = 1980-06-15\nearnings = { 2000 = 30000, 2001 = 30000 }",
    );
    let projection = run(&plan);
    let first_full_year = projection
        .years
        .iter()
        .find(|row| row.year == 2043)
        .unwrap();
    let benefit = projection.deflate_in(2043, first_full_year.income["ss"]);
    assert!((benefit - 40_000).abs() <= 1, "{benefit}");
}

#[test]
fn a_computed_benefit_needs_a_claim_at_sixty_two() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-12"
kind = "social-security"
owner = "me"
start = { age = 60, owner = "me" }
"#,
    );
    let plan = Plan::from_toml_str(&plan).unwrap();
    let issues = validate_plan(&plan, &TaxTables::embedded());
    assert!(
        issues
            .iter()
            .any(|issue| issue.path == "income[0].start" && issue.message.contains("62")),
        "{issues:?}"
    );
}

#[test]
fn a_computed_benefit_needs_the_formula_amounts() {
    let plan = head(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "income-13"
kind = "social-security"
owner = "me"
start = { age = 67, owner = "me" }
"#,
    )
    .replace("start_year = 2026", "start_year = 2027");
    let plan = Plan::from_toml_str(&plan).unwrap();
    let mut tables = TaxTables::embedded();
    tables
        .add_dir(std::path::Path::new("tests/fixtures/tax-override"))
        .unwrap();
    let issues = validate_plan(&plan, &tables);
    assert!(
        issues
            .iter()
            .any(|issue| issue.path == "income[0].amount" && issue.message.contains("2027")),
        "{issues:?}"
    );
}
