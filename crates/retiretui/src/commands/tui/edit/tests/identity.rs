//! How a listed item is known: by an id stated first, then a name.

use retiretui_engine::plan::ID_KEY;
use toml::Value;

use super::SCREENS;
use crate::commands::tui::support;

#[test]
fn every_item_known_by_an_id_states_it_first_and_its_name_second() {
    let keyed = SCREENS
        .iter()
        .filter(|ops| ops.list.is_some_and(|list| list.identity == ID_KEY));
    for ops in keyed {
        let first: Vec<&str> = ops.fields.iter().take(2).map(|spec| spec.key).collect();
        assert_eq!(first, [ID_KEY, "name"], "{}", ops.title);
    }
}

#[test]
fn a_new_item_is_given_the_lowest_id_its_kind_leaves_free() {
    let mut plan = support::test_projected().plan;
    let blank_of = |singular: &str, plan: &_| {
        let list = SCREENS
            .iter()
            .find_map(|ops| ops.list.filter(|list| list.singular == singular))
            .expect("the domain");
        (list.blank)(plan).get(ID_KEY).cloned()
    };
    let seeded = |id: &str| Some(Value::String(id.to_owned()));
    assert_eq!(blank_of("Expense", &plan), seeded("expense-1"));
    assert_eq!(blank_of("Income Source", &plan), seeded("income-1"));
    let mut taken = plan.expenses[0].clone();
    taken.id = "expense-1".to_owned();
    plan.expenses.push(taken);
    assert_eq!(blank_of("Expense", &plan), seeded("expense-2"));
    assert_eq!(
        blank_of("Residency", &plan),
        None,
        "a country is its identity"
    );
}
