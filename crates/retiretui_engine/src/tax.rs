//! Federal tax formulas. The rule shapes live here; the yearly values come
//! from [`crate::params`].

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use crate::params::{BenefitParams, Bracket, StateParams, TaxParams};
use crate::plan::{AccountKind, Dollars, FilingStatus, Person, PlanDate};

/// The age Medicare coverage (and IRMAA exposure) begins.
pub const MEDICARE_AGE: u8 = 65;

/// The earliest age a retirement benefit can be claimed, and whose year's
/// bend points compute it.
pub const EARLIEST_CLAIM_AGE: i16 = 62;

/// The age whose year's average wage a worker's earnings are indexed to.
const INDEXING_AGE: i16 = 60;

/// The age a career filled at one salary is taken to begin, as SSA's own
/// illustrative earners do.
pub const FIRST_WORKING_AGE: i16 = 22;

/// The age past which delaying a claim earns nothing more.
pub const LATEST_CREDIT_AGE: i16 = 70;

/// Months in a year, which the benefit's monthly figures are counted in.
pub const MONTHS_PER_YEAR: i32 = 12;
/// The highest indexed years averaged into the AIME, and their months.
const COMPUTATION_YEARS: usize = 35;
const COMPUTATION_MONTHS: f64 = (COMPUTATION_YEARS as i32 * MONTHS_PER_YEAR) as f64;
/// The PIA formula's replacement rates, per bend-point band.
const PIA_RATES: [f64; 3] = [0.90, 0.32, 0.15];
/// Early claims lose 5/9 of 1% a month for 36 months, 5/12 of 1% beyond;
/// delayed claims earn 2/3 of 1% a month for those born 1943 and later.
const EARLY_FIRST_MONTHS: i32 = 36;
const EARLY_FIRST_RATE: f64 = 5.0 / 900.0;
const EARLY_BEYOND_RATE: f64 = 5.0 / 1200.0;
const DELAYED_RATE: f64 = 2.0 / 300.0;
/// Full retirement age rises two months a birth year across 1938-42 and
/// 1955-59, from 65 to 66 and from 66 to 67.
const FRA_65_LAST_BIRTH_YEAR: i16 = 1937;
const FRA_66_FIRST_BIRTH_YEAR: i16 = 1943;
const FRA_66_LAST_BIRTH_YEAR: i16 = 1954;
const FRA_67_FIRST_BIRTH_YEAR: i16 = 1960;
const FRA_STEP_MONTHS: i32 = 2;

/// Years between the MAGI that is measured and the premiums it prices.
pub const IRMAA_LOOKBACK_YEARS: i16 = 2;

/// Whether a person is Medicare-covered in `year`.
#[must_use]
pub fn is_medicare_covered(person: &Person, year: i16) -> bool {
    person.age_in_year(year) >= i16::from(MEDICARE_AGE)
}

/// Birth year from which SECURE 2.0 moves the RMD start to 75.
const RMD_AGE_75_BIRTH_YEAR: i16 = 1960;
const RMD_START_AGE_EARLY: u8 = 73;
const RMD_START_AGE_LATE: u8 = 75;

/// Tax on ordinary taxable income (after deductions) via the bracket walk.
#[must_use]
pub fn ordinary_tax(params: &TaxParams, status: FilingStatus, taxable: Dollars) -> Dollars {
    walk_brackets(params.brackets.for_status(status), taxable)
}

/// A state's income tax: ordinary income and gains alike, with the taxable
/// share of Social Security where the state taxes it, less its deduction.
#[must_use]
pub fn state_tax(
    state: &StateParams,
    status: FilingStatus,
    income: Dollars,
    taxable_social_security: Dollars,
) -> Dollars {
    let benefits = if state.taxes_social_security {
        taxable_social_security
    } else {
        0
    };
    let taxable = income + benefits - state.deduction.get(status);
    walk_brackets(state.brackets.for_status(status), taxable)
}

