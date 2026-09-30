//! A plan stated as an overlay over another resolves back to it exactly.

use std::fs;
use std::path::{Path, PathBuf};

use retiretui_engine::plan::{Issue, Plan, Scenario, to_table};
use toml::value::Datetime;
use toml::{Table, Value};

const BASE: &str = "base.toml";

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// An example plan beneath its draft: a plan over itself, or a scenario's
/// base resolved beneath the scenario, with the scenario as last written.
struct Example {
    name: String,
    beneath: Table,
    draft: Table,
    kept: Option<Scenario>,
}

fn canonical(table: Table) -> Table {
    to_table(&Plan::from_toml_table(table).unwrap()).unwrap()
}

fn read_example(path: &Path) -> Example {
    let text = fs::read_to_string(path).unwrap();
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let Some(scenario) = Scenario::from_toml_str(&text).unwrap() else {
        let plan = canonical(text.parse().unwrap());
        return Example {
            name,
            beneath: plan.clone(),
            draft: plan,
            kept: None,
        };
    };
    let base_text = fs::read_to_string(examples().join(scenario.base())).unwrap();
    let beneath = canonical(base_text.parse().unwrap());
    let draft = canonical(scenario.apply(beneath.clone()).unwrap());
    Example {
        name,
        beneath,
        draft,
        kept: Some(scenario),
    }
}

fn every_example() -> Vec<Example> {
    let mut paths: Vec<PathBuf> = fs::read_dir(examples())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect();
    paths.sort();
    paths.iter().map(|path| read_example(path)).collect()
}

/// The overlay written, read back as a save's text is, and merged.
fn resolved(beneath: &Table, draft: &Table, kept: Option<&Scenario>) -> Plan {
    let overlay = Scenario::over(BASE, beneath, draft, kept).unwrap();
    let text = overlay.to_toml_string().unwrap();
    let read = Scenario::from_toml_str(&text).unwrap().unwrap();
    assert_eq!(read.base(), BASE);
    Plan::from_toml_table(read.apply(beneath.clone()).unwrap()).unwrap()
}

fn array<'a>(plan: &'a mut Table, section: &str) -> Option<&'a mut Vec<Value>> {
    plan.get_mut(section)?.as_array_mut()
}

/// Each edit a draft makes, applied where the plan has what it edits.
type Edit = fn(&mut Table) -> bool;

const EDITS: [(&str, Edit); 9] = [
    ("a setting changed", |plan| {
        plan["plan"]["inflation"] = Value::Float(0.0317);
        true
    }),
    ("a market key changed", |plan| {
        let Some(market) = plan.get_mut("market").and_then(Value::as_table_mut) else {
            return false;
        };
        market.insert("leave_at_least".to_owned(), Value::Integer(4242));
        true
    }),
    ("an item added", |plan| {
        let added: Table = "id = \"added\"\namount = 1234".parse().unwrap();
        let expenses = plan
            .entry("expenses")
            .or_insert_with(|| Value::Array(Vec::new()));
        expenses.as_array_mut().unwrap().push(Value::Table(added));
        true
    }),
    ("an item removed", |plan| {
        array(plan, "expenses").is_some_and(|expenses| {
            expenses.remove(0);
            true
        })
    }),
    ("an optional item field cleared", |plan| {
        let Some(incomes) = array(plan, "income") else {
            return false;
        };
        let ended = incomes.iter_mut().find_map(|income| {
            income
                .as_table_mut()
                .and_then(|income| income.remove("end"))
        });
        ended.is_some()
    }),
    ("an item moved to the end", |plan| {
        array(plan, "accounts").is_some_and(|accounts| {
            let first = accounts.remove(0);
            accounts.push(first);
            accounts.len() > 1
        })
    }),
    ("items reversed", |plan| {
        array(plan, "accounts").is_some_and(|accounts| {
            accounts.reverse();
            accounts.len() > 1
        })
    }),
    ("a person's field changed", |plan| {
        let birth: Datetime = "1971-02-03".parse().unwrap();
        plan["household"]["people"][0]["birth"] = Value::Datetime(birth);
        true
    }),
    ("residency emptied", |plan| {
        plan.remove("residency").is_some()
    }),
];

