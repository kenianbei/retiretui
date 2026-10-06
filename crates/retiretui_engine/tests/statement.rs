//! Reading the Social Security statement XML into a person's record.

mod common;

use retiretui_engine::plan::Plan;
use retiretui_engine::statement::{StatementError, parse};

use common::FULL;

const STATEMENT: &str = include_str!("fixtures/statement.xml");

#[test]
fn reads_birth_and_every_earnings_year() {
    let statement = parse(STATEMENT).unwrap();
    assert_eq!(statement.birth.0.to_string(), "1975-06-14");
    assert_eq!(
        statement.earnings,
        [(1995, 4_200), (1996, 0), (2024, 168_600)].into()
    );
}

#[test]
fn refuses_what_is_not_a_statement() {
    assert_eq!(
        parse("<html>not a statement</html>"),
        Err(StatementError::NotAStatement)
    );
    let no_birth = STATEMENT.replace("DateOfBirth", "Birthday");
    assert_eq!(
        parse(&no_birth),
        Err(StatementError::Missing("DateOfBirth"))
    );
    let bad_amount = STATEMENT.replace("<osss:FicaEarnings>4200<", "<osss:FicaEarnings>4,200<");
    assert!(matches!(
        parse(&bad_amount),
        Err(StatementError::Malformed {
            tag: "FicaEarnings",
            ..
        })
    ));
}

#[test]
fn spreads_a_sum_over_several_years_evenly_with_the_remainder_last() {
    let grouped = STATEMENT
        .replace(
            "startYear=\"1995\" endYear=\"1995\"",
            "startYear=\"1991\" endYear=\"1995\"",
        )
        .replace("<osss:FicaEarnings>4200<", "<osss:FicaEarnings>4203<");
    let statement = parse(&grouped).unwrap();
    assert_eq!(
        statement.earnings,
        [
            (1991, 840),
            (1992, 840),
            (1993, 840),
            (1994, 840),
            (1995, 843),
            (1996, 0),
            (2024, 168_600)
        ]
        .into()
    );
    assert_eq!(statement.grouped, [(1991, 1995)]);
    let note = statement.spread_note().unwrap();
    assert!(note.contains("1991-1995"), "{note}");
    assert_eq!(parse(STATEMENT).unwrap().spread_note(), None);
}

#[test]
fn refuses_a_range_that_runs_backwards() {
    let backwards = STATEMENT.replace(
        "startYear=\"1995\" endYear=\"1995\"",
        "startYear=\"1995\" endYear=\"1991\"",
    );
    assert!(matches!(
        parse(&backwards),
        Err(StatementError::Malformed {
            tag: "Earnings",
            ..
        })
    ));
}

#[test]
fn adopting_replaces_the_record_of_the_person_it_is_for() {
    let statement = parse(STATEMENT).unwrap();
    let mut plan = Plan::from_toml_str(FULL).unwrap();
    plan.household.people[0].earnings.insert(1980, 1);
    plan.adopt_earnings("jordan", &statement).unwrap();
    assert_eq!(plan.household.people[0].earnings, statement.earnings);
    assert_eq!(plan.validate(), []);
    let wrong_person = plan.adopt_earnings("alex", &statement).unwrap_err();
    assert_eq!(wrong_person.path, "household.people[1].birth");
    assert!(
        wrong_person.message.contains("1979-02-01"),
        "{wrong_person}"
    );
    let nobody = plan.adopt_earnings("sam", &statement).unwrap_err();
    assert!(nobody.message.contains("unknown person"));
}