fn walk_brackets(brackets: &[Bracket], taxable: Dollars) -> Dollars {
    let mut tax = 0.0;
    for (i, bracket) in brackets.iter().enumerate() {
        if taxable <= bracket.over {
            break;
        }
        let top = brackets
            .get(i + 1)
            .map_or(taxable, |next| taxable.min(next.over));
        tax += bracket.rate * (top - bracket.over) as f64;
    }
    tax.round() as Dollars
}

/// Tax on long-term gains stacked on top of ordinary taxable income.
#[must_use]
pub fn ltcg_tax(
    params: &TaxParams,
    status: FilingStatus,
    ordinary_taxable: Dollars,
    gains: Dollars,
) -> Dollars {
    if gains <= 0 {
        return 0;
    }
    let stack_bottom = ordinary_taxable.max(0);
    let zero_top = params.ltcg.zero_until.get(status);
    let fifteen_top = params.ltcg.fifteen_until.get(status);
    let in_zero = (zero_top - stack_bottom).clamp(0, gains);
    let above_zero = gains - in_zero;
    let in_middle = (fifteen_top - stack_bottom.max(zero_top)).clamp(0, above_zero);
    let in_top = above_zero - in_middle;
    let tax = params.ltcg.middle_rate * in_middle as f64 + params.ltcg.top_rate * in_top as f64;
    tax.round() as Dollars
}

/// The taxable portion of a Social Security benefit, from provisional income
/// (`other_income` plus half the benefit) against the statutory thresholds.
#[must_use]
pub fn taxable_social_security(
    params: &TaxParams,
    status: FilingStatus,
    other_income: Dollars,
    benefit: Dollars,
) -> Dollars {
    if benefit <= 0 {
        return 0;
    }
    let provisional = other_income + benefit / 2;
    let base = params.social_security.provisional_base.get(status);
    let upper = params.social_security.provisional_upper.get(status);
    if provisional <= base {
        return 0;
    }
    if provisional <= upper {
        return ((provisional - base) / 2).min(benefit / 2);
    }
    let from_lower_band = ((upper - base) / 2).min(benefit / 2);
    let above_upper = (0.85 * (provisional - upper) as f64).round() as Dollars;
    (above_upper + from_lower_band).min((0.85 * benefit as f64).round() as Dollars)
}

/// Full retirement age in months, from the birth year.
#[must_use]
pub fn full_retirement_months(birth_year: i16) -> i32 {
    let steps = |from: i16| FRA_STEP_MONTHS * i32::from(birth_year - from);
    if birth_year <= FRA_65_LAST_BIRTH_YEAR {
        65 * MONTHS_PER_YEAR
    } else if birth_year < FRA_66_FIRST_BIRTH_YEAR {
        65 * MONTHS_PER_YEAR + steps(FRA_65_LAST_BIRTH_YEAR)
    } else if birth_year <= FRA_66_LAST_BIRTH_YEAR {
        66 * MONTHS_PER_YEAR
    } else if birth_year < FRA_67_FIRST_BIRTH_YEAR {
        66 * MONTHS_PER_YEAR + steps(FRA_66_LAST_BIRTH_YEAR)
    } else {
        67 * MONTHS_PER_YEAR
    }
}

/// The annual retirement benefit for a claim at `claim_age` whole years,
/// in the dollars of the year the worker turns 62 and before the COLAs
/// that run from it: each year's covered earnings capped at its own wage
/// base and indexed to the average wage of the year they turn 60, the
/// highest 35 years averaged monthly, the PIA through the bend points of
/// the eligibility year truncated to the dime, then reduced or credited
/// month by month against full retirement age and truncated to the dollar.
/// Claims past 70 earn 70's credit.
#[must_use]
pub fn social_security_benefit(
    params: &BenefitParams,
    birth_year: i16,
    claim_age: i16,
    earnings: &BTreeMap<i16, Dollars>,
) -> Dollars {
    let pia = primary_insurance_amount(params, birth_year, earnings);
    let monthly = (pia * claim_factor(birth_year, claim_age) * 100.0).round() / 100.0;
    monthly.floor() as Dollars * MONTHS_PER_YEAR as Dollars
}

