//! Round-trip and schema tests for the plan document.

mod common;

use retiretui_engine::plan::{AccountKind, IncomeKind, Plan, TreatmentClass, TriggerForm};

use common::FULL;

#[test]
fn parses_the_full_fixture() {
    let plan = Plan::from_toml_str(FULL).unwrap();
    assert_eq!(plan.schema, 1);
    assert_eq!(plan.plan.start_year, 2026);
    assert_eq!(plan.household.people.len(), 2);
    assert_eq!(plan.accounts.len(), 7);
    assert_eq!(plan.income.len(), 4);
    assert_eq!(plan.expenses.len(), 2);
    assert_eq!(plan.transfers.len(), 1);
    assert_eq!(plan.conversions.len(), 1);
    assert_eq!(plan.events.len(), 1);
    assert_eq!(plan.residency.len(), 1);
}

#[test]
fn fixture_is_valid() {
    let plan = Plan::from_toml_str(FULL).unwrap();
    let issues = plan.validate();
    assert!(issues.is_empty(), "unexpected issues: {issues:?}");
}

#[test]
fn round_trips_through_canonical_toml() {
    let plan = Plan::from_toml_str(FULL).unwrap();
    let text = plan.to_toml_string().unwrap();
    let reparsed = Plan::from_toml_str(&text).unwrap();
    assert_eq!(plan, reparsed);
}

#[test]
fn account_kinds_map_to_treatments() {
    let plan = Plan::from_toml_str(FULL).unwrap();
    let treatment = |id: &str| plan.account(id).unwrap().treatment();
    assert_eq!(treatment("cash"), TreatmentClass::Taxable);
    assert_eq!(treatment("brokerage"), TreatmentClass::Taxable);
    assert_eq!(treatment("fid-401k"), TreatmentClass::Deferred);
    assert_eq!(treatment("roth-ira"), TreatmentClass::Roth);
    assert_eq!(treatment("pension-dc"), TreatmentClass::Deferred);
    assert_eq!(treatment("hsa"), TreatmentClass::Hsa);
    assert_eq!(plan.account("pension-dc").unwrap().kind, AccountKind::K414k);
}

#[test]
fn trigger_forms_classify() {
    let plan = Plan::from_toml_str(FULL).unwrap();
    let retire = &plan.events[0].trigger;
    assert_eq!(
        retire.form().unwrap(),
        TriggerForm::Age {
            owner: "jordan",
            years: 62
        }
    );
    let salary_end = plan.income_source("salary").unwrap().end.as_ref().unwrap();
    assert_eq!(
        salary_end.form().unwrap(),
        TriggerForm::Event {
            id: "retire",
            offset: -1
        }
    );
    let pension_dc = plan.account("pension-dc").unwrap();
    assert_eq!(
        pension_dc.locked_until.as_ref().unwrap().form().unwrap(),
        TriggerForm::Income {
            id: "db-pension",
            offset: 0
        }
    );
    let windfall = plan
        .income
        .iter()
        .find(|income| income.kind == IncomeKind::Windfall)
        .unwrap();
    assert!(matches!(
        windfall.on.as_ref().unwrap().form().unwrap(),
        TriggerForm::Date(_)
    ));
}

#[test]
fn unknown_fields_are_rejected() {
    let text = FULL.replace("name = \"base\"", "name = \"base\"\nbogus_key = 1");
    assert!(Plan::from_toml_str(&text).is_err());
}

#[test]
fn a_person_s_name_is_kept_and_shown_in_place_of_the_id() {
    let text = r#"
schema = 1

[plan]
name = "named"
start_year = 2026
horizon_age = 90
inflation = 0.025

[household]
filing = "married-joint"

[[household.people]]
id = "pat"
name = "Pat Lee"
birth = 1970-01-01

[[household.people]]
id = "sam"
birth = 1971-01-01
"#;
    let plan = retiretui_engine::plan::Plan::from_toml_str(text).unwrap();
    assert_eq!(plan.person_name("pat"), "Pat Lee");
    assert_eq!(plan.person_name("sam"), "sam", "no name shows the id");
    assert_eq!(plan.person_name("nobody"), "nobody");
    let written = plan.to_toml_string().unwrap();
    assert!(written.contains("name = \"Pat Lee\""), "{written}");
    let again = retiretui_engine::plan::Plan::from_toml_str(&written).unwrap();
    assert_eq!(again, plan);
}