#[test]
fn every_example_resolves_back_to_itself() {
    for example in every_example() {
        let draft = Plan::from_toml_table(example.draft.clone()).unwrap();
        let kept = example.kept.as_ref();
        assert_eq!(
            resolved(&example.beneath, &example.draft, kept),
            draft,
            "{}",
            example.name
        );
    }
}

#[test]
fn every_edit_to_every_example_resolves_to_the_draft() {
    let mut applied = 0;
    for example in every_example() {
        for (edit_name, edit) in EDITS {
            let mut draft = example.draft.clone();
            if !edit(&mut draft) {
                continue;
            }
            applied += 1;
            let draft = canonical(draft);
            let expected = Plan::from_toml_table(draft.clone()).unwrap();
            let got = resolved(&example.beneath, &draft, example.kept.as_ref());
            assert_eq!(got, expected, "{edit_name} on {}", example.name);
        }
    }
    assert!(applied > 50, "only {applied} edits applied");
}

fn issues_of(beneath: &Table, draft: &Table) -> Vec<Issue> {
    Scenario::over(BASE, beneath, draft, None).unwrap_err()
}

fn example_named(name: &str) -> Example {
    read_example(&examples().join(name))
}

#[test]
fn clearing_what_the_base_states_outside_an_item_is_refused_where_it_is() {
    let example = example_named("market-mix.toml");
    let paths = |edit: fn(&mut Table)| {
        let mut draft = example.draft.clone();
        edit(&mut draft);
        let issues = issues_of(&example.beneath, &draft);
        issues
            .into_iter()
            .map(|issue| issue.path)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        paths(|draft| {
            draft["plan"].as_table_mut().unwrap().remove("name");
        }),
        ["plan.name"]
    );
    assert_eq!(paths(|draft| drop(draft.remove("market"))), ["market"]);
    let medicare = example_named("retired-couple.toml");
    let mut draft = medicare.draft.clone();
    assert!(
        draft.remove("medicare").is_some(),
        "the example states medicare"
    );
    let issues = issues_of(&medicare.beneath, &draft);
    assert_eq!(issues[0].path, "medicare");
    assert_eq!(
        issues[0].message,
        "a scenario cannot clear what its base states"
    );
}

fn stated(overlay: &Scenario) -> Table {
    overlay.to_toml_string().unwrap().parse().unwrap()
}

#[test]
fn what_the_overlay_pinned_is_stated_again_while_the_draft_holds_it() {
    let example = example_named("starter.toml");
    let inflation = example.beneath["plan"]["inflation"].clone();
    let pinned = format!("schema = 1\nbase = \"{BASE}\"\n[plan]\ninflation = {inflation}\n");
    let kept = Scenario::from_toml_str(&pinned).unwrap().unwrap();
    let again = Scenario::over(BASE, &example.beneath, &example.draft, Some(&kept)).unwrap();
    assert_eq!(stated(&again)["plan"]["inflation"], inflation);
    let fresh = Scenario::over(BASE, &example.beneath, &example.draft, None).unwrap();
    assert!(!stated(&fresh).contains_key("plan"), "nothing differs");
    let mut changed = example.draft.clone();
    changed["plan"]["inflation"] = Value::Float(0.05);
    let moved = Scenario::over(BASE, &example.beneath, &changed, Some(&kept)).unwrap();
    assert_eq!(stated(&moved)["plan"]["inflation"], Value::Float(0.05));
}

#[test]
fn an_example_scenario_restates_what_it_stated() {
    for example in every_example() {
        let Some(kept) = &example.kept else {
            continue;
        };
        let again = Scenario::over(BASE, &example.beneath, &example.draft, Some(kept)).unwrap();
        let (before, after) = (stated(kept), stated(&again));
        for section in before.keys().filter(|&section| section != "base") {
            assert!(after.contains_key(section), "{section} of {}", example.name);
        }
    }
}
