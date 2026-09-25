//! Federal tax formula tests, hand-checked against published 2026 figures.

use std::path::Path;

use retiretui_engine::params::{Inflation, TaxTables};
use retiretui_engine::plan::{AccountKind, FilingStatus};
use retiretui_engine::tax;

const INFLATION: f64 = 0.02;

fn params_2026() -> retiretui_engine::params::TaxParams {
    TaxTables::embedded().params_for(2026, &Inflation::constant(INFLATION))
}

#[test]
fn embedded_tables_load() {
    let tables = TaxTables::embedded();
    assert_eq!(tables.latest_known_year(), Some(2026));
    let params = tables.params_for(2026, &Inflation::constant(INFLATION));
    assert_eq!(params.year, 2026);
    assert_eq!(params.deductions.standard.single, 16100);
    assert_eq!(params.deductions.standard.married_joint, 32200);
}

#[test]
fn ordinary_tax_matches_hand_computation() {
    let params = params_2026();
    // Single, 100,000 taxable: 1,240 + 4,560 + 10,912.
    assert_eq!(
        tax::ordinary_tax(&params, FilingStatus::Single, 100_000),
        16_712
    );
    // MFJ, 200,000 taxable: 2,480 + 9,120 + 21,824.
    assert_eq!(
        tax::ordinary_tax(&params, FilingStatus::MarriedJoint, 200_000),
        33_424
    );
    assert_eq!(tax::ordinary_tax(&params, FilingStatus::Single, 0), 0);
    assert_eq!(tax::ordinary_tax(&params, FilingStatus::Single, -5_000), 0);
}

#[test]
fn ltcg_stacks_on_ordinary_income() {
    let params = params_2026();
    // MFJ: 48,900 of the gains fill the zero band, 11,100 at 15%.
    assert_eq!(
        tax::ltcg_tax(&params, FilingStatus::MarriedJoint, 50_000, 60_000),
        1_665
    );
    // Single straddling all three bands:
    // 9,450 at 0%, 496,050 at 15%, 94,500 at 20%.
    assert_eq!(
        tax::ltcg_tax(&params, FilingStatus::Single, 40_000, 600_000),
        93_308
    );
    assert_eq!(tax::ltcg_tax(&params, FilingStatus::Single, 40_000, 0), 0);
}

#[test]
fn social_security_taxation_regions() {
    let params = params_2026();
    // Below the base threshold: nothing taxable.
    assert_eq!(
        tax::taxable_social_security(&params, FilingStatus::Single, 10_000, 20_000),
        0
    );
    // Between thresholds: half the excess over the base.
    assert_eq!(
        tax::taxable_social_security(&params, FilingStatus::Single, 20_000, 20_000),
        2_500
    );
    // Above the upper threshold, capped at 85% of the benefit.
    assert_eq!(
        tax::taxable_social_security(&params, FilingStatus::Single, 40_000, 20_000),
        17_000
    );
}

#[test]
fn rmd_uses_the_uniform_lifetime_table() {
    let params = params_2026();
    assert_eq!(tax::rmd(&params, 73, 265_000), 10_000);
    assert_eq!(tax::rmd(&params, 71, 265_000), 0);
    assert_eq!(tax::rmd(&params, 130, 20_000), 10_000);
    assert_eq!(tax::rmd(&params, 80, 0), 0);
}

#[test]
fn rmd_start_age_follows_birth_year() {
    assert_eq!(tax::rmd_start_age(1955), 73);
    assert_eq!(tax::rmd_start_age(1959), 73);
    assert_eq!(tax::rmd_start_age(1960), 75);
    assert_eq!(tax::rmd_start_age(1980), 75);
}

#[test]
fn employee_limits_by_kind_and_age() {
    let params = params_2026();
    let limit = |kind, age| tax::employee_limit(&params, FilingStatus::MarriedJoint, kind, age);
    assert_eq!(limit(AccountKind::K401k, 49), Some(24_500));
    assert_eq!(limit(AccountKind::K401k, 52), Some(32_500));
    assert_eq!(limit(AccountKind::K401k, 61), Some(35_750));
    assert_eq!(limit(AccountKind::K401k, 64), Some(32_500));
    assert_eq!(limit(AccountKind::Ira, 52), Some(8_600));
    assert_eq!(limit(AccountKind::SimpleIra, 61), Some(22_250));
    assert_eq!(limit(AccountKind::Hsa, 56), Some(9_750));
    assert_eq!(
        tax::employee_limit(&params, FilingStatus::Single, AccountKind::Hsa, 45),
        Some(4_400)
    );
    assert_eq!(limit(AccountKind::K414k, 40), None);
    assert_eq!(limit(AccountKind::Brokerage, 40), None);
}

