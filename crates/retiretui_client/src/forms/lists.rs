//! Fields that mean something only beside others: rows shown while the
//! table they sit in is present, and rows that hold one list between them.

use toml::{Table, Value};

use super::offers::{Offer, Vocabulary};
use super::{FieldKind, FieldSpec};
use crate::codec::{get_path, is_within};

/// The key of the tick among `fields` whose table the field `key` is in.
#[must_use]
pub fn gate_of(fields: &[FieldSpec], key: &str) -> Option<&'static str> {
    let is_gate =
        |gate: &&FieldSpec| matches!(gate.kind, FieldKind::Presence(_)) && is_within(key, gate.key);
    fields.iter().find(is_gate).map(|gate| gate.key)
}

/// Whether the tick the field `key` sits under, where it sits under one,
/// holds its table in `item`.
#[must_use]
pub fn is_gate_open(fields: &[FieldSpec], key: &str, item: &Table) -> bool {
    gate_of(fields, key).is_none_or(|gate| get_path(item, gate).is_some())
}

/// What a place of an order over `vocabulary` offers: the words no other
/// place holds - those `is_held` says another place holds - so none can be
/// picked twice. One the file stated twice is left where it is, since `own`
/// is always offered.
#[must_use]
pub fn unused(
    vocabulary: Vocabulary,
    own: Option<&Value>,
    is_held: impl Fn(&Value) -> bool,
) -> Vec<Offer> {
    let is_taken = |offer: &Offer| {
        let word = Value::String(offer.value.clone());
        Some(&word) != own && is_held(&word)
    };
    let offers = vocabulary.offers().into_iter();
    offers.filter(|offer| !is_taken(offer)).collect()
}

const EMPTY_ORDER: &str = "needs at least one of its rows picked";

/// The order `parts` make by their places, blanks closed up, or the complaint
/// where all are blank: an absent order is the schema's default, not none.
#[must_use]
pub fn ordered(
    parts: impl Iterator<Item = (usize, Option<Value>)>,
) -> (Option<Value>, Option<&'static str>) {
    let mut held: Vec<(usize, Value)> = parts
        .filter_map(|(place, value)| Some((place, value?)))
        .collect();
    if held.is_empty() {
        return (None, Some(EMPTY_ORDER));
    }
    held.sort_by_key(|(place, _)| *place);
    let order = held.into_iter().map(|(_, value)| value).collect();
    (Some(Value::Array(order)), None)
}

const GAPPED_LIST: &str = "is blank, but the row after it is not";

/// The list `parts` make, each placed by how far from its end it sits, or the
/// complaint where a place nearer the end than a filled one is blank.
#[must_use]
pub fn listed(
    parts: impl Iterator<Item = (usize, Option<Value>)>,
) -> (Option<Value>, Option<&'static str>) {
    let mut held: Vec<(usize, Value)> = parts
        .filter_map(|(back, value)| Some((back, value?)))
        .collect();
    held.sort_by_key(|(back, _)| *back);
    let is_gapped = held.iter().enumerate().any(|(at, (back, _))| at != *back);
    if is_gapped {
        return (None, Some(GAPPED_LIST));
    }
    let list: Vec<Value> = held.into_iter().rev().map(|(_, value)| value).collect();
    ((!list.is_empty()).then_some(Value::Array(list)), None)
}

#[cfg(test)]
mod tests {
    use super::super::cells::nth_back;
    use super::*;

    fn amounts(list: &[i64]) -> Value {
        Value::Array(list.iter().copied().map(Value::Integer).collect())
    }

    #[test]
    fn a_list_is_read_and_made_from_its_end() {
        let list = amounts(&[180_000, 185_000]);
        assert_eq!(nth_back(Some(&list), 0), Some(&Value::Integer(185_000)));
        assert_eq!(nth_back(Some(&list), 1), Some(&Value::Integer(180_000)));
        assert_eq!(nth_back(Some(&list), 2), None);
        assert_eq!(nth_back(None, 0), None);
        let recent = Some(Value::Integer(185_000));
        let before = Some(Value::Integer(180_000));
        let both = [(0, recent.clone()), (1, before.clone())];
        assert_eq!(listed(both.into_iter()), (Some(list), None), "oldest first");
        let alone = [(0, recent), (1, None)];
        assert_eq!(listed(alone.into_iter()), (Some(amounts(&[185_000])), None));
        assert_eq!(listed([(0, None), (1, None)].into_iter()), (None, None));
        let gapped = [(0, None), (1, before)];
        assert_eq!(listed(gapped.into_iter()), (None, Some(GAPPED_LIST)));
    }
}
