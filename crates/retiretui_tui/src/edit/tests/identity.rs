//! How a listed item is known: by a name stated first, then an id.

use retiretui_engine::plan::ID_KEY;
use toml::Value;

use super::screens;
use crate::support;

#[test]
fn every_item_known_by_an_id_states_its_name_first_and_the_id_second() {
    let keyed = screens().filter(|ops| ops.list.is_some_and(|list| list.identity == ID_KEY));
    for ops in keyed {
        let first: Vec<&str> = ops.fields.iter().take(2).map(|spec| spec.key).collect();
        assert_eq!(first, ["name", ID_KEY], "{}", ops.title);
    }
}

#[test]
fn a_new_item_is_given_the_lowest_id_its_kind_leaves_free() {
    let mut plan = support::test_projected().plan;
    let blank_of = |singular: &str, plan: &_| {
        let list = screens()
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
