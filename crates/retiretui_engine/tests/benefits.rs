//! Social Security benefits computed from an earnings record.

mod common;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Projection, validate_plan};
use retiretui_engine::tax::earnings_at_wage;

use common::{head, run};

/// A salary to 66 and a benefit computed at 67, after the person's line:
/// what a stated record goes before.
const SALARIED_CLAIM: &str = r#"
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
"#;

/// `record` as the person's `earnings` line.
fn record_line(record: &std::collections::BTreeMap<i16, i64>) -> String {
    let years: Vec<String> = (record.iter())
        .map(|(year, amount)| format!("{year} = {amount}"))
        .collect();
    format!("earnings = {{ {} }}", years.join(", "))
}

/// A record of one year without earnings: stated, so nothing is filled.
const NOTHING_BEFORE: &str = "earnings = { 2000 = 0 }";

#[test]
fn social_security_without_an_amount_is_computed_from_earnings() {
    let plan = head(&format!("{NOTHING_BEFORE}\n{SALARIED_CLAIM}"));
    let projection = run(&plan);
    // Born 1980-06-15: 21 nominal salary years 2026-2046 at 2.5%, nothing
    // before, indexed to 2040's wage (2024's grown 3.6% a year) and bent
    // at 2042's points [2,264, 13,646]: AIME 7,646, PIA 3,759.80, so
    // 45,108 a year in 2042 dollars, 30,386 in 2026's; claimed at 67 it
    // carries six COLAs by 2048 and pays July on in 2047.
    let income = |year| {
        let row = projection.row(year);
        row.map_or(0, |row| row.income.get("ss").copied().unwrap_or(0))
    };
    assert_eq!(income(2046), 0);
    assert_eq!(income(2047), 29_771);
    assert_eq!(income(2048), 52_312);
    assert_eq!(projection.deflate_in(2048, income(2048)), 30_386);
}

#[test]
fn an_empty_record_is_filled_with_a_career_at_the_first_years_salary() {
    let filled = run(&head(SALARIED_CLAIM));
    let params = common::benefit_params();
    let career = earnings_at_wage(&params, 100_000, 2026, 2002..=2025);
    let record = record_line(&career);
    let stated = run(&head(&format!("{record}\n{SALARIED_CLAIM}")));
    let paid = |projection: &Projection| projection.row(2048).unwrap().income["ss"];
    assert_eq!(paid(&filled), paid(&stated));
    let partial = run(&head(&format!("{NOTHING_BEFORE}\n{SALARIED_CLAIM}")));
    assert!(
        paid(&filled) > paid(&partial),
        "a stated record is not filled"
    );
}

#[test]
fn a_plan_may_state_the_wage_growth_a_benefit_is_indexed_over() {
    let plan = head(&format!(
        "{NOTHING_BEFORE}\n{}",
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
"#
    ))
    .replace("inflation = 0.025", "inflation = 0.025\nwage_growth = 0.0");
    let projection = run(&plan);
    let paid = projection.row(2048).unwrap().income["ss"];
    // A wage frozen at 2024's indexes nothing up: less than the 52,312 the
    // table's 3.6% pays.
    assert!(paid < 52_312 && paid > 0, "{paid}");
}

#[test]
fn a_computed_benefit_carries_colas_from_the_age_62_year() {
    let plan = head(&format!(
        "{NOTHING_BEFORE}\n{}",
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
"#
    ));
    let projection = run(&plan);
    let paid = |year| projection.row(year).unwrap().income["ss"];
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
    let paid = |projection: &Projection, year| projection.row(year).unwrap().income["ss"];
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
    let first_full_year = projection.row(2043).unwrap();
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
    let issues = validate_plan(&plan, &common::with_override());
    assert!(
        issues
            .iter()
            .any(|issue| issue.path == "income[0].amount" && issue.message.contains("2027")),
        "{issues:?}"
    );
}

/// A frozen benefit computed from the salary to 66, started by `start`,
/// with whatever the person's line needs after it in `extra`.
fn frozen_claim(start: &str, extra: &str) -> String {
    head(&format!(
        r#"
[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[income]]
id = "salary"
kind = "salary"
owner = "me"
amount = 100000
end = {{ age = 66, owner = "me" }}

[[income]]
id = "ss"
kind = "social-security"
owner = "me"
cola = false
start = {start}
{extra}
"#
    ))
}

fn paid_by(text: &str) -> impl Fn(i16) -> f64 + use<> {
    let projection = run(text);
    move |year| {
        projection
            .row(year)
            .map_or(0.0, |row| row.income["ss"] as f64)
    }
}

fn assert_near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.001, "{actual} vs {expected}");
}

