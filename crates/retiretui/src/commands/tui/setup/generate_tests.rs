use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{FilingStatus, Plan, PlanDate, Trigger};
use retiretui_engine::project::{project, validate_plan};

use super::tests::as_table;
use super::{LifeStage, SetupAnswers, generate};
use crate::commands::tui::edit::from_table;

const START_YEAR: i16 = 2026;

/// A filled-in answer to every question, so a generated plan is the
/// richest shape the form can build.
const ANSWERED: &str = r#"
name = "Jordan Example"
birth_year = 1975
retirement_age = 62
salary = 150000
social_security = 40000
claim_age = 67
partner_name = "Alex"
partner_birth_year = 1979
partner_retirement_age = 65
partner_salary = 90000
partner_social_security = 30000
partner_claim_age = 67
"#;

fn answered(filing: FilingStatus, stage: LifeStage, body: &str) -> SetupAnswers {
    from_table(as_table(filing, stage, body)).expect("the answers parse")
}

fn generated(filing: FilingStatus, stage: LifeStage, body: &str) -> Plan {
    let tables = TaxTables::embedded();
    generate::plan(&answered(filing, stage, body), START_YEAR, &tables)
        .expect("the answers build a plan")
}

fn benefit_of<'a>(plan: &'a Plan, owner: &str) -> &'a retiretui_engine::plan::Income {
    plan.income
        .iter()
        .find(|income| income.id == format!("ss-{owner}"))
        .expect("the benefit")
}

fn every_household() -> impl Iterator<Item = (FilingStatus, LifeStage)> {
    FilingStatus::ALL.iter().copied().flat_map(|filing| {
        LifeStage::ALL
            .iter()
            .copied()
            .map(move |stage| (filing, stage))
    })
}

#[test]
fn every_household_the_form_asks_about_validates_and_projects() {
    let tables = TaxTables::embedded();
    for (filing, stage) in every_household() {
        for body in ["", ANSWERED] {
            let plan = generated(filing, stage, body);
            let issues = validate_plan(&plan, &tables);
            assert!(issues.is_empty(), "{filing:?} {stage:?}: {issues:?}");
            let projection = project(&plan, &tables);
            assert!(!projection.years.is_empty(), "{filing:?} {stage:?}");
            let workplace = plan
                .accounts
                .iter()
                .filter(|account| account.kind == retiretui_engine::plan::AccountKind::K401k);
            assert!(
                workplace.clone().count() > 0
                    && workplace
                        .clone()
                        .all(|account| account.allocation.is_some()),
                "a new workplace account holds a mix: {filing:?} {stage:?}"
            );
        }
    }
}

#[test]
fn the_people_a_filing_status_names_are_the_ones_it_needs() {
    for stage in LifeStage::ALL.iter().copied() {
        let single = generated(FilingStatus::Single, stage, ANSWERED);
        assert_eq!(single.household.people.len(), 1, "{stage:?}");
        let joint = generated(FilingStatus::MarriedJoint, stage, ANSWERED);
        assert_eq!(joint.household.people.len(), 2, "{stage:?}");
    }
}

#[test]
fn nothing_a_generated_plan_names_dangles() {
    for (filing, stage) in every_household() {
        let plan = generated(filing, stage, ANSWERED);
        let people: Vec<&str> = plan
            .household
            .people
            .iter()
            .map(|person| person.id.as_str())
            .collect();
        let events: Vec<&str> = plan.events.iter().map(|event| event.id.as_str()).collect();
        let named = format!("{filing:?} {stage:?}");
        let resolves = |trigger: Option<&Trigger>| {
            let Some(trigger) = trigger else {
                return true;
            };
            trigger
                .owner
                .as_ref()
                .is_none_or(|owner| people.contains(&owner.as_str()))
                && trigger
                    .event
                    .as_ref()
                    .is_none_or(|event| events.contains(&event.as_str()))
        };
        for account in &plan.accounts {
            assert!(people.contains(&account.owner.as_str()), "{named}");
        }
        for income in &plan.income {
            assert!(people.contains(&income.owner.as_str()), "{named}");
            assert!(resolves(income.start.as_ref()), "{named}: {income:?}");
            assert!(resolves(income.end.as_ref()), "{named}: {income:?}");
        }
        for event in &plan.events {
            assert!(resolves(Some(&event.trigger)), "{named}: {event:?}");
        }
    }
}

