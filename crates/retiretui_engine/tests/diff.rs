//! What one plan changes of another.

mod common;

use retiretui_engine::plan::{Change, ChangeKind, Plan, Scenario, diff};
use toml::{Table, Value};

use common::FULL;

fn full() -> Plan {
    common::plan_from(FULL)
}

/// The full fixture with `overlay` merged over it.
fn over(overlay: &str) -> Plan {
    let text = format!("schema = 1\nbase = \"full.toml\"\n{overlay}");
    let scenario = Scenario::from_toml_str(&text).unwrap().unwrap();
    let merged = scenario.apply(FULL.parse::<Table>().unwrap()).unwrap();
    Plan::from_toml_table(merged).unwrap()
}

fn changes(overlay: &str) -> Vec<Change> {
    diff(&full(), &over(overlay)).unwrap()
}

/// Each change as `section id field kind`, compact enough to compare.
fn described(changes: &[Change]) -> Vec<String> {
    changes
        .iter()
        .map(|change| {
            let kind = match &change.kind {
                ChangeKind::Added => "added".to_owned(),
                ChangeKind::Removed => "removed".to_owned(),
                ChangeKind::Changed { from, to } => {
                    format!("{} -> {}", shown(from.as_ref()), shown(to.as_ref()))
                }
            };
            let place = [
                Some(change.section.as_str()),
                change
                    .item
                    .as_ref()
                    .and_then(|item| item.get("id")?.as_str()),
                change.field.as_deref(),
            ];
            let place: Vec<&str> = place.into_iter().flatten().collect();
            format!("{} {kind}", place.join(" "))
        })
        .collect()
}

fn shown(value: Option<&Value>) -> String {
    value.map_or_else(|| "none".to_owned(), Value::to_string)
}

#[test]
fn identical_plans_differ_in_nothing() {
    assert_eq!(diff(&full(), &full()).unwrap(), Vec::new());
}

#[test]
fn items_are_matched_by_id_and_listed_in_file_order() {
    let found = changes(
        r#"
[[conversions]]
id = "roth-2027"
from = "fid-401k"
to = "roth-ira"
amount = 20000
on = { date = 2027-01-01 }

[[expenses]]
id = "travel"
remove = true

[[income]]
id = "ss-jordan"
start = { age = 70, owner = "jordan" }
"#,
    );
    assert_eq!(
        described(&found),
        [
            r#"income ss-jordan start { age = 67, owner = "jordan" } -> { age = 70, owner = "jordan" }"#,
            "expenses travel removed",
            "conversions roth-2027 added",
        ]
    );
    let ChangeKind::Changed { to: Some(to), .. } = &found[0].kind else {
        panic!("{found:?}");
    };
    assert_eq!(to["age"].as_integer(), Some(70));
}

#[test]
fn a_removed_item_stands_where_it_was_in_the_base() {
    let found = changes(
        r#"
[[accounts]]
id = "brokerage"
remove = true

[[accounts]]
id = "hsa"
balance = 30000

[[accounts]]
id = "pension-dc"
remove = true

[[accounts]]
id = "savings"
kind = "cash"
owner = "alex"
balance = 5000
"#,
    );
    assert_eq!(
        described(&found),
        [
            "accounts brokerage removed",
            "accounts hsa balance 25000 -> 30000",
            "accounts pension-dc removed",
            "accounts savings added",
        ]
    );
}

#[test]
fn a_top_level_table_differs_key_by_key_into_its_tables() {
    let found = changes("[plan]\ninflation = 0.03\n\n[market.stocks]\nmean = 0.07\n");
    assert_eq!(
        described(&found),
        [
            "plan inflation 0.025 -> 0.03",
            "market stocks.mean none -> 0.07"
        ]
    );
}

#[test]
fn a_list_without_ids_differs_as_a_whole() {
    let found = changes(
        "[plan]\nwithdrawal_order = [\"deferred\", \"taxable\", \"roth\", \"hsa\"]\n\n[[residency]]\ncountry = \"us\"\nstate = \"wa\"\n",
    );
    assert_eq!(
        described(&found),
        [
            r#"plan withdrawal_order ["taxable", "deferred", "roth", "hsa"] -> ["deferred", "taxable", "roth", "hsa"]"#,
            r#"residency [{ country = "us", state = "or" }] -> [{ country = "us", state = "wa" }]"#,
        ]
    );
}

#[test]
fn people_are_items_under_the_household() {
    let found = changes(
        "[household]\nfiling = \"single\"\n\n[[household.people]]\nid = \"alex\"\nremove = true\n",
    );
    assert_eq!(
        described(&found),
        [
            r#"household filing "married-joint" -> "single""#,
            "household.people alex removed",
        ]
    );
}
