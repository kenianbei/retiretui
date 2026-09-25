//! Scenario overlay merge tests.

use retiretui_engine::plan::{Issue, Plan, Scenario, ScenarioError};

const BASE: &str = r#"
schema = 1

[plan]
name = "base"
start_year = 2026
horizon_age = 90
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1980-01-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 1000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 10000

[[accounts]]
id = "r"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[income]]
id = "salary"
kind = "salary"
owner = "me"
amount = 100000
end = { date = 2040-01-01 }

[[expenses]]
id = "living"
amount = 50000

[[expenses]]
id = "travel"
amount = 10000
end = { date = 2045-01-01 }

[[conversions]]
id = "ladder"
from = "k"
to = "r"
amount = 5000

[[residency]]
country = "us"
state = "or"
"#;

const HEAD: &str = "schema = 1\nbase = \"base.toml\"\n";

fn scenario(overlay: &str) -> Scenario {
    Scenario::from_toml_str(&format!("{HEAD}{overlay}"))
        .unwrap()
        .expect("a scenario document")
}

fn apply(overlay: &str) -> Result<toml::Table, Vec<Issue>> {
    scenario(overlay).apply(toml::from_str(BASE).unwrap())
}

fn merged(overlay: &str) -> Plan {
    let plan = Plan::from_toml_table(apply(overlay).unwrap()).unwrap();
    let issues = plan.validate();
    assert!(issues.is_empty(), "merged plan invalid: {issues:?}");
    plan
}

#[test]
fn plan_text_is_not_a_scenario() {
    assert!(Scenario::from_toml_str(BASE).unwrap().is_none());
}

#[test]
fn scenario_document_rules() {
    let missing_schema = Scenario::from_toml_str("base = \"plan.toml\"\n");
    assert!(matches!(
        missing_schema,
        Err(ScenarioError::UnsupportedSchema)
    ));
    let bad_base = Scenario::from_toml_str("schema = 1\nbase = 5\n");
    assert!(matches!(bad_base, Err(ScenarioError::InvalidBase)));
    let empty_base = Scenario::from_toml_str("schema = 1\nbase = \"\"\n");
    assert!(matches!(empty_base, Err(ScenarioError::InvalidBase)));
    let parsed = scenario("");
    assert_eq!(parsed.base(), "base.toml");
}

#[test]
fn stated_fields_replace_matched_items() {
    let plan = merged(
        "[[income]]\nid = \"salary\"\nend = { date = 2030-06-01 }\n\
         [[expenses]]\nid = \"travel\"\namount = 20000\n",
    );
    let salary = plan.income_source("salary").unwrap();
    assert_eq!(salary.amount, Some(100_000));
    assert_eq!(salary.end.as_ref().unwrap().date.unwrap().year(), 2030);
    let travel = plan
        .expenses
        .iter()
        .find(|expense| expense.id == "travel")
        .unwrap();
    assert_eq!(travel.amount, 20_000);
    assert!(travel.end.is_some(), "unstated fields must survive");
}

#[test]
fn triggers_replace_as_values() {
    let plan = merged("[[income]]\nid = \"salary\"\nend = { age = 60, owner = \"me\" }\n");
    let end = plan.income_source("salary").unwrap().end.clone().unwrap();
    assert!(end.date.is_none(), "trigger bases must not combine");
    assert_eq!(end.age, Some(60));
}

#[test]
fn unmatched_items_append() {
    let plan =
        merged("[[expenses]]\nid = \"sabbatical\"\namount = 12000\non = { date = 2029-06-01 }\n");
    assert_eq!(plan.expenses.len(), 3);
    assert!(plan.expenses.iter().any(|e| e.id == "sabbatical"));
}

#[test]
fn remove_deletes_and_errors_when_unmatched() {
    let plan = merged("[[expenses]]\nid = \"travel\"\nremove = true\n");
    assert_eq!(plan.expenses.len(), 1);
    let issues = apply("[[expenses]]\nid = \"ghost\"\nremove = true\n").unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.path == "expenses[0]" && issue.message.contains("matched no")),
        "{issues:?}"
    );
}

#[test]
fn replace_substitutes_wholesale() {
    let plan = merged("[[expenses]]\nid = \"travel\"\nreplace = true\namount = 8000\n");
    let travel = plan
        .expenses
        .iter()
        .find(|expense| expense.id == "travel")
        .unwrap();
    assert_eq!(travel.amount, 8000);
    assert!(travel.end.is_none(), "replace must clear unstated fields");
}

#[test]
fn plan_and_household_field_replace() {
    let plan = merged(
        "[plan]\nname = \"retire-2030\"\n\
         [[household.people]]\nid = \"me\"\nbirth = 1981-02-02\n",
    );
    assert_eq!(plan.plan.name.as_deref(), Some("retire-2030"));
    assert_eq!(plan.plan.start_year, 2026, "unstated settings must survive");
    assert_eq!(plan.household.people[0].birth.year(), 1981);
}

#[test]
fn cliffs_merge_by_name_and_medicare_wholesale() {
    let base = format!(
        "{BASE}\n[medicare]\nprior_magi = [80000]\n\n[[cliffs]]\nid = \"aca\"\nmagi_over = 90000\ncost = 12000\n"
    );
    let overlay = "[[cliffs]]\nid = \"aca\"\nmagi_over = 95000\n\n[medicare]\npart_d = false\n";
    let scenario = Scenario::from_toml_str(&format!("{HEAD}{overlay}"))
        .unwrap()
        .expect("a scenario document");
    let merged = scenario.apply(toml::from_str(&base).unwrap()).unwrap();
    let plan = Plan::from_toml_table(merged).unwrap();
    assert!(plan.validate().is_empty(), "{:?}", plan.validate());
    assert_eq!(plan.cliffs[0].magi_over, 95_000, "stated field replaced");
    assert_eq!(plan.cliffs[0].cost, 12_000, "unstated field survives");
    let medicare = plan.medicare.unwrap();
    assert!(!medicare.part_d);
    assert!(
        medicare.prior_magi.is_empty(),
        "unknown tables replace wholesale"
    );
}

#[test]
fn residency_replaces_wholesale() {
    let plan = merged("[[residency]]\ncountry = \"us\"\nstate = \"wa\"\n");
    assert_eq!(plan.residency.len(), 1);
    assert_eq!(plan.residency[0].state.as_deref(), Some("wa"));
}

#[test]
fn unknown_sections_die_at_deserialization() {
    let table = apply("[[incomes]]\nid = \"x\"\n").unwrap();
    let error = Plan::from_toml_table(table).unwrap_err().to_string();
    assert!(error.contains("incomes"), "{error}");
}

#[test]
fn marker_shape_rules() {
    let issues =
        apply("[[expenses]]\nid = \"travel\"\nremove = true\nreplace = true\n").unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.message.contains("exclusive")),
        "{issues:?}"
    );
    let issues = apply("[[expenses]]\nid = \"travel\"\nremove = 1\n").unwrap_err();
    assert!(
        issues
            .iter()
            .any(|issue| issue.path == "expenses[0].remove"),
        "{issues:?}"
    );
    let issues = apply("[[expenses]]\nremove = true\n").unwrap_err();
    assert!(
        issues.iter().any(|issue| issue.message.contains("`id`")),
        "{issues:?}"
    );
}

#[test]
fn overlay_round_trips_canonically() {
    let parsed = scenario("[[expenses]]\nid = \"travel\"\nremove = true\n");
    let text = parsed.to_toml_string().unwrap();
    let reparsed = Scenario::from_toml_str(&text).unwrap().unwrap();
    assert_eq!(parsed, reparsed);
}