#[test]
fn penalty_exemption_covers_457b() {
    assert!(tax::is_penalty_exempt(AccountKind::K457b));
    assert!(!tax::is_penalty_exempt(AccountKind::K401k));
    assert!(!tax::is_penalty_exempt(AccountKind::Ira));
}

#[test]
fn future_years_inflate_indexed_values_only() {
    let tables = TaxTables::embedded();
    let params = tables.params_for(2036, &Inflation::constant(INFLATION));
    assert_eq!(params.year, 2036);
    let factor = 1.02f64.powi(10);
    let expected = (16_100.0 * factor).round() as i64;
    assert_eq!(params.deductions.standard.single, expected);
    // Statutory Social Security thresholds never move.
    assert_eq!(params.social_security.provisional_base.single, 25_000);
    // Divisors never move.
    assert!((params.rmd.divisors[1].divisor - 26.5).abs() < f64::EPSILON);
}

#[test]
fn override_directory_replaces_and_extends_years() {
    let mut tables = TaxTables::embedded();
    tables
        .add_dir(Path::new("tests/fixtures/tax-override"))
        .unwrap();
    assert_eq!(tables.latest_known_year(), Some(2027));
    let params = tables.params_for(2027, &Inflation::constant(INFLATION));
    assert_eq!(params.deductions.standard.single, 16_500);
    // 2026 still answers from the embedded table.
    let params = tables.params_for(2026, &Inflation::constant(INFLATION));
    assert_eq!(params.deductions.standard.single, 16_100);
    // A missing directory adds nothing and is not an error.
    tables
        .add_dir(Path::new("tests/fixtures/no-such-dir"))
        .unwrap();
}

#[test]
fn irmaa_tiers_are_cliffs() {
    let params = params_2026();
    let single = FilingStatus::Single;
    assert_eq!(tax::irmaa_surcharge(&params, single, 109_000, true), 0);
    assert_eq!(
        tax::irmaa_surcharge(&params, single, 109_001, true),
        888 + 168,
        "one dollar over buys the whole first tier"
    );
    assert_eq!(
        tax::irmaa_surcharge(&params, single, 109_001, false),
        888,
        "part D can be excluded"
    );
    assert_eq!(
        tax::irmaa_surcharge(&params, single, 600_000, true),
        5_328 + 1_056,
        "the top tier is open-ended"
    );
    assert_eq!(
        tax::irmaa_surcharge(&params, FilingStatus::MarriedJoint, 218_001, true),
        888 + 168,
        "married thresholds differ"
    );
}

#[test]
fn irmaa_extends_with_inflation() {
    let tables = TaxTables::embedded();
    let extended = tables.params_for(2046, &Inflation::constant(INFLATION));
    let base = tables.params_for(2026, &Inflation::constant(INFLATION));
    assert!(extended.irmaa[0].magi_over.single > base.irmaa[0].magi_over.single);
    assert!(extended.irmaa[0].part_b > base.irmaa[0].part_b);
    assert_eq!(extended.irmaa.len(), base.irmaa.len());
}

#[test]
fn a_state_taxes_income_less_its_deduction_through_its_own_brackets() {
    let params = params_2026();
    let oregon = &params.states["or"];
    // Publication OR-ESTIMATE's own example: 68,000 taxable, jointly.
    assert_eq!(
        tax::state_tax(oregon, FilingStatus::MarriedJoint, 68_000 + 5_800, 0),
        5_312
    );
    // Single, 50,000 taxable: 216 + 462 + 3,377.50.
    assert_eq!(
        tax::state_tax(oregon, FilingStatus::Single, 50_000 + 2_900, 0),
        4_056
    );
    assert_eq!(tax::state_tax(oregon, FilingStatus::Single, 1_000, 0), 0);
}

