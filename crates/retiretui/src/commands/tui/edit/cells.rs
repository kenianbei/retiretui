//! What a value reads as where it is shown: in a table's cell, and in the
//! text field that edits it. A field parses whatever it shows, so an item
//! opened and applied untouched is the item it was.

use retiretui_engine::plan::Plan;
use toml::{Table, Value};

use super::codec::{get_path, parse_text, share_left, to_text};
use super::domain::{FieldKind, FieldSpec};
use super::group::{nth, nth_back};
use super::offers::{Offer, display_name, ref_offers};
use crate::commands::tui::present;

#[derive(Clone, Copy)]
pub struct Column {
    pub key: &'static str,
    /// What heads the column where the field's label is too long for it.
    header: Option<&'static str>,
    /// A phrase over the whole item, where one key does not say it - so
    /// its key need not name a field.
    pub(super) phrase: Option<fn(&Table, &Plan) -> String>,
}

impl Column {
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            header: None,
            phrase: None,
        }
    }

    pub const fn headed(self, header: &'static str) -> Self {
        Self {
            header: Some(header),
            ..self
        }
    }

    pub const fn phrased(self, phrase: fn(&Table, &Plan) -> String) -> Self {
        Self {
            phrase: Some(phrase),
            ..self
        }
    }

    pub fn header(&self, fields: &[FieldSpec]) -> &'static str {
        let label = || field_of(fields, self.key).map_or(self.key, |spec| spec.label);
        self.header.unwrap_or_else(label)
    }

    /// Whether the column holds numbers, which line up on the right; a
    /// phrase is words whatever its key holds.
    pub fn is_numeric(&self, fields: &[FieldSpec]) -> bool {
        self.phrase.is_none()
            && field_of(fields, self.key).is_some_and(|spec| {
                matches!(
                    spec.kind,
                    FieldKind::Money | FieldKind::Whole | FieldKind::Rate | FieldKind::Share
                )
            })
    }
}

pub(super) fn field_of<'a>(fields: &'a [FieldSpec], key: &str) -> Option<&'a FieldSpec> {
    fields.iter().find(|spec| spec.key == key)
}

/// A table cell: what it says, and the number it says where it is one,
/// which is what its column is ordered by.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Cell {
    pub text: String,
    pub number: Option<f64>,
}

/// What a column's cells are read against, found once per table rather
/// than once per cell.
pub struct Shown<'a> {
    key: &'static str,
    spec: Option<&'a FieldSpec>,
    offers: Vec<Offer>,
    /// What the domain knows its items by, on the column that shows it.
    identity: Option<&'a str>,
    phrase: Option<fn(&Table, &Plan) -> String>,
    fields: &'a [FieldSpec],
}

impl<'a> Shown<'a> {
    pub fn of(column: &Column, fields: &'a [FieldSpec], identity: &'a str, plan: &Plan) -> Self {
        Self::at(column, field_of(fields, column.key), fields, identity, plan)
    }

    /// Read against one field of the item: `spec` itself, which a key
    /// several fields share - a list's places - does not say which.
    pub(super) fn of_field(spec: &'a FieldSpec, fields: &'a [FieldSpec], plan: &Plan) -> Self {
        Self::at(&Column::new(spec.key), Some(spec), fields, "", plan)
    }

    fn at(
        column: &Column,
        spec: Option<&'a FieldSpec>,
        fields: &'a [FieldSpec],
        identity: &'a str,
        plan: &Plan,
    ) -> Self {
        let offers = match spec.map(|spec| spec.kind) {
            Some(FieldKind::Choice(vocabulary) | FieldKind::Order(vocabulary, _)) => {
                vocabulary.offers()
            }
            Some(FieldKind::Ref(source)) => ref_offers(plan, source),
            _ => Vec::new(),
        };
        Self {
            key: column.key,
            spec,
            offers,
            identity: (column.key == identity).then_some(identity),
            phrase: column.phrase,
            fields,
        }
    }

    /// The column an item is known by shows its display name; one with a
    /// phrase of its own says that.
    pub fn cell(&self, item: &Table, plan: &Plan) -> Cell {
        if let Some(identity) = self.identity {
            let text = display_name(item, identity, self.fields).unwrap_or_default();
            return Cell { text, number: None };
        }
        if let Some(phrase) = self.phrase {
            let text = phrase(item, plan);
            return Cell { text, number: None };
        }
        let left;
        let value = if self
            .spec
            .is_some_and(|spec| spec.kind == FieldKind::Remainder)
        {
            left = share_left(item, self.key).map(Value::Float);
            left.as_ref()
        } else {
            get_path(item, self.key)
        };
        let number = value.and_then(|value| match value {
            Value::Integer(whole) => Some(*whole as f64),
            Value::Float(real) => Some(*real),
            _ => None,
        });
        Cell {
            text: self.text(value, plan),
            number,
        }
    }

