use toml::{Table, Value};

use super::validate::push_issue;
use super::{Issue, PlanError, SCHEMA_VERSION};

pub(super) const SCHEMA_KEY: &str = "schema";
const BASE_KEY: &str = "base";

/// Errors from reading a scenario document.
#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    /// The TOML failed to parse.
    #[error(transparent)]
    Parse(#[from] toml::de::Error),
    /// `base` is present but not a non-empty string.
    #[error("`base` must be a non-empty string")]
    InvalidBase,
    /// `schema` is missing or not this build's version.
    #[error("`schema` is required and must be {SCHEMA_VERSION}")]
    UnsupportedSchema,
}

/// A scenario overlay document: a reference to a base document plus deltas
/// applied over it. Reading the referenced base and following chains is the
/// caller's job; the engine only merges parsed tables.
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    base: String,
    document: Table,
}

impl Scenario {
    /// Parses a scenario from TOML text. Returns `Ok(None)` when the
    /// document has no `base` key, meaning it is a plan rather than a
    /// scenario.
    ///
    /// # Errors
    ///
    /// Returns [`ScenarioError`] when the text is not valid TOML, `base` is
    /// not a non-empty string, or `schema` is missing or unsupported.
    pub fn from_toml_str(text: &str) -> Result<Option<Self>, ScenarioError> {
        let document: Table = toml::from_str(text)?;
        let Some(base_value) = document.get(BASE_KEY) else {
            return Ok(None);
        };
        let base = base_value
            .as_str()
            .filter(|base| !base.is_empty())
            .ok_or(ScenarioError::InvalidBase)?
            .to_owned();
        let schema = document.get(SCHEMA_KEY).and_then(Value::as_integer);
        if schema != Some(i64::from(SCHEMA_VERSION)) {
            return Err(ScenarioError::UnsupportedSchema);
        }
        Ok(Some(Self { base, document }))
    }

    /// The base document this scenario overlays, as written in the file.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Serializes the overlay document to canonical TOML. The app owns the
    /// format: comments and layout of the source file are not preserved.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError::Serialize`] when serialization fails.
    pub fn to_toml_string(&self) -> Result<String, PlanError> {
        Ok(toml::to_string_pretty(&self.document)?)
    }

    /// Applies the overlay to a parsed base document and returns the merged
    /// table. `[plan]` and `household` field-replace (people merge by `id`);
    /// item arrays merge by identity - their `id` - where
    /// stated fields replace the base item's, `remove = true` deletes,
    /// `replace = true` substitutes wholesale, and an unmatched fragment
    /// appends; `residency` replaces wholesale. Unknown sections copy
    /// through, to be rejected when the result deserializes into a plan.
    ///
    /// # Errors
    ///
    /// Returns every merge problem found: a `remove` that matched nothing, a
    /// non-boolean or contradictory marker, a marker on an item lacking its
    /// section's identity field, or a malformed overlay section.
    pub fn apply(&self, mut base: Table) -> Result<Table, Vec<Issue>> {
        let mut issues = Vec::new();
        for (section, value) in &self.document {
            match section.as_str() {
                SCHEMA_KEY | BASE_KEY => {}
                "plan" | "market" => merge_fields(&mut base, section, value, &mut issues),
                HOUSEHOLD_KEY => merge_household(&mut base, value, &mut issues),
                _ if is_keyed(section) => {
                    let items = ItemSection {
                        name: section,
                        path: section,
                    };
                    merge_items(&mut base, &items, value, &mut issues);
                }
                _ => {
                    base.insert(section.clone(), value.clone());
                }
            }
        }
        if issues.is_empty() {
            Ok(base)
        } else {
            Err(issues)
        }
    }
}

/// The field every keyed item is matched on.
pub const ID_KEY: &str = "id";

/// The table holding the household's keyed people, and their key within it.
pub(super) const HOUSEHOLD_KEY: &str = "household";
pub(super) const PEOPLE_KEY: &str = "people";
pub(super) const PEOPLE_PATH: &str = "household.people";

/// Whether the top-level `section` is a list of items matched by [`ID_KEY`].
pub(super) fn is_keyed(section: &str) -> bool {
    matches!(
        section,
        "events"
            | "accounts"
            | "income"
            | "expenses"
            | "cliffs"
            | "transfers"
            | "conversions"
            | "contributions"
    )
}

