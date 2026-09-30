//! One plan stated over another as a scenario overlay: the inverse of
//! [`Scenario::apply`], read off both plans' canonical tables.

use std::collections::BTreeSet;

use toml::{Table, Value};

use super::scenario::{
    BASE_KEY, HOUSEHOLD_KEY, ID_KEY, PEOPLE_KEY, REMOVE_KEY, REPLACE_KEY, SCHEMA_KEY, is_keyed,
};
use super::validate::push_issue;
use super::{Issue, SCHEMA_VERSION, Scenario};

/// The merge replaces what an overlay states and deletes only items, so
/// nothing else the base states can be cleared.
const CLEARED: &str = "a scenario cannot clear what its base states";

/// One section as the plan beneath, the draft, and the overlay last
/// written each hold it.
#[derive(Clone, Copy)]
struct Sides<'a> {
    from: Option<&'a Value>,
    to: Option<&'a Value>,
    kept: Option<&'a Value>,
}

impl Scenario {
    /// The overlay naming `base` that [`Scenario::apply`] merges over the
    /// canonical plan table `beneath` into `draft`. Whatever `kept` states
    /// that `draft` still holds is stated again, so a value the overlay
    /// pinned stays pinned while the base agrees with it.
    ///
    /// # Errors
    ///
    /// Returns an issue at each path where `draft` clears what `beneath`
    /// states outside an item - a key of `[plan]`, the household or
    /// `[market]`, or a whole section - since no overlay can state that.
    pub fn over(
        base: &str,
        beneath: &Table,
        draft: &Table,
        kept: Option<&Self>,
    ) -> Result<Self, Vec<Issue>> {
        let mut issues = Vec::new();
        let mut document = Table::new();
        let schema = Value::Integer(SCHEMA_VERSION.into());
        document.insert(SCHEMA_KEY.to_owned(), schema);
        document.insert(BASE_KEY.to_owned(), Value::String(base.to_owned()));
        for section in sections(beneath, draft) {
            let sides = Sides {
                from: beneath.get(section),
                to: draft.get(section),
                kept: kept.and_then(|kept| kept.document.get(section)),
            };
            if let Some(stated) = state_section(section, sides, &mut issues) {
                document.insert(section.to_owned(), stated);
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        let base = base.to_owned();
        Ok(Self { base, document })
    }
}

fn sections<'a>(beneath: &'a Table, draft: &'a Table) -> BTreeSet<&'a str> {
    (beneath.keys().chain(draft.keys()))
        .map(String::as_str)
        .filter(|&section| section != SCHEMA_KEY)
        .collect()
}

fn state_section(section: &str, sides: Sides<'_>, issues: &mut Vec<Issue>) -> Option<Value> {
    match section {
        "plan" | "market" => state_fields(section, sides, issues).map(Value::Table),
        HOUSEHOLD_KEY => state_household(sides, issues),
        _ if is_keyed(section) => state_items(sides),
        _ => state_whole(section, sides, issues),
    }
}

fn at<'a>(side: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    side.and_then(|value| value.get(key))
}

/// A table whose keys the merge replaces whole: each key the draft
/// changed, and each the overlay stated at the value the draft holds.
fn state_fields(path: &str, sides: Sides<'_>, issues: &mut Vec<Issue>) -> Option<Table> {
    let Some(to) = sides.to.and_then(Value::as_table) else {
        if sides.from.is_some() {
            push_issue(issues, path, CLEARED);
        }
        return None;
    };
    let beneath = sides.from.and_then(Value::as_table);
    for key in beneath.into_iter().flat_map(Table::keys) {
        if !to.contains_key(key) {
            push_issue(issues, format!("{path}.{key}"), CLEARED);
        }
    }
    let stated: Table = (to.iter())
        .filter(|&(key, value)| {
            at(sides.from, key) != Some(value) || at(sides.kept, key) == Some(value)
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    (!stated.is_empty()).then_some(stated)
}

/// The household's own fields as a table's, and its people as items.
fn state_household(sides: Sides<'_>, issues: &mut Vec<Issue>) -> Option<Value> {
    let without_people = |side: Option<&Value>| {
        let mut table = side.and_then(Value::as_table)?.clone();
        table.remove(PEOPLE_KEY);
        Some(Value::Table(table))
    };
    let (from, to) = (without_people(sides.from), without_people(sides.to));
    let kept = without_people(sides.kept);
    let own = Sides {
        from: from.as_ref(),
        to: to.as_ref(),
        kept: kept.as_ref(),
    };
    let mut stated = state_fields(HOUSEHOLD_KEY, own, issues).unwrap_or_default();
    let people = Sides {
        from: at(sides.from, PEOPLE_KEY),
        to: at(sides.to, PEOPLE_KEY),
        kept: at(sides.kept, PEOPLE_KEY),
    };
    if let Some(people) = state_items(people) {
        stated.insert(PEOPLE_KEY.to_owned(), people);
    }
    (!stated.is_empty()).then_some(Value::Table(stated))
}

fn items(side: Option<&Value>) -> &[Value] {
    side.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn id_of(item: &Value) -> Option<&str> {
    item.get(ID_KEY).and_then(Value::as_str)
}

fn is_same_item(item: &Value, other: &Value) -> bool {
    id_of(item).is_some_and(|id| id_of(other) == Some(id))
}

/// An item list as the merge applies it: the merge keeps the items it
/// matches where they stood and appends the rest, so the draft's leading
/// items still in their order beneath are stated where they changed, every
/// other item beneath is removed, and the rest of the draft is appended
/// whole - an item moved being removed and appended again.
fn state_items(sides: Sides<'_>) -> Option<Value> {
    let (from, to, kept) = (items(sides.from), items(sides.to), items(sides.kept));
    let (settled, appended) = to.split_at(settled_len(from, to));
    let mut stated: Vec<Value> = (settled.iter())
        .filter_map(|item| state_item(item, from, kept))
        .collect();
    let removed = from
        .iter()
        .filter(|beneath| !settled.iter().any(|item| is_same_item(item, beneath)));
    stated.extend(removed.filter_map(removal));
    stated.extend(appended.iter().cloned());
    (!stated.is_empty()).then_some(Value::Array(stated))
}

/// How many of the draft's leading items are beneath in the same order.
fn settled_len(from: &[Value], to: &[Value]) -> usize {
    let mut last = None;
    let is_in_order = |item: &&Value| {
        let place = from.iter().position(|beneath| is_same_item(beneath, item));
        let is_after = place.is_some_and(|at| last.is_none_or(|last| at > last));
        if is_after {
            last = place;
        }
        is_after
    };
    to.iter().take_while(is_in_order).count()
}

fn removal(item: &Value) -> Option<Value> {
    let mut marker = Table::new();
    marker.insert(ID_KEY.to_owned(), Value::String(id_of(item)?.to_owned()));
    marker.insert(REMOVE_KEY.to_owned(), Value::Boolean(true));
    Some(Value::Table(marker))
}

/// A matched item's keys the draft changed or the overlay pinned, or the
/// whole item substituted where the draft cleared a key it had.
fn state_item(item: &Value, from: &[Value], kept: &[Value]) -> Option<Value> {
    let to = item.as_table()?;
    let beneath = from.iter().find(|beneath| is_same_item(beneath, item))?;
    let beneath = beneath.as_table()?;
    if beneath.keys().any(|key| !to.contains_key(key)) {
        let mut whole = to.clone();
        whole.insert(REPLACE_KEY.to_owned(), Value::Boolean(true));
        return Some(Value::Table(whole));
    }
    let is_removal = |fragment: &Value| fragment.get(REMOVE_KEY) == Some(&Value::Boolean(true));
    let pinned =
        (kept.iter().rev()).find(|fragment| is_same_item(fragment, item) && !is_removal(fragment));
    let mut stated = Table::new();
    stated.insert(ID_KEY.to_owned(), Value::String(id_of(item)?.to_owned()));
    stated.extend(
        (to.iter())
            .filter(|&(key, value)| {
                key != ID_KEY && (beneath.get(key) != Some(value) || at(pinned, key) == Some(value))
            })
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    (stated.len() > 1).then_some(Value::Table(stated))
}

/// A section the merge inserts whole: stated where the draft changed it or
/// the overlay stated it as the draft holds it. A plan writes no empty
/// list, so a list the draft emptied is stated empty.
fn state_whole(section: &str, sides: Sides<'_>, issues: &mut Vec<Issue>) -> Option<Value> {
    let emptied = Value::Array(Vec::new());
    let to = match (sides.to, sides.from) {
        (Some(to), _) => to,
        (None, Some(Value::Array(_))) => &emptied,
        (None, Some(_)) => {
            push_issue(issues, section, CLEARED);
            return None;
        }
        (None, None) => return None,
    };
    (sides.from != Some(to) || sides.kept == Some(to)).then(|| to.clone())
}
