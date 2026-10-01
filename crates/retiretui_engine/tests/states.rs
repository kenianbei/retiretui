//! The modeled states that leave retirement income untaxed, each against
//! figures worked by hand from its 2026 table.

mod common;

use retiretui_engine::plan::Dollars;

use common::{living_in, run};

const SALARY: &str = r#"
[[income]]
id = "pay"
kind = "salary"
owner = "me"
amount = 100000
"#;

/// A pension of 20,000, a conversion of 30,000 out of an IRA, and 10,000
/// of rent.
const RETIRED: &str = r#"
[[income]]
id = "pension"
kind = "pension"
owner = "me"
amount = 20000

[[income]]
id = "rent"
kind = "rental"
owner = "me"
amount = 10000

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 400000

[[accounts]]
id = "roth"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[conversions]]
id = "ladder"
from = "ira"
to = "roth"
amount = 30000
on = { date = 2026-01-01 }
"#;

/// 50,000 moved in 2026 out of `owner`'s 401(k), whose table ends with
/// `stated`, into a brokerage of theirs.
fn cashing_out(owner: &str, stated: &str) -> String {
    format!(
        r#"
[[accounts]]
id = "k-{owner}"
kind = "401k"
owner = "{owner}"
balance = 400000
{stated}

[[accounts]]
id = "brokerage-{owner}"
kind = "brokerage"
owner = "{owner}"
balance = 0

[[transfers]]
id = "cash-out-{owner}"
from = "k-{owner}"
to = "brokerage-{owner}"
amount = 50000
on = {{ date = 2026-01-01 }}
"#
    )
}

/// What the state takes in 2026.
fn state_tax(text: &str) -> Dollars {
    run(text).years[0].taxes.state
}

#[test]
fn illinois_taxes_wages_less_the_exemption_and_no_retirement_income() {
    // 4.95% of 100,000 less 2,925.
    assert_eq!(state_tax(&living_in("il", 1976, SALARY)), 4_805);
    // Born June 1976: 50 in 2026. Only the rent is taxed: 4.95% of 10,000
    // less 2,925.
    let early = format!("{RETIRED}{}", cashing_out("me", ""));
    assert_eq!(state_tax(&living_in("il", 1976, &early)), 350);
}

#[test]
fn pennsylvania_taxes_what_is_deferred_and_not_what_an_hsa_is_paid() {
    let paying_in = format!(
        r#"{SALARY}
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 0

[[accounts]]
id = "hsa"
kind = "hsa"
owner = "me"
balance = 0

[[contributions]]
id = "deferred"
to = "k"
amount = 10000

[[contributions]]
id = "health"
to = "hsa"
amount = 4000
"#
    );
    // 3.07% of 100,000 less the 4,000.
    assert_eq!(state_tax(&living_in("pa", 1976, &paying_in)), 2_947);
}

#[test]
fn pennsylvania_taxes_a_distribution_until_the_year_of_59_and_a_half() {
    // 3.07% of the 50,000. Born June 1968: 58 in 2026.
    assert_eq!(
        state_tax(&living_in("pa", 1968, &cashing_out("me", ""))),
        1_535
    );
    let left = cashing_out("me", "separated = { date = 2025-08-01 }");
    assert_eq!(state_tax(&living_in("pa", 1968, &left)), 1_535);
    // Born June 1967: 59 and a half in December 2026.
    assert_eq!(state_tax(&living_in("pa", 1967, &cashing_out("me", ""))), 0);
}

#[test]
fn pennsylvania_leaves_a_pension_and_a_conversion_untaxed_at_any_age() {
    // Born June 1976: 50 in 2026. 3.07% of the 10,000 of rent.
    assert_eq!(state_tax(&living_in("pa", 1976, RETIRED)), 307);
}

#[test]
fn mississippi_taxes_an_early_distribution_and_no_other() {
    // Born June 1970: 56 in 2026. 4% of 50,000 less 8,300 and the 10,000
    // the schedule leaves untaxed.
    let early = cashing_out("me", "");
    assert_eq!(state_tax(&living_in("ms", 1970, &early)), 1_268);
    // The job left at 55, the year before.
    let left = cashing_out("me", "separated = { date = 2025-08-01 }");
    assert_eq!(state_tax(&living_in("ms", 1970, &left)), 0);
    // Born June 1976: 50 in 2026. The rent is under the deduction.
    assert_eq!(state_tax(&living_in("ms", 1976, RETIRED)), 0);
}

#[test]
fn mississippi_s_rate_steps_down_as_enacted_over_a_deduction_held_nominal() {
    // 58,300 a year in nominal dollars: 40,000 above the 8,300 and the
    // 10,000, whatever inflation does.
    let nominal = SALARY.replace("amount = 100000", "amount = 58300\ncola = false");
    let text = living_in("ms", 1980, &nominal).replace("inflation = 0.0", "inflation = 0.02");
    let projection = run(&text);
    let taken: Vec<Dollars> = (2026..=2032)
        .map(|year| projection.row(year).unwrap().taxes.state)
        .collect();
    assert_eq!(taken, [1_600, 1_500, 1_400, 1_300, 1_200, 1_200, 1_200]);
}

#[test]
fn iowa_taxes_retirement_income_until_the_year_its_owner_turns_55() {
    // Born June 1972: 54 in 2026. 3.8% of 50,000 less 16,100.
    let early = cashing_out("me", "");
    assert_eq!(state_tax(&living_in("ia", 1972, &early)), 1_288);
    assert_eq!(state_tax(&living_in("ia", 1971, &early)), 0);
    // 3.8% of 100,000 less 16,100.
    assert_eq!(state_tax(&living_in("ia", 1972, SALARY)), 3_188);
}

#[test]
fn iowa_exempts_each_spouse_by_their_own_age() {
    // Born 1971 and 1975: 55 and 51 in 2026. The younger's 50,000 is taxed:
    // 3.8% of it less 32,200.
    let body = format!("{}{}", cashing_out("me", ""), cashing_out("you", ""));
    let couple = living_in("ia", 1971, &body)
        .replace("filing = \"single\"", "filing = \"married-joint\"")
        .replace(
            "birth = 1971-06-15",
            "birth = 1971-06-15\n\n[[household.people]]\nid = \"you\"\nbirth = 1975-06-15",
        );
    assert_eq!(state_tax(&couple), 676);
}
