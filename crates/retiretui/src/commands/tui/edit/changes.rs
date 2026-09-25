//! What one plan changes of another, in the words an issue is read in:
//! the page, the item by its display name and the field's label, then
//! what happened - each value as the form shows it.

use retiretui_engine::plan::{Change, ChangeKind, Plan};
use toml::Value;

use super::cells::Shown;
use super::codec::to_text;
use super::domain::{BLANK, FieldSpec};
use super::{Located, PLACE_SEPARATOR, locate, place_words};

const ADDED: &str = "added";
const REMOVED: &str = "removed";
const CHANGED: &str = "changed";
const ARROW: &str = " → ";

/// `change`, its old value read against `base` and its new against
/// `other`, where it names an account or a person.
pub fn change_words(change: &Change, (base, other): (&Plan, &Plan)) -> String {
    let path = match &change.field {
        Some(field) => format!("{}.{field}", change.section),
        None => change.section.clone(),
    };
    let located = locate(&path);
    let mut place = located.as_ref().map_or_else(
        || vec![path.clone()],
        |located| place_words(located, change.item.as_ref()),
    );
    let spec = located.as_ref().and_then(|located| spec_at(located, &path));
    if let (None, Some(field)) = (spec, &change.field) {
        place.push(field.clone());
    }
    let what = match &change.kind {
        ChangeKind::Added => ADDED.to_owned(),
        ChangeKind::Removed => REMOVED.to_owned(),
        ChangeKind::Changed { .. } if change.field.is_none() => CHANGED.to_owned(),
        ChangeKind::Changed { from, to } => {
            let words = |value: &Option<Value>, plan| {
                value_words(located.as_ref(), spec, value.as_ref(), plan)
            };
            format!("{}{ARROW}{}", words(from, base), words(to, other))
        }
    };
    format!("{}: {what}", place.join(PLACE_SEPARATOR))
}

/// The field `path` is exactly, rather than one it reaches into.
fn spec_at(located: &Located, path: &str) -> Option<&'static FieldSpec> {
    located.field.filter(|spec| path.ends_with(spec.key))
}

/// `value` as `spec` shows it - every place of a list several fields
/// share - or as the file spells it where no field shows it.
fn value_words(
    located: Option<&Located>,
    spec: Option<&FieldSpec>,
    value: Option<&Value>,
    plan: &Plan,
) -> String {
    let (Some(located), Some(spec)) = (located, spec) else {
        return value.map_or_else(|| BLANK.to_owned(), to_text);
    };
    let fields = located.ops.fields;
    let sharing = fields.iter().filter(|field| field.key == spec.key);
    let shown: Vec<String> = sharing
        .map(|field| Shown::of_field(field, fields, plan).text(value, plan))
        .filter(|text| !text.is_empty())
        .collect();
    if !shown.is_empty() {
        return shown.join(", ");
    }
    value.map_or_else(|| spec.blank_word().to_owned(), to_text)
}
