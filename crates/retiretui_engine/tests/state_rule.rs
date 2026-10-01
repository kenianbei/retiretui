//! The state income tax rule: what a state's table can say of whose income
//! is taxed and from where, over tables made here and the embedded Oregon.

mod common;

use retiretui_engine::params::{Inflation, Source, StateParams, TaxParams, TaxTables};
use retiretui_engine::plan::{Dollars, FilingStatus, PlanDate};
use retiretui_engine::tax::{self, PersonIncome, StateIncome};

use common::{params_2026, tables_with};

const INFLATION: f64 = 0.02;

/// A state taking a tenth of everything, with `rows` before its brackets.
fn flat_tenth(rows: &str) -> StateParams {
    let text = format!(
        "{rows}\n[brackets]\nsingle = [{{ over = 0, rate = 0.1 }}]\nmarried-joint = [{{ over = 0, rate = 0.1 }}]\n"
    );
    toml::from_str(&text).unwrap()
}

/// Someone born on the 15th of `month` in `year`, with `income` by source.
fn person(year: i16, month: i8, income: &[(Source, Dollars)]) -> PersonIncome {
    let mut person = PersonIncome::new(PlanDate(jiff::civil::date(year, month, 15)));
    for &(source, amount) in income {
        person.add(source, amount);
    }
    person
}

fn in_year(year: i16, people: &[PersonIncome]) -> StateIncome<'_> {
    StateIncome {
        year,
        people,
        gains: 0,
        taxable_social_security: 0,
        ira_deducted: 0,
    }
}

/// What one person owes `state` in `year`.
fn owed(state: &StateParams, year: i16, person: PersonIncome) -> Dollars {
    tax::state_tax(state, FilingStatus::Single, &in_year(year, &[person]))
}

fn earning(wages: Dollars) -> PersonIncome {
    person(1980, 6, &[(Source::Wages, wages)])
}

#[test]
fn a_state_taxes_income_less_its_deduction_through_its_own_brackets() {
    let params = params_2026();
    let oregon = &params.states["or"];
    // Publication OR-ESTIMATE's own example: 68,000 taxable, jointly.
    let joint = [earning(68_000 + 5_800)];
    assert_eq!(
        tax::state_tax(oregon, FilingStatus::MarriedJoint, &in_year(2026, &joint)),
        5_312
    );
    // Single, 50,000 taxable: 216 + 462 + 3,377.50.
    assert_eq!(owed(oregon, 2026, earning(50_000 + 2_900)), 4_056);
    assert_eq!(owed(oregon, 2026, earning(1_000)), 0);
}

#[test]
fn a_state_taxes_gains_as_it_taxes_the_rest() {
    let people = [earning(50_000)];
    let income = StateIncome {
        gains: 10_000,
        ..in_year(2026, &people)
    };
    let owed = tax::state_tax(&flat_tenth(""), FilingStatus::Single, &income);
    assert_eq!(owed, 6_000);
}

#[test]
fn a_state_taxes_social_security_only_where_it_says_so() {
    let people = [earning(40_000)];
    let income = StateIncome {
        taxable_social_security: 20_000,
        ..in_year(2026, &people)
    };
    let exempt = tax::state_tax(&flat_tenth(""), FilingStatus::Single, &income);
    assert_eq!(exempt, 4_000);
    let taxing = flat_tenth("taxes-social-security = true");
    assert_eq!(
        tax::state_tax(&taxing, FilingStatus::Single, &income),
        6_000
    );
}