#[test]
fn a_salary_stops_where_the_person_retires_and_a_retiree_has_none() {
    let working = generated(FilingStatus::Single, LifeStage::Working, ANSWERED);
    assert_eq!(working.events.len(), 1, "a retirement to end the salary at");
    let salary = working
        .income
        .iter()
        .find(|income| income.id == "salary-jordanexample")
        .expect("the salary");
    assert!(salary.end.is_some(), "it ends at the retirement");

    let retired = generated(FilingStatus::Single, LifeStage::Retired, ANSWERED);
    assert!(retired.events.is_empty(), "nothing left to retire at");
    assert!(
        retired
            .income
            .iter()
            .all(|income| income.kind.as_str() != "salary"),
        "a retiree earns none"
    );
    let benefit = retired.income.first().expect("the benefit");
    let claimed = benefit.start.as_ref().expect("a benefit states its claim");
    assert_eq!(
        claimed.date.map(PlanDate::year),
        Some(START_YEAR),
        "a retiree claims from the plan's first year"
    );
}

#[test]
fn a_working_person_with_no_figure_typed_gets_a_benefit_computed_from_a_career() {
    let tables = TaxTables::embedded();
    let untyped = ANSWERED.replace("social_security = 40000\n", "");
    let plan = generated(FilingStatus::MarriedJoint, LifeStage::Working, &untyped);
    let computed = benefit_of(&plan, "jordanexample");
    assert_eq!(computed.amount, None, "left for the engine");
    assert!(computed.is_derived());
    assert_eq!(
        computed.start.as_ref().and_then(|start| start.age),
        Some(67)
    );
    let record = &plan.household.people[0].earnings;
    assert_eq!(record.keys().next(), Some(&(1975 + 22)), "from age 22");
    assert_eq!(record.keys().next_back(), Some(&(START_YEAR - 1)));
    assert_eq!(
        record[&(START_YEAR - 1)],
        144_788,
        "the salary a year back by the wage growth"
    );
    assert!(
        record[&2000] < record[&2024],
        "scaled down by the wage index"
    );

    let typed = benefit_of(&plan, "alex");
    assert_eq!(typed.amount, Some(30_000), "the partner's figure stands");
    assert!(plan.household.people[1].earnings.is_empty());

    assert!(validate_plan(&plan, &tables).is_empty());
    let found = retiretui_engine::optimize::optimize_claims(&plan, &tables, &[], &[]).unwrap();
    assert_eq!(
        found.incomes,
        ["ss-jordanexample"],
        "and the claim search takes it"
    );
    assert_eq!(found.candidates.len(), 9);

    let retired = generated(FilingStatus::Single, LifeStage::Retired, &untyped);
    assert!(
        retired.income.is_empty(),
        "a retiree with no figure and no salary has no benefit to compute"
    );
}

#[test]
fn a_name_that_writes_nothing_down_still_gets_an_id_of_its_own() {
    let plan = generated(
        FilingStatus::MarriedJoint,
        LifeStage::Working,
        "name = \"!!!\"\npartner_name = \"???\"\n",
    );
    let ids: Vec<&str> = plan
        .household
        .people
        .iter()
        .map(|person| person.id.as_str())
        .collect();
    assert_eq!(ids, ["person", "person2"], "{ids:?}");
}

#[test]
fn two_people_spelt_the_same_are_still_two_people() {
    let plan = generated(
        FilingStatus::MarriedJoint,
        LifeStage::Working,
        "name = \"Sam Lee\"\npartner_name = \"sam-lee\"\n",
    );
    let ids: Vec<&str> = plan
        .household
        .people
        .iter()
        .map(|person| person.id.as_str())
        .collect();
    assert_eq!(ids, ["samlee", "samlee2"], "{ids:?}");
    let issues = validate_plan(&plan, &TaxTables::embedded());
    assert!(issues.is_empty(), "{issues:?}");
}

#[test]
fn a_typed_name_is_kept_as_the_person_s_name_beside_the_id_made_of_it() {
    let plan = generated(FilingStatus::MarriedJoint, LifeStage::Working, ANSWERED);
    let partner = &plan.household.people[1];
    assert_eq!(partner.name.as_deref(), Some("Alex"));
    assert_eq!(partner.display_name(), "Alex");
    assert_eq!(partner.id, "alex");
}
