//! Benefits checked to the dollar against SSA's Detailed Calculator 2026.2,
//! each worker earning the same in every year from 1990 to 2025.

mod common;

use std::collections::BTreeMap;

use jiff::civil::date;
use retiretui_engine::plan::{Dollars, PlanDate};
use retiretui_engine::tax;

/// The last year a published COLA carries the benefit to a 2026 claim.
const LAST_COLA_YEAR: i16 = 2025;

/// A worker born on a date, earning a yearly amount, claiming in a month,
/// and the monthly benefit SSA's calculator pays them.
type Case = ((i16, i8, i8), Dollars, (i16, i8), Dollars);

/// Checks each case's monthly benefit, figured as a projection figures it,
/// against SSA's.
fn assert_ssa(cases: &[Case]) {
    let params = common::benefit_params();
    for &((year, month, day), yearly, (claim_year, claim_month), ssa_monthly) in cases {
        let birth = PlanDate(date(year, month, day));
        let earnings: BTreeMap<i16, Dollars> = (1990..=2025).map(|year| (year, yearly)).collect();
        let eligibility = tax::benefit_birth_year(birth) + tax::EARLIEST_CLAIM_AGE;
        let colas: Vec<f64> = (eligibility..=LAST_COLA_YEAR)
            .map(|year| params.cola[&year])
            .collect();
        let age = tax::age_months(birth, tax::month_index(claim_year, claim_month));
        let annual = tax::social_security_benefit(&params, birth, age, &earnings, &colas);
        let monthly = annual / Dollars::from(tax::MONTHS_PER_YEAR);
        assert_eq!(monthly, ssa_monthly, "born {birth:?}");
    }
}

#[test]
fn each_cola_and_the_reduced_benefit_round_down_to_the_dime() {
    assert_ssa(&[
        // 2,210.60 × 1.028 = 2,272.4968: SSA's PIA is 2,272.40, not 2,272.50.
        ((1961, 6, 15), 32_000, (2026, 10), 2_019),
        // 1,282.10 × 0.95 = 1,217.995, down to 1,217.90.
        ((1959, 12, 20), 11_000, (2026, 1), 1_217),
    ]);
}

#[test]
fn earnings_are_capped_at_each_years_published_wage_base() {
    // 1990's 51,300 and 1991's 53,400 count in full; 2010, 2011 and 2016
    // count no more than the base held without a COLA.
    assert_ssa(&[((1962, 6, 15), 200_000, (2026, 10), 3_346)]);
}

#[test]
fn someone_born_on_new_years_day_is_figured_on_the_year_before() {
    assert_ssa(&[
        // First eligible in 2021, full retirement age 66 and 10 months.
        ((1960, 1, 1), 40_000, (2026, 1), 2_619),
        ((1960, 1, 2), 40_000, (2026, 1), 2_493),
        // First eligible in 2025: its bend points and December's COLA.
        ((1964, 1, 1), 40_000, (2026, 1), 1_945),
        ((1964, 1, 2), 40_000, (2026, 1), 1_971),
    ]);
}
