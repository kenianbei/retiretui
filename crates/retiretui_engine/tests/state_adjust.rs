//! What a state's table can adjust its tax by: a deduction lost above an
//! income or added to at 65, federal tax subtracted, a credit for each
//! person, and an excise on gains. Each over a table made here.

mod common;

use std::collections::BTreeMap;

use retiretui_engine::params::{Inflation, Source, StateParams};
use retiretui_engine::plan::{Dollars, FilingStatus};
use retiretui_engine::tax::{self, PersonIncome, StateIncome};

use common::{TENTH, flat_tenth, in_year, person, tables_with};

const INFLATION: f64 = 0.02;

/// Someone born in June of `year`.
fn born(year: i16) -> PersonIncome {
    person(year, 6, &[])
}

/// 2026 for `people`, the first of them with `wages`, which is the AGI.
fn earning(people: &mut [PersonIncome], wages: Dollars) -> StateIncome<'_> {
    people[0].add(Source::Wages, wages);
    StateIncome {
        agi: wages,
        ..in_year(2026, people)
    }
}

/// What `state` takes of `income`, from one person alone or two jointly.
fn owed(state: &StateParams, income: &StateIncome) -> Dollars {
    let status = if income.people.len() == 2 {
        FilingStatus::MarriedJoint
    } else {
        FilingStatus::Single
    };
    tax::state_tax(state, status, income)
}

/// What one person born in 1980 owes on `wages` and nothing else.
fn owed_on(state: &StateParams, wages: Dollars) -> Dollars {
    owed(state, &earning(&mut [born(1980)], wages))
}

const DEDUCTION: &str = "
[deduction]
single = 3000
married-joint = 6000
";

const CLIFF: &str = "
[deduction-until-agi]
single = 250000
married-joint = 500000
";

#[test]
fn a_deduction_is_lost_whole_above_the_agi_its_table_names() {
    let state = flat_tenth(&format!("{DEDUCTION}{CLIFF}"));
    assert_eq!(owed_on(&state, 250_000), 24_700);
    assert_eq!(owed_on(&state, 250_001), 25_000);
    let couple = |wages| owed(&state, &earning(&mut [born(1980), born(1982)], wages));
    assert_eq!(couple(500_000), 49_400);
    assert_eq!(couple(500_001), 50_000);
}

const AT_65: &str = "
[deduction-at-65]
single = 1200
married-joint = 1000
";

#[test]
fn each_person_adds_to_the_deduction_from_the_year_they_reach_65() {
    let state = flat_tenth(&format!("{DEDUCTION}{AT_65}"));
    let alone = |birth_year| owed(&state, &earning(&mut [born(birth_year)], 50_000));
    // Born June 1962: 64 in 2026. Born June 1961: 65 in 2026.
    assert_eq!(alone(1962), 4_700);
    assert_eq!(alone(1961), 4_580);
    let couple = |older, younger| owed(&state, &earning(&mut [born(older), born(younger)], 50_000));
    assert_eq!(couple(1962, 1970), 4_400);
    assert_eq!(couple(1961, 1970), 4_300);
    assert_eq!(couple(1955, 1961), 4_200);
}

#[test]
fn an_addition_at_65_goes_with_the_deduction_it_adds_to() {
    let keys = format!(
        "{DEDUCTION}{AT_65}\n[deduction-until-agi]\nsingle = 100000\nmarried-joint = 200000"
    );
    let state = flat_tenth(&keys);
    let owed = |wages| owed(&state, &earning(&mut [born(1950)], wages));
    assert_eq!(owed(100_000), 9_580);
    assert_eq!(owed(100_001), 10_000);
}

const SUBTRACTION: &str = "
[federal-tax-subtraction]
cap = 8750
steps = 5

[federal-tax-subtraction.phase-out]
single = { from = 125000, to = 145000 }
married-joint = { from = 250000, to = 290000 }
";

/// What `people` owe on `agi` of wages with `federal_tax` paid.
fn owed_after(
    state: &StateParams,
    people: &mut [PersonIncome],
    agi: Dollars,
    federal_tax: Dollars,
) -> Dollars {
    let income = StateIncome {
        federal_tax,
        ..earning(people, agi)
    };
    owed(state, &income)
}