#[test]
fn a_state_taxes_social_security_only_where_it_says_so() {
    let params = params_2026();
    let mut state = params.states["or"].clone();
    let exempt = tax::state_tax(&state, FilingStatus::Single, 40_000, 20_000);
    assert_eq!(
        exempt,
        tax::state_tax(&state, FilingStatus::Single, 40_000, 0)
    );
    state.taxes_social_security = true;
    assert_eq!(
        tax::state_tax(&state, FilingStatus::Single, 40_000, 20_000),
        tax::state_tax(&state, FilingStatus::Single, 60_000, 0)
    );
}

#[test]
fn a_state_with_no_income_tax_is_a_table_and_one_not_modeled_is_none() {
    let params = params_2026();
    for code in ["ak", "fl", "nv", "nh", "sd", "tn", "tx", "wa", "wy"] {
        let tax = tax::state_tax(&params.states[code], FilingStatus::Single, 500_000, 50_000);
        assert_eq!(tax, 0, "{code}");
    }
    assert!(!params.states.contains_key("ca"));
}

#[test]
fn state_tables_extend_with_inflation() {
    let later = TaxTables::embedded().params_for(2036, &Inflation::constant(INFLATION));
    let oregon = &later.states["or"];
    assert_eq!(oregon.deduction.single, 3_535);
    assert_eq!(oregon.brackets.single[1].over, 5_546);
    assert!((oregon.brackets.single[1].rate - 0.0675).abs() < f64::EPSILON);
    assert_eq!(oregon.brackets.single[3].over, 125_000, "set by statute");
}

/// SSA's "case A" for workers retiring in 2026: born 1964, these nominal
/// earnings 1986-2025, AIME 5,825, PIA 2,609.80, and 1,826 a month at 62.
const CASE_A_EARNINGS: [(i16, i64); 40] = [
    (1986, 16_196),
    (1987, 17_283),
    (1988, 18_191),
    (1989, 18_971),
    (1990, 19_909),
    (1991, 20_715),
    (1992, 21_850),
    (1993, 22_107),
    (1994, 22_770),
    (1995, 23_755),
    (1996, 24_994),
    (1997, 26_533),
    (1998, 28_007),
    (1999, 29_657),
    (2000, 31_392),
    (2001, 32_238),
    (2002, 32_660),
    (2003, 33_558),
    (2004, 35_224),
    (2005, 36_621),
    (2006, 38_419),
    (2007, 40_281),
    (2008, 41_330),
    (2009, 40_826),
    (2010, 41_914),
    (2011, 43_354),
    (2012, 44_839),
    (2013, 45_544),
    (2014, 47_298),
    (2015, 49_085),
    (2016, 49_783),
    (2017, 51_651),
    (2018, 53_677),
    (2019, 55_848),
    (2020, 57_590),
    (2021, 62_889),
    (2022, 66_421),
    (2023, 69_560),
    (2024, 73_133),
    (2025, 75_868),
];

fn benefit_params() -> retiretui_engine::params::BenefitParams {
    params_2026().social_security.benefit.unwrap()
}

#[test]
fn social_security_benefit_matches_ssa_case_a() {
    let params = benefit_params();
    let earnings = std::collections::BTreeMap::from(CASE_A_EARNINGS);
    let at = |age| tax::social_security_benefit(&params, 1964, age, &earnings);
    // 60 months early: 20% for the first 36, 10% for the rest of 2,609.80.
    assert_eq!(at(62), 1_826 * 12);
    assert_eq!(at(67), 2_609 * 12);
    // 36 months late: 24% more; nothing past 70.
    assert_eq!(at(70), 3_236 * 12);
    assert_eq!(at(75), at(70));
}

#[test]
fn social_security_benefit_caps_and_pads_the_record() {
    let params = benefit_params();
    let one_year = |amount| {
        let earnings = std::collections::BTreeMap::from([(2026, amount)]);
        tax::social_security_benefit(&params, 1964, 67, &earnings)
    };
    // One year of 2026's 184,500 over 420 months: AIME 439, all in the 90%
    // band.
    assert_eq!(one_year(184_500), 395 * 12);
    assert_eq!(one_year(1_000_000), one_year(184_500));
    let empty = std::collections::BTreeMap::new();
    assert_eq!(tax::social_security_benefit(&params, 1964, 67, &empty), 0);
}

