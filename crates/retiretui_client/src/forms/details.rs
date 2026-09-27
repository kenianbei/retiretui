//! What is read rather than edited: every field of an item in the form's
//! words, then what it keeps that no field edits.

use retiretui_engine::plan::Plan;
use toml::Table;

use super::cells::Shown;
use super::lists::gate_of;
use super::offers::NAME_KEY;
use super::{FieldSpec, Form, ListOps, applies};
use crate::codec::get_path;

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

/// The item's rows: each field it has a use for - shown by its own rule,
/// and by the tick whose table it is in - labelled and phrased as the
/// form phrases it, then the record where the domain keeps one.
#[must_use]
pub fn rows(form: &Form, item: &Table, plan: &Plan) -> Vec<[String; 2]> {
    let opened = applies::opened(form, item);
    let is_used = |spec: &&FieldSpec| {
        let is_held = |gate| get_path(&opened, gate).is_some();
        spec.shown.is_none_or(|shown| shown(&opened))
            && gate_of(form.fields, spec.key).is_none_or(is_held)
    };
    let mut rows: Vec<[String; 2]> = form
        .fields
        .iter()
        .filter(is_used)
        .map(|spec| {
            let shown = Shown::of_field(spec, form.fields, plan);
            [spec.label.to_owned(), shown.cell(&opened, plan).text]
        })
        .collect();
    if let Some(record) = form.list.and_then(|list| list.record) {
        rows.extend(record(item));
    }
    rows
}