#[test]
fn federal_tax_is_subtracted_up_to_the_cap() {
    let state = flat_tenth(SUBTRACTION);
    let owed = |federal_tax| owed_after(&state, &mut [born(1980)], 80_000, federal_tax);
    assert_eq!(owed(0), 8_000);
    assert_eq!(owed(5_020), 7_498);
    assert_eq!(owed(8_750), 7_125);
    assert_eq!(owed(8_770), 7_125);
    assert_eq!(owed(40_000), 7_125);
}

#[test]
fn the_cap_falls_a_step_at_the_foot_of_its_band_and_at_each_step_to_its_top() {
    // A state taking everything, so that what it leaves is what it subtracts.
    let taking_all = format!("{}{SUBTRACTION}", TENTH.replace("0.1", "1.0"));
    let state: StateParams = toml::from_str(&taking_all).unwrap();
    let subtracted =
        |people: &mut [PersonIncome], agi: Dollars| agi - owed_after(&state, people, agi, 50_000);
    let alone = |agi| subtracted(&mut [born(1980)], agi);
    let stepped = [
        (124_999, 8_750),
        (125_000, 7_000),
        (129_999, 7_000),
        (130_000, 5_250),
        (135_000, 3_500),
        (140_000, 1_750),
        (144_999, 1_750),
        (145_000, 0),
        (900_000, 0),
    ];
    for (agi, cap) in stepped {
        assert_eq!(alone(agi), cap, "{agi}");
    }
    let couple = |agi| subtracted(&mut [born(1980), born(1982)], agi);
    let stepped = [
        (249_990, 8_750),
        (250_000, 7_000),
        (260_000, 5_250),
        (279_990, 3_500),
        (280_000, 1_750),
        (290_000, 0),
    ];
    for (agi, cap) in stepped {
        assert_eq!(couple(agi), cap, "{agi}");
    }
}

const CREDIT: &str = "
[exemption-credit]
per-person = 263

[exemption-credit.until-agi]
single = 100000
married-joint = 200000
";

#[test]
fn a_credit_for_each_person_comes_off_the_tax_and_is_never_refunded() {
    let state = flat_tenth(CREDIT);
    assert_eq!(owed_on(&state, 50_000), 4_737);
    assert_eq!(owed_on(&state, 2_000), 0);
    let couple = owed(&state, &earning(&mut [born(1980), born(1982)], 50_000));
    assert_eq!(couple, 4_474);
}

#[test]
fn a_credit_is_lost_whole_above_the_agi_its_table_names() {
    let state = flat_tenth(CREDIT);
    assert_eq!(owed_on(&state, 100_000), 9_737);
    assert_eq!(owed_on(&state, 100_001), 10_000);
    let couple = |wages| owed(&state, &earning(&mut [born(1980), born(1982)], wages));
    assert_eq!(couple(200_000), 19_474);
    assert_eq!(couple(200_001), 20_000);
    let unlimited = flat_tenth("[exemption-credit]\nper-person = 40");
    assert_eq!(owed_on(&unlimited, 900_000), 89_960);
}

#[test]
fn a_credit_adds_for_each_person_from_the_year_they_reach_65() {
    let state = flat_tenth("[exemption-credit]\nper-person = 40\nat-65 = 20");
    let couple = |older, younger| owed(&state, &earning(&mut [born(older), born(younger)], 50_000));
    assert_eq!(couple(1962, 1970), 4_920);
    assert_eq!(couple(1961, 1970), 4_900);
    assert_eq!(couple(1955, 1961), 4_880);
}

const EXCISE: &str = "
[gains-excise]
deduction = 290000
brackets = [
  { over = 0, rate = 0.07 },
  { over = 1000000, rate = 0.099, unindexed = true },
]
";