/// A career at one real wage, as SSA's Quick Calculator fills a record
/// from current earnings: `salary`, in the dollars of `paid_in`, scaled to
/// each of `years` by that year's average wage over `paid_in`'s, so every
/// year indexes back to the salary; a year the index does not carry is
/// left out.
#[must_use]
pub fn earnings_at_wage(
    params: &BenefitParams,
    salary: Dollars,
    paid_in: i16,
    years: RangeInclusive<i16>,
) -> BTreeMap<i16, Dollars> {
    years
        .filter_map(|year| {
            let scaled = salary as f64 * params.wage_ratio(year, paid_in)?;
            Some((year, scaled.round() as Dollars))
        })
        .collect()
}

/// The share of the year a worker turns `claim_age` that a claim at that
/// age is paid for: the months from the one the age is attained in, out
/// of twelve. An age is attained the day before the birthday, except that
/// 62 must be attained by the month's first day; nothing when that falls
/// in the next year.
#[must_use]
pub fn claim_year_share(birth: PlanDate, claim_age: i16) -> f64 {
    let first_paid = i16::from(birth.0.month())
        + match (claim_age == EARLIEST_CLAIM_AGE, birth.0.day()) {
            (true, day) if day > 2 => 1,
            (false, 1) => -1,
            _ => 0,
        };
    let months = (MONTHS_PER_YEAR as i16 + 1 - first_paid).clamp(0, MONTHS_PER_YEAR as i16);
    f64::from(months) / f64::from(MONTHS_PER_YEAR)
}

fn primary_insurance_amount(
    params: &BenefitParams,
    birth_year: i16,
    earnings: &BTreeMap<i16, Dollars>,
) -> f64 {
    let index_year = birth_year + INDEXING_AGE;
    let to_index_year = |year| {
        if year >= index_year {
            1.0
        } else {
            params.wage_ratio(index_year, year).unwrap_or(1.0)
        }
    };
    let mut indexed: Vec<f64> = earnings
        .iter()
        .map(|(&year, &amount)| amount.min(params.wage_base(year)) as f64 * to_index_year(year))
        .collect();
    indexed.sort_by(|a, b| b.total_cmp(a));
    let total: f64 = indexed.iter().take(COMPUTATION_YEARS).sum();
    let aime = (total / COMPUTATION_MONTHS).floor();
    let eligibility_year = birth_year + EARLIEST_CLAIM_AGE;
    let [first, second] = params
        .bend_points(eligibility_year)
        .map(|point| point as f64);
    let pia = PIA_RATES[0] * aime.min(first)
        + PIA_RATES[1] * (aime.min(second) - first).max(0.0)
        + PIA_RATES[2] * (aime - second).max(0.0);
    ((pia * 100.0).round() / 10.0).floor() / 10.0
}

fn claim_factor(birth_year: i16, claim_age: i16) -> f64 {
    let full = full_retirement_months(birth_year);
    let claim = i32::from(claim_age.min(LATEST_CREDIT_AGE)) * MONTHS_PER_YEAR;
    if claim >= full {
        return 1.0 + f64::from(claim - full) * DELAYED_RATE;
    }
    let early = full - claim;
    let first = early.min(EARLY_FIRST_MONTHS);
    1.0 - f64::from(first) * EARLY_FIRST_RATE - f64::from(early - first) * EARLY_BEYOND_RATE
}

/// The annual IRMAA surcharge per covered person for a lookback MAGI.
/// Tiers are cliffs: one dollar over a threshold buys the whole tier.
#[must_use]
pub fn irmaa_surcharge(
    params: &TaxParams,
    status: FilingStatus,
    magi: Dollars,
    part_d: bool,
) -> Dollars {
    params
        .irmaa
        .iter()
        .rev()
        .find(|tier| magi > tier.magi_over.get(status))
        .map_or(0, |tier| tier.part_b + if part_d { tier.part_d } else { 0 })
}