#[test]
fn a_state_with_no_income_tax_is_a_table_and_one_not_modeled_is_none() {
    let params = params_2026();
    let people = [earning(500_000)];
    let income = StateIncome {
        taxable_social_security: 50_000,
        ..in_year(2026, &people)
    };
    for code in ["ak", "fl", "nv", "nh", "sd", "tn", "tx", "wa", "wy"] {
        let owed = tax::state_tax(&params.states[code], FilingStatus::Single, &income);
        assert_eq!(owed, 0, "{code}");
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

#[test]
fn a_row_without_an_age_exempts_its_sources_at_any_age() {
    let state = flat_tenth("[[exclusions]]\nsources = [\"pension\", \"conversion\"]");
    let income = [
        (Source::Wages, 50_000),
        (Source::Pension, 20_000),
        (Source::Conversion, 30_000),
        (Source::Distribution, 10_000),
    ];
    // Born 1990: 36 in 2026. The wages and the distribution are taxed.
    assert_eq!(owed(&state, 2026, person(1990, 6, &income)), 6_000);
    assert_eq!(
        owed(&flat_tenth(""), 2026, person(1990, 6, &income)),
        11_000
    );
}

#[test]
fn a_row_with_an_age_exempts_from_the_year_the_age_is_reached() {
    let state = flat_tenth("[[exclusions]]\nsources = [\"distribution\"]\nfrom-age = 59.5");
    let drawing = |month| person(1970, month, &[(Source::Distribution, 40_000)]);
    // Born June 1970: 59 and a half in December 2029.
    assert_eq!(owed(&state, 2028, drawing(6)), 4_000);
    assert_eq!(owed(&state, 2029, drawing(6)), 0);
    assert_eq!(owed(&state, 2030, drawing(6)), 0);
    // Born August 1970: 59 and a half in February 2030.
    assert_eq!(owed(&state, 2029, drawing(8)), 4_000);
    assert_eq!(owed(&state, 2030, drawing(8)), 0);
    let whole = flat_tenth("[[exclusions]]\nsources = [\"distribution\"]\nfrom-age = 55");
    // Born December 1970: 55 in December 2025.
    assert_eq!(owed(&whole, 2024, drawing(12)), 4_000);
    assert_eq!(owed(&whole, 2025, drawing(12)), 0);
}

#[test]
fn a_row_naming_distributions_leaves_an_early_one_taxed() {
    let state = flat_tenth("[[exclusions]]\nsources = [\"distribution\"]");
    let income = [
        (Source::Distribution, 10_000),
        (Source::EarlyDistribution, 20_000),
    ];
    assert_eq!(owed(&state, 2026, person(1980, 6, &income)), 2_000);
}

#[test]
fn each_spouse_is_exempt_by_their_own_age() {
    let state = flat_tenth("[[exclusions]]\nsources = [\"distribution\"]\nfrom-age = 55");
    // Born 1970 and 1975: 56 and 51 in 2026.
    let couple = [
        person(1970, 6, &[(Source::Distribution, 30_000)]),
        person(1975, 6, &[(Source::Distribution, 20_000)]),
    ];
    let owed = |year| tax::state_tax(&state, FilingStatus::MarriedJoint, &in_year(year, &couple));
    assert_eq!(owed(2024), 5_000);
    assert_eq!(owed(2026), 2_000);
    assert_eq!(owed(2030), 0);
}

#[test]
fn a_state_that_taxes_deferrals_taxes_what_is_paid_into_a_plan_or_an_ira() {
    // 100,000 of salary less 4,000 paid into an HSA, and 10,000 deferred.
    let worker = [person(
        1980,
        6,
        &[(Source::Wages, 96_000), (Source::Deferral, -10_000)],
    )];
    let income = StateIncome {
        ira_deducted: 7_000,
        ..in_year(2026, &worker)
    };
    let following = tax::state_tax(&flat_tenth(""), FilingStatus::Single, &income);
    assert_eq!(following, 7_900);
    let taxing = flat_tenth("taxes-deferrals = true");
    assert_eq!(
        tax::state_tax(&taxing, FilingStatus::Single, &income),
        9_600
    );
}

#[test]
fn a_row_exempts_what_a_source_holds_and_no_more() {
    let state = flat_tenth("[[exclusions]]\nsources = [\"deferral\", \"wages\"]");
    let pension = (Source::Pension, 50_000);
    // An HSA contribution with no salary under it still reduces income.
    let retired = [
        (Source::Wages, -4_000),
        (Source::Deferral, -10_000),
        pension,
    ];
    assert_eq!(owed(&state, 2026, person(1980, 6, &retired)), 3_600);
    let working = [
        (Source::Wages, 96_000),
        (Source::Deferral, -10_000),
        pension,
    ];
    assert_eq!(owed(&state, 2026, person(1980, 6, &working)), 4_000);
}

const FIXED: &str = r"
[states.ca]
deduction-unindexed = true

[states.ca.deduction]
single = 8300
married-joint = 16600

[states.ca.brackets]
single = [
  { over = 10000, rate = 0.04, unindexed = true, later = [
    { from = 2029, rate = 0.0325 },
    { from = 2027, rate = 0.0375 },
    { from = 2030, rate = 0.03 },
  ] },
]
married-joint = [{ over = 10000, rate = 0.04, unindexed = true }]
";

#[test]
fn a_deduction_the_law_fixes_is_not_inflated() {
    let tables = tables_with(FIXED);
    let later = tables.params_for(2036, &Inflation::constant(INFLATION));
    assert_eq!(later.states["ca"].deduction.single, 8_300);
    assert_eq!(later.states["ca"].deduction.married_joint, 16_600);
    let indexed = tables_with(&FIXED.replace("deduction-unindexed = true", ""));
    let later = indexed.params_for(2036, &Inflation::constant(INFLATION));
    // 8,300 at 2% for ten years.
    assert_eq!(later.states["ca"].deduction.single, 10_118);
}

#[track_caller]
fn assert_rates(tables: &TaxTables, rate_of: fn(&TaxParams) -> f64) {
    let stepped = [
        (2025, 0.04),
        (2026, 0.04),
        (2027, 0.0375),
        (2028, 0.0375),
        (2029, 0.0325),
        (2030, 0.03),
        (2060, 0.03),
    ];
    for (year, rate) in stepped {
        let params = tables.params_for(year, &Inflation::constant(INFLATION));
        assert!((rate_of(&params) - rate).abs() < f64::EPSILON, "{year}");
    }
}

#[test]
fn a_bracket_takes_the_rate_the_law_has_set_by_the_year() {
    let tables = tables_with(FIXED);
    assert_rates(&tables, |params| {
        params.states["ca"].brackets.single[0].rate
    });
    let unstepped = tables.params_for(2040, &Inflation::constant(INFLATION));
    let joint = &unstepped.states["ca"].brackets.married_joint[0];
    assert!((joint.rate - 0.04).abs() < f64::EPSILON);
}

#[test]
fn a_federal_bracket_steps_as_a_state_s_does() {
    let embedded = include_str!("../tax/2026.toml");
    let first = "  { over = 0, rate = 0.10 },";
    let stepping = "  { over = 0, rate = 0.04, later = [{ from = 2027, rate = 0.0375 }, { from = 2029, rate = 0.0325 }, { from = 2030, rate = 0.03 }] },";
    assert!(embedded.contains(first));
    let mut tables = TaxTables::embedded();
    tables
        .add_source(&embedded.replacen(first, stepping, 1), "test")
        .unwrap();
    assert_rates(&tables, |params| params.brackets.single[0].rate);
}

#[test]
fn a_table_without_the_new_keys_reads_as_it_did_and_one_with_them_writes_back() {
    let plain = flat_tenth("");
    assert!(plain.exclusions.is_empty());
    assert!(!plain.taxes_deferrals && !plain.deduction_unindexed);
    assert!(plain.brackets.single[0].later.is_empty());
    let written = toml::to_string(&plain).unwrap();
    for key in [
        "exclusions",
        "taxes-deferrals",
        "deduction-unindexed",
        "later",
    ] {
        assert!(!written.contains(key), "{written}");
    }
    let full = tables_with(&format!(
        "{FIXED}\n[[states.ca.exclusions]]\nsources = [\"pension\", \"early-distribution\"]\nfrom-age = 59.5\n\n[[states.ca.exclusions]]\nsources = [\"conversion\"]\n"
    ));
    let state = &full
        .params_for(2026, &Inflation::constant(INFLATION))
        .states["ca"];
    assert_eq!(state.exclusions[0].from_age, Some(59.5));
    assert_eq!(state.exclusions[1].sources, [Source::Conversion]);
    let read: StateParams = toml::from_str(&toml::to_string(state).unwrap()).unwrap();
    assert_eq!(&read, state);
    let misspelt = "[[exclusions]]\nsources = [\"pensions\"]\n";
    assert!(toml::from_str::<StateParams>(misspelt).is_err());
}