#[test]
fn a_date_claim_is_priced_and_paid_from_its_month() {
    // Born 1980-06-15, full retirement age 67 is June 2047, paid seven
    // months of that year; October is four months past it, paid three and
    // credited from 2048.
    let at_67 = paid_by(&frozen_claim(r#"{ age = 67, owner = "me" }"#, ""));
    let dated = paid_by(&frozen_claim("{ date = 2047-10-01 }", ""));
    assert_near(dated(2047) * 4.0 / (at_67(2047) * 12.0 / 7.0), 1.0);
    assert_near(dated(2048) / at_67(2048), 1.0 + 4.0 * 2.0 / 300.0);
    let chained = paid_by(&frozen_claim(
        r#"{ event = "retire" }"#,
        "[[events]]\nid = \"retire\"\ntrigger = { date = 2047-10-01 }\n",
    ));
    assert_near(chained(2047), dated(2047));
    assert_near(chained(2048), dated(2048));
}

#[test]
fn delayed_credits_earned_in_the_claim_year_are_paid_from_the_next_january() {
    // At 68 in June 2048: seven months of credit by January, twelve after.
    let at_68 = paid_by(&frozen_claim(r#"{ age = 68, owner = "me" }"#, ""));
    let credited = |months: f64| 1.0 + months * 2.0 / 300.0;
    assert_near(
        at_68(2048) * 12.0 / 7.0 / at_68(2049),
        credited(7.0) / credited(12.0),
    );
    // At 70 every credit is paid at once.
    let at_70 = frozen_claim(r#"{ age = 70, owner = "me" }"#, "");
    let at_70 = paid_by(&at_70.replace("horizon_age = 70", "horizon_age = 75"));
    assert_near(at_70(2050) * 12.0 / 7.0 / at_70(2051), 1.0);
}

#[test]
fn a_computed_benefit_claimed_before_the_month_62_is_attained_is_refused() {
    let issues = |start| {
        let plan = Plan::from_toml_str(&frozen_claim(start, "")).unwrap();
        validate_plan(&plan, &TaxTables::embedded())
    };
    let early = issues("{ date = 2042-03-01 }");
    assert_eq!(early.len(), 1, "{early:?}");
    assert!(
        early[0].message.ends_with("claims at 61 and 9 months"),
        "{early:?}"
    );
    assert!(issues("{ date = 2042-06-01 }").is_empty());
}

#[test]
fn a_benefit_eligible_before_the_plan_carries_the_published_colas() {
    // Born 1960, 62 in 2022: 2022-2025's published COLAs carry the benefit
    // to the plan's 2026 dollars, frozen after.
    let record: std::collections::BTreeMap<i16, i64> =
        (1982..=2021).map(|year| (year, 60_000)).collect();
    let stated = record_line(&record);
    let text = frozen_claim(r#"{ age = 67, owner = "me" }"#, "")
        .replace(
            "birth = 1980-06-15",
            &format!("birth = 1960-06-15\n{stated}"),
        )
        .replace("amount = 100000", "amount = 0");
    let paid = paid_by(&text);
    let params = common::benefit_params();
    let published = [0.087, 0.032, 0.025, 0.028];
    let expected =
        retiretui_engine::tax::social_security_benefit(&params, 1960, 67 * 12, &record, &published);
    assert_near(paid(2028), expected as f64);
}

#[test]
fn a_benefit_claimed_before_the_plan_is_priced_at_the_age_it_was_claimed() {
    // Born 1960-06-15 and claimed at 62 in 2022: 59 months early, not the
    // 66 the owner is in the plan's first year.
    let record = (1982..=2021).map(|year| (year, 60_000)).collect();
    let text = frozen_claim(r#"{ age = 62, owner = "me" }"#, "")
        .replace(
            "birth = 1980-06-15",
            &format!("birth = 1960-06-15\n{}", record_line(&record)),
        )
        .replace("amount = 100000", "amount = 0");
    let published = [0.087, 0.032, 0.025, 0.028];
    let expected = retiretui_engine::tax::social_security_benefit(
        &common::benefit_params(),
        1960,
        62 * 12 + 1,
        &record,
        &published,
    );
    assert_near(paid_by(&text)(2026), expected as f64);
}

#[test]
fn a_computed_benefit_under_a_rate_of_its_own_grows_at_it_from_the_age_62_year() {
    let plan = head(&format!("{NOTHING_BEFORE}\n{SALARIED_CLAIM}")).replace(
        "kind = \"social-security\"",
        "kind = \"social-security\"\ncola = 0.03",
    );
    let projection = run(&plan);
    let paid = |year| projection.row(year).unwrap().income["ss"];
    // 45,108 in 2042 dollars at 3% from 2042, not the plan's 2.5%, June on in
    // 2047; kept as 28,110 whole 2026 dollars, so 2048 and 2049 are $1 over.
    assert_eq!(paid(2047), 30_504);
    assert_eq!(paid(2048), 53_862);
    assert_eq!(paid(2049), 55_478);
}

#[test]
fn a_stated_benefit_under_a_rate_of_its_own_grows_at_it_from_the_plan_start() {
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
amount = 12000
cola = 0.03
start = { age = 67, owner = "me" }
"#,
    );
    let projection = run(&plan);
    let paid = |year| projection.row(year).unwrap().income["ss"];
    // A statement's figure is today's dollars: 12,000 at 3% for the 22 and
    // 23 years from 2026, not frozen until the claim; June on in 2047.
    assert_eq!(paid(2047), 13_022);
    assert_eq!(paid(2048), 22_993);
    assert_eq!(paid(2049), 23_683);
}