#[test]
fn an_excise_is_levied_on_the_gains_its_deduction_leaves() {
    let state: StateParams = toml::from_str(EXCISE).unwrap();
    let owed = |people: &mut [PersonIncome], gains| {
        let income = StateIncome {
            gains,
            ..earning(people, 400_000)
        };
        owed(&state, &income)
    };
    let alone = |gains| owed(&mut [born(1980)], gains);
    assert_eq!(alone(0), 0);
    assert_eq!(alone(290_000), 0);
    assert_eq!(alone(500_000), 14_700);
    // 70,000 on the first million taxed, and 9.9% of the 210,000 beyond.
    assert_eq!(alone(1_500_000), 90_790);
    assert_eq!(owed(&mut [born(1980), born(1982)], 500_000), 14_700);
}

#[test]
fn an_excise_is_added_after_a_credit_has_taken_the_income_tax_to_nothing() {
    let state: StateParams = toml::from_str(&format!("{CREDIT}{EXCISE}")).unwrap();
    let mut people = [born(1980)];
    let income = StateIncome {
        gains: 300_000,
        ..earning(&mut people, 0)
    };
    assert_eq!(owed(&state, &income), 700);
}

/// The table `keys` make of a state the embedded tables lack, as `year`
/// reads it.
fn table_in(keys: &str, year: i16) -> StateParams {
    let states = BTreeMap::from([("ca", flat_tenth(keys))]);
    let text = toml::to_string(&BTreeMap::from([("states", states)])).unwrap();
    let params = tables_with(&text).params_for(year, &Inflation::constant(INFLATION));
    params.states["ca"].clone()
}

#[test]
fn what_the_law_indexes_grows_and_what_it_fixes_does_not() {
    let keys = format!("{AT_65}{CLIFF}{SUBTRACTION}{CREDIT}{EXCISE}");
    assert_eq!(table_in(&keys, 2026), flat_tenth(&keys));
    let later = table_in(&keys, 2036);
    let subtraction = later.federal_tax_subtraction.unwrap();
    assert_eq!(subtraction.cap, 10_666);
    assert_eq!(subtraction.phase_out.single.from, 125_000);
    assert_eq!(subtraction.phase_out.married_joint.to, 290_000);
    let credit = later.exemption_credit.unwrap();
    assert_eq!(credit.per_person, 321);
    assert_eq!(credit.until_agi.unwrap().single, 100_000);
    let excise = later.gains_excise.unwrap();
    assert_eq!(excise.deduction, 353_508);
    assert_eq!(excise.brackets[1].over, 1_000_000);
    assert_eq!(later.deduction_at_65.single, 1_200);
    assert_eq!(later.deduction_until_agi.unwrap().single, 250_000);
}

#[test]
fn a_credit_the_law_fixes_is_not_inflated() {
    let keys = "\n[exemption-credit]\nper-person = 40\nat-65 = 20\nunindexed = true\n";
    let credit = table_in(keys, 2036).exemption_credit.unwrap();
    assert_eq!((credit.per_person, credit.at_65), (40, 20));
    let indexed = table_in(&keys.replace("unindexed = true", ""), 2036);
    let credit = indexed.exemption_credit.unwrap();
    assert_eq!((credit.per_person, credit.at_65), (49, 20));
}

#[test]
fn an_excise_bracket_takes_the_rate_the_law_sets_for_a_later_year() {
    let keys = EXCISE.replace(
        "{ over = 0, rate = 0.07 }",
        "{ over = 0, rate = 0.07, later = [{ from = 2028, rate = 0.08 }] }",
    );
    let rate_in = |year| table_in(&keys, year).gains_excise.unwrap().brackets[0].rate;
    assert!((rate_in(2027) - 0.07).abs() < f64::EPSILON);
    assert!((rate_in(2028) - 0.08).abs() < f64::EPSILON);
}

#[test]
fn a_table_is_written_back_without_the_keys_it_does_not_state() {
    let bare = toml::to_string(&flat_tenth("")).unwrap();
    for key in ["at-65", "until-agi", "subtraction", "credit", "excise"] {
        assert!(!bare.contains(key), "{key}: {bare}");
    }
    let keys = format!("{AT_65}{SUBTRACTION}{CREDIT}{EXCISE}");
    let stated = flat_tenth(&keys);
    let written = toml::to_string(&stated).unwrap();
    assert_eq!(toml::from_str::<StateParams>(&written).unwrap(), stated);
    assert!(toml::from_str::<StateParams>(&SUBTRACTION.replace("steps = 5", "steps = 0")).is_err());
}