/// The MAGI at which IRMAA tier `tier` (zero-indexed) begins - the ceiling
/// that keeps a household below it; `None` past the table.
#[must_use]
pub fn irmaa_threshold(params: &TaxParams, status: FilingStatus, tier: u8) -> Option<Dollars> {
    params
        .irmaa
        .get(usize::from(tier))
        .map(|entry| entry.magi_over.get(status))
}

/// The required minimum distribution for the year a person reaches `age`,
/// from the prior year-end `balance`. Zero below the table's first age; ages
/// past the table's last row keep its final divisor.
#[must_use]
pub fn rmd(params: &TaxParams, age: u8, balance: Dollars) -> Dollars {
    if balance <= 0 {
        return 0;
    }
    let divisors = &params.rmd.divisors;
    let Some(first) = divisors.first() else {
        return 0;
    };
    if age < first.age {
        return 0;
    }
    let divisor = divisors
        .iter()
        .rev()
        .find(|row| row.age <= age)
        .map_or(first.divisor, |row| row.divisor);
    (balance as f64 / divisor).round() as Dollars
}

/// The age a person's RMDs begin under SECURE 2.0, from their birth year.
#[must_use]
pub fn rmd_start_age(birth_year: i16) -> u8 {
    if birth_year >= RMD_AGE_75_BIRTH_YEAR {
        RMD_START_AGE_LATE
    } else {
        RMD_START_AGE_EARLY
    }
}

/// Whether early withdrawals from this wrapper escape the penalty
/// (governmental 457(b) distributions are penalty-free at any age).
#[must_use]
pub fn is_penalty_exempt(kind: AccountKind) -> bool {
    matches!(kind, AccountKind::K457b)
}

/// The accounts of one person that share one employee limit for the year.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum LimitPool {
    /// 401(k) and 403(b), one elective limit between them.
    EmployerPlan,
    /// A 457(b), limited on its own.
    Deferred457,
    /// A SIMPLE IRA, limited on its own.
    Simple,
    /// Traditional and Roth IRAs together.
    Ira,
    /// An HSA, whose employer contributions count too.
    Hsa,
}

/// The pool the kind's employee contributions count against, or `None`
/// where the law limits nothing.
#[must_use]
pub fn limit_pool(kind: AccountKind) -> Option<LimitPool> {
    match kind {
        AccountKind::K401k | AccountKind::K403b => Some(LimitPool::EmployerPlan),
        AccountKind::K457b => Some(LimitPool::Deferred457),
        AccountKind::SimpleIra => Some(LimitPool::Simple),
        AccountKind::Ira => Some(LimitPool::Ira),
        AccountKind::Hsa => Some(LimitPool::Hsa),
        AccountKind::K414k | AccountKind::SepIra | AccountKind::Brokerage | AccountKind::Cash => {
            None
        }
    }
}

/// The employee contribution limit for an account kind at an age (the age
/// reached during the year), or `None` when no employee limit applies.
/// HSA family coverage is assumed for a married household.
#[must_use]
pub fn employee_limit(
    params: &TaxParams,
    status: FilingStatus,
    kind: AccountKind,
    age: u8,
) -> Option<Dollars> {
    let limits = &params.limits;
    let plan_catch_up = |base_50: Dollars, base_60: Dollars| match age {
        60..=63 => base_60,
        50.. => base_50,
        _ => 0,
    };
    let limit = match limit_pool(kind)? {
        LimitPool::EmployerPlan | LimitPool::Deferred457 => {
            limits.employer_plan
                + plan_catch_up(
                    limits.employer_plan_catch_up_50,
                    limits.employer_plan_catch_up_60,
                )
        }
        LimitPool::Simple => {
            limits.simple + plan_catch_up(limits.simple_catch_up_50, limits.simple_catch_up_60)
        }
        LimitPool::Ira => limits.ira + if age >= 50 { limits.ira_catch_up_50 } else { 0 },
        LimitPool::Hsa => {
            let coverage = match status {
                FilingStatus::Single => limits.hsa_self,
                FilingStatus::MarriedJoint => limits.hsa_family,
            };
            coverage + if age >= 55 { limits.hsa_catch_up_55 } else { 0 }
        }
    };
    Some(limit)
}
