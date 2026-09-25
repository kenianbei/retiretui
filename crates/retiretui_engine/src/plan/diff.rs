//! How one plan differs from another, read off their canonical TOML: items
//! matched by their id as the overlay merge matches them, tables key by
//! key, and anything else - a list without ids among them - as one value.

use std::collections::BTreeSet;

use toml::{Table, Value};

use super::scenario::{HOUSEHOLD_KEY, ID_KEY, PEOPLE_KEY, PEOPLE_PATH, SCHEMA_KEY, is_keyed};
use super::{Plan, PlanError};

/// The top-level sections in the order a plan file writes them.
const SECTIONS: [&str; 13] = [
    "plan",
    "household",
    "medicare",
    "events",
    "accounts",
    "income",
    "expenses",
    "cliffs",
    "transfers",
    "conversions",
    "contributions",
    "residency",
    "market",
];

/// One way a plan differs from the plan it is measured against.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// The section, as an issue's path names it: `plan`, `income`,
    /// `household.people`.
    pub section: String,
    /// The key within the item, or dotted within the section's table; none
    /// for a whole item, or a section that differs as a whole.
    pub field: Option<String>,
    /// The item it is about, as the plan holding it states it: the other
    /// plan, or the base for an item removed.
    pub item: Option<Table>,
    /// What happened there.
    pub kind: ChangeKind,
}

/// What a [`Change`] is.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    /// An item only the other plan has.
    Added,
    /// An item only the base has.
    Removed,
    /// A value that differs; none on the side that does not state it.
    Changed {
        /// The base's value.
        from: Option<Value>,
        /// The other plan's value.
        to: Option<Value>,
    },
}

/// Every way `other` differs from `base`, in the plan file's section
/// order, a section's items in `other`'s order with each removed one where
/// it stood in `base`.
///
/// # Errors
///
/// Returns [`PlanError`] when either plan fails to serialize or read back.
pub fn diff(base: &Plan, other: &Plan) -> Result<Vec<Change>, PlanError> {
    let (base, other) = (canonical(base)?, canonical(other)?);
    let mut changes = Vec::new();
    for section in sections(&base, &other) {
        let (from, to) = (base.get(section), other.get(section));
        if is_keyed(section) {
            diff_items(&mut changes, section, from, to);
        } else if section == HOUSEHOLD_KEY {
            diff_household(&mut changes, from, to);
        } else {
            diff_fields(&mut changes, (section, None), from, to);
        }
    }
    Ok(changes)
}

fn canonical(plan: &Plan) -> Result<Table, PlanError> {
    Ok(plan.to_toml_string()?.parse()?)
}

/// The tables' sections, those a plan file writes in its order.
fn sections<'a>(base: &'a Table, other: &'a Table) -> Vec<&'a str> {
    let mut sections: Vec<&str> = keys(base, other)
        .into_iter()
        .filter(|&section| section != SCHEMA_KEY)
        .collect();
    let place = |section: &str| SECTIONS.iter().position(|&known| known == section);
    sections.sort_by_key(|&section| place(section).unwrap_or(SECTIONS.len()));
    sections
}

fn keys<'a>(base: &'a Table, other: &'a Table) -> BTreeSet<&'a str> {
    base.keys()
        .chain(other.keys())
        .map(String::as_str)
        .collect()
}

/// Where in a section a value is: its section, and the key within it.
type At<'a> = (&'a str, Option<String>);

/// Tables compare key by key down to what is not a table; a value missing
/// on one side is an empty table there. Anything else compares whole.
fn diff_fields(changes: &mut Vec<Change>, at: At, from: Option<&Value>, to: Option<&Value>) {
    if from == to {
        return;
    }
    let empty = Table::new();
    let (section, field) = at;
    let (Some(from_table), Some(to_table)) = (as_table(from, &empty), as_table(to, &empty)) else {
        changes.push(changed(section, None, field, (from, to)));
        return;
    };
    for key in keys(from_table, to_table) {
        let within = field
            .as_ref()
            .map_or_else(|| key.to_owned(), |outer| format!("{outer}.{key}"));
        diff_fields(
            changes,
            (section, Some(within)),
            from_table.get(key),
            to_table.get(key),
        );
    }
}

fn as_table<'a>(value: Option<&'a Value>, empty: &'a Table) -> Option<&'a Table> {
    value.map_or(Some(empty), Value::as_table)
}

/// The household's own keys, then its people as items.
fn diff_household(changes: &mut Vec<Change>, from: Option<&Value>, to: Option<&Value>) {
    let split = |value: Option<&Value>| {
        let mut own = value.and_then(Value::as_table).cloned().unwrap_or_default();
        let people = own.remove(PEOPLE_KEY);
        (Value::Table(own), people)
    };
    let ((from_own, from_people), (to_own, to_people)) = (split(from), split(to));
    diff_fields(
        changes,
        (HOUSEHOLD_KEY, None),
        Some(&from_own),
        Some(&to_own),
    );
    diff_items(
        changes,
        PEOPLE_PATH,
        from_people.as_ref(),
        to_people.as_ref(),
    );
}

fn items(value: Option<&Value>) -> Vec<&Table> {
    let listed = value.and_then(Value::as_array).map(Vec::as_slice);
    listed
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_table)
        .collect()
}

/// Items matched by id: each of `to`'s added or changed key by key, each
/// of `from`'s it lacks removed right after the item it followed.
fn diff_items(changes: &mut Vec<Change>, section: &str, from: Option<&Value>, to: Option<&Value>) {
    let (base, other) = (items(from), items(to));
    let position = |among: &[&Table], item: &Table| {
        among
            .iter()
            .position(|candidate| candidate.get(ID_KEY) == item.get(ID_KEY))
    };
    // Each removed item under the place of the kept one it followed: 0 is
    // the start, and `at + 1` after the base's item at `at`.
    let mut removed: Vec<Vec<&Table>> = vec![Vec::new(); base.len() + 1];
    let mut after = 0;
    for (at, &item) in base.iter().enumerate() {
        if position(&other, item).is_some() {
            after = at + 1;
        } else {
            removed[after].push(item);
        }
    }
    let mut remove = |after: usize, changes: &mut Vec<Change>| {
        for gone in std::mem::take(&mut removed[after]) {
            changes.push(whole(section, gone, ChangeKind::Removed));
        }
    };
    remove(0, changes);
    for &item in &other {
        match position(&base, item) {
            Some(at) => {
                diff_item(changes, section, base[at], item);
                remove(at + 1, changes);
            }
            None => changes.push(whole(section, item, ChangeKind::Added)),
        }
    }
}

/// An item's keys each compared whole: a trigger or a glide path is one
/// value.
fn diff_item(changes: &mut Vec<Change>, section: &str, from: &Table, to: &Table) {
    for key in keys(from, to) {
        let values = (from.get(key), to.get(key));
        if values.0 != values.1 {
            changes.push(changed(section, Some(to), Some(key.to_owned()), values));
        }
    }
}

fn whole(section: &str, item: &Table, kind: ChangeKind) -> Change {
    Change {
        section: section.to_owned(),
        field: None,
        item: Some(item.clone()),
        kind,
    }
}

fn changed(
    section: &str,
    item: Option<&Table>,
    field: Option<String>,
    (from, to): (Option<&Value>, Option<&Value>),
) -> Change {
    Change {
        section: section.to_owned(),
        field,
        item: item.cloned(),
        kind: ChangeKind::Changed {
            from: from.cloned(),
            to: to.cloned(),
        },
    }
}
