//! What is read rather than edited: every field of an item in the form's
//! words, then what it keeps that no field edits.

use retiretui_engine::plan::Plan;
use serde::Serialize;
use toml::Table;

use super::cells::Shown;
use super::lists::is_gate_open;
use super::offers::NAME_KEY;
use super::{FieldSpec, Form, ListOps, applies};

/// Whether the domain's items say more than its columns show: a field no
/// column shows, or a record. The identity is known to the table whether
/// or not a column shows it, and a column showing the identity shows the
/// name in its place.
#[must_use]
pub fn has_details(form: &Form, list: ListOps) -> bool {
    let is_column = |key: &str| list.columns.iter().any(|column| column.key == key);
    let shows_name = is_column(list.identity);
    let is_known = |key: &str| key == list.identity || (shows_name && key == NAME_KEY);
    list.record.is_some()
        || form
            .fields
            .iter()
            .any(|spec| !is_known(spec.key) && !is_column(spec.key))
}

/// A row of an item read out: a field's label and what it holds.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadRow {
    /// What the form calls the field.
    pub label: String,
    /// What it holds, in the form's words.
    pub text: String,
    /// Whether that is what an empty field stands for rather than a value
    /// the plan states.
    pub is_unstated: bool,
    /// The heading the field is gathered under, where it is.
    pub group: Option<&'static str>,
}

/// The item's rows: each field it has a use for - shown by its own rule,
/// and by the tick whose table it is in - labelled and phrased as the
/// form phrases it, then the record where the domain keeps one.
#[must_use]
pub fn rows(form: &Form, item: &Table, plan: &Plan) -> Vec<ReadRow> {
    let opened = applies::opened(form, item);
    let is_used = |spec: &&FieldSpec| {
        spec.is_shown_for(&opened) && is_gate_open(form.fields, spec.key, &opened)
    };
    let mut rows: Vec<ReadRow> = form
        .fields
        .iter()
        .filter(is_used)
        .map(|spec| {
            let cell = Shown::of_field(spec, form.fields, plan).cell(&opened, plan);
            ReadRow {
                label: spec.label.to_owned(),
                text: cell.text,
                is_unstated: cell.is_unstated,
                group: spec.group,
            }
        })
        .collect();
    if let Some(record) = form.list.and_then(|list| list.record) {
        rows.extend(record(item).into_iter().map(|[label, text]| ReadRow {
            label,
            text,
            is_unstated: false,
            group: None,
        }));
    }
    rows
}