    /// What `value` reads as in the column: a list's place and an order's
    /// are read out of the whole the item holds.
    pub(super) fn text(&self, value: Option<&Value>, plan: &Plan) -> String {
        let Some(spec) = self.spec else {
            return value.map(to_text).unwrap_or_default();
        };
        let value = match spec.kind {
            FieldKind::Listed(back) => nth_back(value, back),
            FieldKind::Order(_, place) => nth(value, place),
            _ => value,
        };
        let Some(value) = value else {
            return spec.blank.unwrap_or_default().to_owned();
        };
        match spec.kind {
            FieldKind::Flag => ticked(value.as_bool() == Some(true)),
            FieldKind::Presence(_) => ticked(value.is_table()),
            FieldKind::Trigger => present::trigger(value, plan),
            FieldKind::Choice(_) | FieldKind::Ref(_) | FieldKind::Order(..) => self.offered(value),
            kind => field_text(kind, Some(value), false),
        }
    }

    fn offered(&self, value: &Value) -> String {
        let held = value.as_str();
        let offer = self
            .offers
            .iter()
            .find(|offer| Some(offer.value.as_str()) == held);
        offer.map_or_else(|| to_text(value), |offer| offer.label.clone())
    }
}

const TICKED: &str = "✓";

fn ticked(is_ticked: bool) -> String {
    if is_ticked {
        TICKED.to_owned()
    } else {
        String::new()
    }
}

/// The text a field of `kind` shows for `value`. Money is plain digits
/// while typed in, since a separator is one more thing to get wrong.
pub fn field_text(kind: FieldKind, value: Option<&Value>, is_focused: bool) -> String {
    let Some(value) = value else {
        return String::new();
    };
    match (kind, value) {
        (FieldKind::Money | FieldKind::Listed(_), Value::Integer(amount)) if !is_focused => {
            present::money(*amount)
        }
        (FieldKind::Rate | FieldKind::Share | FieldKind::Remainder, Value::Float(rate)) => {
            present::rate(*rate)
        }
        (FieldKind::Rate | FieldKind::Share | FieldKind::Remainder, Value::Integer(rate)) => {
            present::rate(*rate as f64)
        }
        (FieldKind::Growth, value) => present::growth(Some(value)),
        _ => to_text(value),
    }
}

/// The value `text` stands for in a field of `kind`; `None` clears it.
/// Text the kind cannot read is left to the file's own syntax, so the
/// schema is what refuses it.
pub fn parse_field(kind: FieldKind, text: &str) -> Option<Value> {
    let read = match kind {
        FieldKind::Money | FieldKind::Listed(_) => present::parse_money(text).map(Value::Integer),
        FieldKind::Rate | FieldKind::Share => present::parse_rate(text).map(Value::Float),
        FieldKind::Growth => present::parse_growth(text),
        _ => None,
    };
    read.or_else(|| parse_text(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_reads_back_what_it_shows() {
        let cases = [
            (FieldKind::Money, Value::Integer(450_000)),
            (FieldKind::Money, Value::Integer(-5_000)),
            (FieldKind::Rate, Value::Float(0.025)),
            (FieldKind::Growth, Value::Boolean(false)),
            (FieldKind::Growth, Value::Float(0.0125)),
            (FieldKind::Whole, Value::Integer(2030)),
            (FieldKind::Text, Value::String("two words".to_owned())),
        ];
        for (kind, value) in cases {
            for is_focused in [false, true] {
                let shown = field_text(kind, Some(&value), is_focused);
                assert_eq!(parse_field(kind, &shown), Some(value.clone()), "{shown}");
            }
        }
    }

    #[test]
    fn money_is_dressed_until_it_is_typed_in() {
        let amount = Value::Integer(450_000);
        assert_eq!(
            field_text(FieldKind::Money, Some(&amount), false),
            "$450,000"
        );
        assert_eq!(field_text(FieldKind::Money, Some(&amount), true), "450000");
        assert_eq!(
            field_text(FieldKind::Whole, Some(&Value::Integer(2030)), false),
            "2030"
        );
    }

    #[test]
    fn text_a_kind_cannot_read_is_left_to_the_schema() {
        let typed = parse_field(FieldKind::Money, "lots");
        assert_eq!(typed, Some(Value::String("lots".to_owned())));
        assert_eq!(parse_field(FieldKind::Money, "  "), None);
    }
}
