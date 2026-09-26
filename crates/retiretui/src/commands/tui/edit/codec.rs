//! Field text is TOML value syntax, so structured values keep the plan
//! file's own grammar and the schema's own error messages. A bare word that
//! is not a TOML value is a string, which is what most identifiers are.

use serde::Serialize;
use serde::de::DeserializeOwned;
use toml::{Table, Value};

const PROBE_KEY: &str = "v";

/// The text a field shows for `value`.
pub fn to_text(value: &Value) -> String {
    match value {
        Value::String(text) if is_bare_string(text) => text.clone(),
        other => other.to_string(),
    }
}

/// A string shows unquoted when it is not itself a TOML value, so reading
/// it back bare gives the same string.
fn is_bare_string(text: &str) -> bool {
    !text.is_empty() && parse_value(text).is_none()
}

/// The value `text` stands for; `None` clears the field.
pub fn parse_text(text: &str) -> Option<Value> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    Some(parse_value(text).unwrap_or_else(|| Value::String(text.to_owned())))
}

fn parse_value(text: &str) -> Option<Value> {
    let probe: Table = format!("{PROBE_KEY} = {text}").parse().ok()?;
    probe.get(PROBE_KEY).cloned()
}

/// What parts a key that reaches into a table the item holds.
const KEY_SEPARATOR: char = '.';

/// What opens an index into a list, in an issue's path.
const INDEX_OPEN: char = '[';

/// What closes an index into a list, in an issue's path.
const INDEX_CLOSE: char = ']';

/// The place `path` names in the list at `key`, where it ends in one:
/// `2` of `plan.withdrawal_order[2]`.
pub fn list_place(path: &str, key: &str) -> Option<usize> {
    let (list, digits) = path.strip_suffix(INDEX_CLOSE)?.rsplit_once(INDEX_OPEN)?;
    list.ends_with(key).then(|| digits.parse().ok())?
}

/// Whether `key` reaches into the table - or, in an issue's path, the
/// list - at `outer`.
pub fn is_within(key: &str, outer: &str) -> bool {
    let deeper = key.strip_prefix(outer);
    deeper.is_some_and(|rest| rest.starts_with([KEY_SEPARATOR, INDEX_OPEN]))
}

/// The value at `key`, which may reach into a table - `contributions.start`
/// - or, by a number, into a list of them: `allocation.0.from`.
pub fn get_path<'a>(table: &'a Table, key: &str) -> Option<&'a Value> {
    match key.split_once(KEY_SEPARATOR) {
        None => table.get(key),
        Some((outer, inner)) => get_within(table.get(outer)?, inner),
    }
}

fn get_within<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Array(items) = value else {
        return get_path(value.as_table()?, key);
    };
    let (at, inner) = key.split_once(KEY_SEPARATOR)?;
    get_path(items.get(at.parse::<usize>().ok()?)?.as_table()?, inner)
}

/// Writes the value at `key`, or clears it. A table is made for the first
/// value it holds and leaves with its last, so none is ever kept empty. A
/// list reached by number grows to hold the place written, and loses only
/// empty places at its end, so a cleared place keeps the ones after it
/// where they are. Writing into a value of the other shape replaces it.
pub fn set_path(table: &mut Table, key: &str, value: Option<Value>) {
    let Some((outer, inner)) = key.split_once(KEY_SEPARATOR) else {
        match value {
            Some(value) => table.insert(key.to_owned(), value),
            None => table.remove(key),
        };
        return;
    };
    let wants_list = inner
        .split_once(KEY_SEPARATOR)
        .is_some_and(|(at, _)| at.parse::<usize>().is_ok());
    let fits = |held: &Value| held.is_array() == wants_list && (held.is_array() || held.is_table());
    if value.is_none() && !table.get(outer).is_some_and(fits) {
        return;
    }
    let held = table.entry(outer).or_insert_with(|| empty(wants_list));
    if !fits(held) {
        *held = empty(wants_list);
    }
    let is_empty = match held {
        Value::Array(items) => set_in_list(items, inner, value),
        Value::Table(held) => {
            set_path(held, inner, value);
            held.is_empty()
        }
        _ => false,
    };
    if is_empty {
        table.remove(outer);
    }
}

fn empty(is_list: bool) -> Value {
    if is_list {
        Value::Array(Vec::new())
    } else {
        Value::Table(Table::new())
    }
}

/// Writes `key`, a place and a key within it, into `items`; whether the
/// list is left empty.
fn set_in_list(items: &mut Vec<Value>, key: &str, value: Option<Value>) -> bool {
    let Some((at, inner)) = key.split_once(KEY_SEPARATOR) else {
        return items.is_empty();
    };
    let Ok(at) = at.parse::<usize>() else {
        return items.is_empty();
    };
    if value.is_some() && items.len() <= at {
        items.resize_with(at + 1, || Value::Table(Table::new()));
    }
    if let Some(Value::Table(place)) = items.get_mut(at) {
        set_path(place, inner, value);
    }
    while items
        .last()
        .is_some_and(|last| last.as_table().is_some_and(Table::is_empty))
    {
        items.pop();
    }
    items.is_empty()
}

