//! Where each taxable dollar of a projected year is recorded, read through a
//! state that takes a tenth of everything and leaves one source untaxed.

mod common;

use retiretui_engine::params::Source;
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::{YearRow, project};

use common::{born_in, plan_from, tables_with};

const CASH: &str = r#"
[[residency]]
country = "us"
state = "ca"

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 500000
"#;

/// 2026 of `plan` in a state taking a tenth of everything but `exempt`,
/// with `keys` in its table.
fn first_year(plan: &str, exempt: Source, keys: &str) -> YearRow {
    let state = format!(
        "[states.ca]\n{keys}\n[states.ca.brackets]\nsingle = [{{ over = 0, rate = 0.1 }}]\nmarried-joint = [{{ over = 0, rate = 0.1 }}]\n\n[[states.ca.exclusions]]\nsources = [\"{}\"]\n",
        exempt.as_str()
    );
    let projection = project(&plan_from(plan), &tables_with(&state));
    projection.years[0].clone()
}

fn tenth(amount: Dollars) -> Dollars {
    (amount as f64 / 10.0).round() as Dollars
}

/// Asserts that what 2026 of `plan` taxes is all recorded as `recorded`:
/// the state takes nothing with that source exempt, and a tenth of all of
/// it with any other.
#[track_caller]
fn assert_recorded_as(plan: &str, recorded: Source) {
    for &exempt in Source::ALL {
        let row = first_year(plan, exempt, "");
        assert!(row.taxes.magi > 0, "{exempt:?} exempt: nothing is taxed");
        let owed = if exempt == recorded {
            0
        } else {
            tenth(row.taxes.magi)
        };
        assert_eq!(row.taxes.state, owed, "{exempt:?} exempt");
    }
}

fn receiving(kinds: &[&str]) -> String {
    let incomes = kinds.iter().map(|kind| {
        format!(
            "\n[[income]]\nid = \"{kind}\"\nkind = \"{kind}\"\nowner = \"me\"\namount = 30000\n"
        )
    });
    born_in(1976, &format!("{CASH}{}", incomes.collect::<String>()))
}

#[test]
fn a_salary_is_wages_a_pension_a_pension_and_every_other_income_other() {
    assert_recorded_as(&receiving(&["salary"]), Source::Wages);
    assert_recorded_as(&receiving(&["pension"]), Source::Pension);
    assert_recorded_as(&receiving(&["annuity"]), Source::Other);
    assert_recorded_as(&receiving(&["rental"]), Source::Other);
    assert_recorded_as(&receiving(&["other"]), Source::Other);
}

/// Someone born in June of `birth_year` holding `account` as `k`, with
/// `rest` beside it.
fn holding(birth_year: i16, account: &str, rest: &str) -> String {
    let body = format!(
        "{CASH}\n[[accounts]]\nid = \"k\"\nowner = \"me\"\nbalance = 400000\n{account}\n{rest}"
    );
    born_in(birth_year, &body)
}

const TO_A_BROKERAGE: &str = r#"
[[accounts]]
id = "brokerage"
kind = "brokerage"
owner = "me"
balance = 0

[[transfers]]
id = "cash-out"
from = "k"
to = "brokerage"
amount = 50000
on = { date = 2026-01-01 }
"#;

#[test]
fn a_transfer_out_of_a_deferred_account_is_a_distribution_early_where_penalized() {
    // Born June 1976: 50 in 2026.
    let early = holding(1976, "kind = \"401k\"", TO_A_BROKERAGE);
    assert_recorded_as(&early, Source::EarlyDistribution);
    let exempt_kind = holding(1976, "kind = \"457b\"", TO_A_BROKERAGE);
    assert_recorded_as(&exempt_kind, Source::Distribution);
    // Born June 1966: 60 in 2026.
    let of_age = holding(1966, "kind = \"401k\"", TO_A_BROKERAGE);
    assert_recorded_as(&of_age, Source::Distribution);
}

#[test]
fn a_required_distribution_is_a_distribution() {
    // Born June 1950: 76 in 2026.
    let required =
        holding(1950, "kind = \"ira\"", "").replace("horizon_age = 70", "horizon_age = 90");
    assert_recorded_as(&required, Source::Distribution);
}