/// An identity-keyed item array being merged: where it sits in its containing
/// table, and the issue-path prefix.
struct ItemSection<'a> {
    name: &'a str,
    path: &'a str,
}

fn merge_fields(base: &mut Table, section: &str, overlay: &Value, issues: &mut Vec<Issue>) {
    let Some(overlay_table) = overlay.as_table() else {
        push_issue(issues, section, "must be a table");
        return;
    };
    let target_value = base
        .entry(section)
        .or_insert_with(|| Value::Table(Table::new()));
    let Some(target) = target_value.as_table_mut() else {
        *target_value = overlay.clone();
        return;
    };
    target.extend(overlay_table.clone());
}

fn merge_household(base: &mut Table, overlay: &Value, issues: &mut Vec<Issue>) {
    let Some(overlay_table) = overlay.as_table() else {
        push_issue(issues, HOUSEHOLD_KEY, "must be a table");
        return;
    };
    let mut scalars = overlay_table.clone();
    let people = scalars.remove(PEOPLE_KEY);
    merge_fields(base, HOUSEHOLD_KEY, &Value::Table(scalars), issues);
    let Some(people) = people else {
        return;
    };
    let Some(target) = base.get_mut(HOUSEHOLD_KEY).and_then(Value::as_table_mut) else {
        return;
    };
    let section = ItemSection {
        name: PEOPLE_KEY,
        path: PEOPLE_PATH,
    };
    merge_items(target, &section, &people, issues);
}

fn merge_items(
    target: &mut Table,
    items: &ItemSection<'_>,
    overlay: &Value,
    issues: &mut Vec<Issue>,
) {
    let Some(overlay_items) = overlay.as_array() else {
        push_issue(issues, items.path, "must be an array of tables");
        return;
    };
    let target_value = target
        .entry(items.name)
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(base_items) = target_value.as_array_mut() else {
        *target_value = overlay.clone();
        return;
    };
    for (index, item) in overlay_items.iter().enumerate() {
        let path = format!("{}[{index}]", items.path);
        merge_item(base_items, &path, item, issues);
    }
}

fn merge_item(base_items: &mut Vec<Value>, path: &str, item: &Value, issues: &mut Vec<Issue>) {
    let Some(table) = item.as_table() else {
        push_issue(issues, path, "must be a table");
        return;
    };
    let mut fields = table.clone();
    let remove = take_marker(&mut fields, "remove", path, issues);
    let replace = take_marker(&mut fields, "replace", path, issues);
    if remove && replace {
        push_issue(issues, path, "`remove` and `replace` are exclusive");
        return;
    }
    let key = fields
        .get(ID_KEY)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let Some(key) = key else {
        if remove || replace {
            push_issue(
                issues,
                path,
                format!("a marker needs `{ID_KEY}` to address an item"),
            );
        } else {
            base_items.push(Value::Table(fields));
        }
        return;
    };
    let matched = base_items.iter().position(|candidate| {
        candidate
            .as_table()
            .and_then(|table| table.get(ID_KEY))
            .and_then(Value::as_str)
            == Some(key.as_str())
    });
    match matched {
        Some(position) if remove => {
            base_items.remove(position);
        }
        Some(position) if replace => base_items[position] = Value::Table(fields),
        Some(position) => merge_matched(&mut base_items[position], fields),
        None if remove => push_issue(
            issues,
            path,
            format!("`remove = true` matched no item with {ID_KEY} `{key}`"),
        ),
        None => base_items.push(Value::Table(fields)),
    }
}

fn merge_matched(target: &mut Value, fields: Table) {
    match target.as_table_mut() {
        Some(table) => table.extend(fields),
        None => *target = Value::Table(fields),
    }
}

fn take_marker(fields: &mut Table, marker: &str, path: &str, issues: &mut Vec<Issue>) -> bool {
    match fields.remove(marker) {
        None => false,
        Some(Value::Boolean(flag)) => flag,
        Some(_) => {
            push_issue(issues, format!("{path}.{marker}"), "must be a boolean");
            false
        }
    }
}