#[test]
fn benefit_params_derive_the_published_amounts_from_the_index() {
    let params = benefit_params();
    assert_eq!(params.wage_index.keys().next_back(), Some(&2024));
    assert_eq!(params.bend_points(2026), [1_286, 7_749]);
    assert_eq!(params.bend_points(2022), [1_024, 6_172]);
    assert_eq!(params.wage_base(2024), 168_600);
    assert_eq!(params.wage_base(2025), 176_100);
    assert_eq!(params.wage_base(2026), 184_500);
    // 69,846.57 grown thirteen years at 3.6%.
    let grown = params.average_wage(2037).unwrap();
    assert!(
        (grown - 69_846.57 * 1.036_f64.powi(13)).abs() < 0.01,
        "{grown}"
    );
    assert_eq!(params.average_wage(1950), None);
    let mut faster = params.clone();
    faster.wage_growth = 0.05;
    assert!(faster.average_wage(2037) > params.average_wage(2037));
}

#[test]
fn earnings_index_to_the_year_the_worker_turns_sixty() {
    // Born 1977: indexed to 2037's grown wage and bent at 2039's points, a
    // 2024 dollar counts for more than it does for someone born 1964,
    // whose age-60 year is the published 2024.
    let params = benefit_params();
    let record = (1999..=2025).map(|year| (year, 50_000)).collect();
    let born_1977 = tax::social_security_benefit(&params, 1977, 67, &record);
    let born_1964 = tax::social_security_benefit(&params, 1964, 67, &record);
    assert!(
        born_1977 > born_1964 * 13 / 10,
        "{born_1977} vs {born_1964}"
    );
}

#[test]
fn earnings_at_wage_index_back_to_the_salary() {
    let params = benefit_params();
    let record = tax::earnings_at_wage(&params, 60_000, 2024, 1940..=2030);
    // The index runs from 1951; past 2024 the salary grows with the wage.
    assert_eq!(record.keys().next(), Some(&1951));
    assert_eq!(record.keys().next_back(), Some(&2030));
    assert_eq!(record[&2024], 60_000);
    assert_eq!(record[&2030], 74_184);
    // 60,000 x 66,621.80 / 69,846.57
    assert_eq!(record[&2023], 57_230);
    let career = tax::earnings_at_wage(&params, 60_000, 2024, 1990..=2024);
    let flat = (1990..=2024).map(|year| (year, 60_000)).collect();
    let scaled = tax::social_security_benefit(&params, 1964, 67, &career);
    let at_face = tax::social_security_benefit(&params, 1964, 67, &flat);
    assert!(
        scaled < at_face,
        "{scaled} vs {at_face}: 1990's 60,000 indexed"
    );
    assert!(scaled > 0);
}

#[test]
fn claim_year_share_follows_the_ssa_calendar() {
    let born = |month, day| retiretui_engine::plan::PlanDate(jiff::civil::date(1980, month, day));
    let paid = |birth, age, months: u8| {
        let share = tax::claim_year_share(birth, age);
        assert!(
            (share - f64::from(months) / 12.0).abs() < f64::EPSILON,
            "{share} for {months} months"
        );
    };
    // Attained the day before the birthday, paid from that month.
    paid(born(6, 14), 67, 7);
    paid(born(12, 20), 70, 1);
    paid(born(2, 1), 70, 12);
    // 62 must be attained by the first of the month.
    paid(born(6, 2), 62, 7);
    paid(born(6, 3), 62, 6);
    paid(born(12, 3), 62, 0);
}

#[test]
fn full_retirement_age_steps_by_birth_year() {
    assert_eq!(tax::full_retirement_months(1937), 65 * 12);
    assert_eq!(tax::full_retirement_months(1940), 65 * 12 + 6);
    assert_eq!(tax::full_retirement_months(1943), 66 * 12);
    assert_eq!(tax::full_retirement_months(1954), 66 * 12);
    assert_eq!(tax::full_retirement_months(1957), 66 * 12 + 6);
    assert_eq!(tax::full_retirement_months(1959), 66 * 12 + 10);
    assert_eq!(tax::full_retirement_months(1960), 67 * 12);
    assert_eq!(tax::full_retirement_months(1990), 67 * 12);
}