/// An item as the table its fields edit. Both directions go through TOML
/// text: the in-memory value (de)serializers do not carry native dates.
pub fn to_table<T: Serialize>(item: &T) -> Table {
    toml::to_string(item)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_default()
}

/// The item a table describes, or the schema's complaint.
///
/// # Errors
///
/// The deserialization message, trimmed of its TOML position prefix.
pub fn from_table<T: DeserializeOwned>(table: Table) -> Result<T, String> {
    let text = toml::to_string(&table).map_err(|error| error.to_string())?;
    toml::from_str(&text).map_err(|error: toml::de::Error| {
        error
            .message()
            .lines()
            .next()
            .unwrap_or("invalid value")
            .to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_words_are_strings_and_values_are_values() {
        assert_eq!(parse_text("me"), Some(Value::String("me".to_owned())));
        assert_eq!(parse_text("401k"), Some(Value::String("401k".to_owned())));
        assert_eq!(parse_text("2026"), Some(Value::Integer(2026)));
        assert_eq!(parse_text("true"), Some(Value::Boolean(true)));
        assert_eq!(parse_text("  "), None);
        let trigger = parse_text("{ age = 60, owner = \"me\" }").unwrap();
        assert_eq!(trigger.get("age"), Some(&Value::Integer(60)));
        assert_eq!(
            parse_text("\"2026\""),
            Some(Value::String("2026".to_owned()))
        );
    }

    #[test]
    fn text_round_trips_through_the_value() {
        for text in [
            "me",
            "401k",
            "2026",
            "true",
            "{ age = 60, owner = \"me\" }",
            "\"2026\"",
        ] {
            let value = parse_text(text).unwrap();
            assert_eq!(parse_text(&to_text(&value)), Some(value), "{text}");
        }
        assert_eq!(to_text(&Value::String("two words".to_owned())), "two words");
        assert_eq!(to_text(&Value::String("2026".to_owned())), "\"2026\"");
    }

    #[test]
    fn a_dotted_key_makes_its_table_and_the_last_value_cleared_removes_it() {
        let mut item: Table = "id = \"a\"".parse().unwrap();
        set_path(&mut item, "held.first", Some(Value::Integer(1)));
        set_path(&mut item, "held.second", Some(Value::Integer(2)));
        assert_eq!(get_path(&item, "held.second"), Some(&Value::Integer(2)));
        assert_eq!(get_path(&item, "held.third"), None);
        assert_eq!(get_path(&item, "absent.first"), None);
        set_path(&mut item, "held.first", None);
        assert!(item.contains_key("held"), "it still holds a value");
        set_path(&mut item, "held.second", None);
        assert!(!item.contains_key("held"), "and now holds none");
        set_path(&mut item, "absent.first", None);
        assert_eq!(item.len(), 1, "clearing makes no table: {item}");
        set_path(&mut item, "id", None);
        assert!(item.is_empty(), "a plain key is a plain key");
    }

    #[test]
    fn a_numbered_key_reaches_into_a_list_and_keeps_its_places() {
        let mut item = Table::new();
        set_path(&mut item, "steps.1.share", Some(Value::Float(0.5)));
        assert_eq!(get_path(&item, "steps.1.share"), Some(&Value::Float(0.5)));
        assert_eq!(
            item["steps"].as_array().map(Vec::len),
            Some(2),
            "place 0 held open"
        );
        set_path(&mut item, "steps.0.share", Some(Value::Float(0.2)));
        set_path(&mut item, "steps.0.share", None);
        assert_eq!(
            get_path(&item, "steps.1.share"),
            Some(&Value::Float(0.5)),
            "place 1 stays"
        );
        set_path(&mut item, "steps.1.share", None);
        assert!(
            !item.contains_key("steps"),
            "the last place cleared removes the list"
        );
        set_path(&mut item, "steps.share", Some(Value::Float(0.1)));
        set_path(&mut item, "steps.0.share", Some(Value::Float(0.3)));
        assert!(item["steps"].is_array(), "writing a place replaces a table");
        set_path(&mut item, "steps.share", None);
        assert!(
            item["steps"].is_array(),
            "clearing a key the list lacks leaves it"
        );
    }

    #[test]
    fn dates_stay_native_toml_dates() {
        let person: retiretui_engine::plan::Person =
            toml::from_str("id = \"a\"\nbirth = 1980-01-02").unwrap();
        let table = to_table(&person);
        assert_eq!(to_text(&table["birth"]), "1980-01-02");
        assert_eq!(
            from_table::<retiretui_engine::plan::Person>(table),
            Ok(person)
        );
    }

    #[test]
    fn tables_surface_the_schemas_error() {
        let mut table = Table::new();
        table.insert("id".to_owned(), Value::String("me".to_owned()));
        let error = from_table::<retiretui_engine::plan::Person>(table).unwrap_err();
        assert!(error.contains("birth"), "{error}");
    }
}