#[test]
fn a_conversion_is_a_conversion() {
    let converting = r#"
[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "ladder"
from = "k"
to = "roth"
amount = 30000
on = { date = 2026-01-01 }
"#;
    assert_recorded_as(
        &holding(1976, "kind = \"ira\"", converting),
        Source::Conversion,
    );
}

/// Someone living on `k` alone.
fn living_on(birth_year: i16, account: &str) -> String {
    let spending = "\n[[expenses]]\nid = \"living\"\namount = 40000\n";
    holding(birth_year, account, spending).replace("balance = 500000", "balance = 0")
}

#[test]
fn a_draw_on_a_deferred_account_is_a_distribution_early_where_penalized() {
    assert_recorded_as(
        &living_on(1976, "kind = \"401k\""),
        Source::EarlyDistribution,
    );
    let freed = "kind = \"401k\"\nseparated = { date = 2025-08-01 }";
    // Born June 1970: 55 in 2025, the year the job is left.
    assert_recorded_as(&living_on(1970, freed), Source::Distribution);
    assert_recorded_as(&living_on(1966, "kind = \"401k\""), Source::Distribution);
}

#[test]
fn what_a_roth_account_earned_is_a_distribution_early_where_penalized() {
    let earned = "kind = \"401k\"\nroth = true\nbasis = 0";
    assert_recorded_as(&living_on(1976, earned), Source::EarlyDistribution);
    // Born June 1970: 56 in 2026, the job left the year before.
    let freed = format!("{earned}\nseparated = {{ date = 2025-08-01 }}");
    assert_recorded_as(&living_on(1970, &freed), Source::Distribution);
}

#[test]
fn what_an_hsa_pays_beyond_medical_spending_is_other_income() {
    assert_recorded_as(&living_on(1976, "kind = \"hsa\""), Source::Other);
    // Born June 1958: 68 in 2026, past the HSA's penalty.
    assert_recorded_as(&living_on(1958, "kind = \"hsa\""), Source::Other);
}

/// Someone on a 100,000 salary and a 20,000 pension who pays 4,000 a year
/// into `account`.
fn paying_into(account: &str) -> String {
    let body = format!(
        r#"{CASH}
[[income]]
id = "pay"
kind = "salary"
owner = "me"
amount = 100000

[[income]]
id = "pension"
kind = "pension"
owner = "me"
amount = 20000

[[accounts]]
id = "k"
owner = "me"
balance = 0
{account}

[[contributions]]
id = "paid-in"
to = "k"
amount = 4000
"#
    );
    born_in(1976, &body)
}

#[test]
fn what_is_paid_into_an_hsa_comes_off_wages() {
    let paying = paying_into("kind = \"hsa\"");
    // The pension alone is left with wages exempt, and wages less the 4,000
    // with the pension exempt.
    assert_eq!(first_year(&paying, Source::Wages, "").taxes.state, 2_000);
    assert_eq!(first_year(&paying, Source::Pension, "").taxes.state, 9_600);
    let taxing = "taxes-deferrals = true";
    assert_eq!(
        first_year(&paying, Source::Pension, taxing).taxes.state,
        9_600
    );
}

#[test]
fn what_is_deferred_is_a_source_of_its_own_that_a_state_may_tax() {
    for account in ["kind = \"401k\"", "kind = \"ira\""] {
        let paying = paying_into(account);
        // Wages exempt, the deferral still comes off the pension.
        let following = first_year(&paying, Source::Wages, "");
        assert_eq!(following.taxes.state, 1_600, "{account}");
        let following = first_year(&paying, Source::Pension, "");
        assert_eq!(following.taxes.state, 9_600, "{account}");
        let taxing = first_year(&paying, Source::Pension, "taxes-deferrals = true");
        assert_eq!(taxing.taxes.state, 10_000, "{account}");
        assert_eq!(taxing.taxes.magi, following.taxes.magi, "{account}");
    }
}

#[test]
fn an_ira_contribution_the_year_s_income_lets_be_deducted_is_taxed_with_deferrals() {
    // A covered worker on 60,000 a year, under the band the deduction is
    // lost over.
    let covered =
        paying_into("kind = \"ira\"").replace("amount = 100000", "amount = 60000\ncovered = true");
    let following = first_year(&covered, Source::Pension, "");
    assert_eq!(following.taxes.state, 5_600);
    let taxing = first_year(&covered, Source::Pension, "taxes-deferrals = true");
    assert_eq!(taxing.taxes.state, 6_000);
}
