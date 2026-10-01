//! What the modeled states adjust their tax by, each against figures worked
//! by hand from its 2026 table.

mod common;

use retiretui_engine::plan::{Dollars, FilingStatus};
use retiretui_engine::tax::{self, StateIncome};

use common::{cashing_out, in_year, living_in, params_2026, person, run, salary};

/// What `state` takes in 2026 of someone born in June of `birth_year`, with
/// `body`.
fn taken(state: &str, birth_year: i16, body: &str) -> Dollars {
    run(&living_in(state, birth_year, body)).years[0]
        .taxes
        .state
}

#[test]
fn oregon_subtracts_federal_tax_up_to_its_cap_and_credits_each_person() {
    // 80,000: 8,770 of federal tax, 8,750 of it subtracted with the 2,910.
    // 216 + 462 + 8.75% of 56,940, less the credit of 263.
    assert_eq!(taken("or", 1980, &salary(80_000)), 5_398);
    // 60,000: all 5,020 of federal tax is subtracted. 216 + 462 + 8.75% of
    // 40,670, less the credit.
    assert_eq!(taken("or", 1980, &salary(60_000)), 3_974);
}

#[test]
fn oregon_s_credit_is_lost_over_100_000_and_its_cap_steps_down_from_125_000() {
    // 100,001: 8.75% of 76,941 over the 678.50 below it, and no credit.
    assert_eq!(taken("or", 1980, &salary(100_000)), 7_148);
    assert_eq!(taken("or", 1980, &salary(100_001)), 7_411);
    // 130,000: two steps down, 5,250 subtracted. 8.75% of 110,440 over the
    // 678.50.
    assert_eq!(taken("or", 1980, &salary(130_000)), 10_342);
    // 145,000: nothing subtracted. 10,618.50 to 125,000, and 9.9% of 17,090.
    assert_eq!(taken("or", 1980, &salary(145_000)), 12_310);
}

#[test]
fn oregon_counts_the_early_withdrawal_penalty_as_federal_tax() {
    // Born June 1980: 46 in 2026. 50,000 out of a 401(k) owes 3,820 of
    // federal tax and 5,000 of penalty, 8,750 of which is subtracted:
    // 216 + 462 + 8.75% of 26,940, less the credit.
    assert_eq!(taken("or", 1980, &cashing_out("me", "")), 2_773);
}

#[test]
fn oregon_illinois_and_mississippi_add_to_the_deduction_from_65() {
    // Born June 1961: 65 in 2026. Oregon deducts 1,200 more: 8.75% of it.
    assert_eq!(taken("or", 1961, &salary(80_000)), 5_398 - 105);
    // Illinois: 4.95% of 100,000 less 2,925 and 1,000.
    assert_eq!(taken("il", 1961, &salary(100_000)), 4_756);
    // Mississippi: 4% of 100,000 less 8,300 and the untaxed 10,000, then
    // less 1,500 more.
    assert_eq!(taken("ms", 1962, &salary(100_000)), 3_268);
    assert_eq!(taken("ms", 1961, &salary(100_000)), 3_208);
}

#[test]
fn illinois_gives_no_exemption_over_250_000() {
    // 4.95% of 250,000 less 2,925, and of 250,001.
    assert_eq!(taken("il", 1980, &salary(250_000)), 12_230);
    assert_eq!(taken("il", 1980, &salary(250_001)), 12_375);
}

#[test]
fn iowa_credits_each_person_and_more_from_65_and_refunds_nothing() {
    // 3.8% of 100,000 less 16,100, less 40 and 20 more at 65.
    assert_eq!(taken("ia", 1961, &salary(100_000)), 3_128);
    // 3.8% of the 400 over the deduction is 15, under the credit.
    assert_eq!(taken("ia", 1980, &salary(16_500)), 0);
}

/// 600,000 in a brokerage that has gained all of it, drawn whole in 2026.
const GAINS: &str = r#"
[[accounts]]
id = "brokerage"
kind = "brokerage"
owner = "me"
balance = 600000
basis = 0
drain_priority = 1

[[expenses]]
id = "house"
amount = 700000
"#;

#[test]
fn washington_taxes_gains_over_its_deduction_and_nothing_else() {
    let row = &run(&living_in("wa", 1970, GAINS)).years[0];
    assert_eq!(row.taxes.gains, 600_000);
    // 7% of 600,000 less 290,000.
    assert_eq!(row.taxes.state, 21_700);
    assert_eq!(taken("wa", 1970, &cashing_out("me", "")), 0);
    assert_eq!(taken("wa", 1970, &salary(900_000)), 0);
}

#[test]
fn washington_takes_9_9_percent_of_taxed_gains_over_a_million() {
    let people = [person(1970, 6, &[])];
    let income = StateIncome {
        gains: 1_500_000,
        ..in_year(2026, &people)
    };
    let washington = &params_2026().states["wa"];
    // 70,000 on the first million taxed, and 9.9% of the 210,000 beyond.
    assert_eq!(
        tax::state_tax(washington, FilingStatus::Single, &income),
        90_790
    );
}
