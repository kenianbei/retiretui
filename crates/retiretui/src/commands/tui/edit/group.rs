//! Fields that mean something only beside others: rows shown while the
//! table they sit in is present, and rows that hold one list between them.

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Changed, Component, Entity, Or, Query, Ref, Res, With, Without};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui::{Display, Node};
use toml::Value;

use super::build::FormField;
use super::codec::{get_path, is_within};
use super::domain::{FieldKind, FieldSpec};
use super::editing::{EditSession, Editing};
use super::form::FormTargets;
use super::offers::Offer;
use super::select::Select;
use crate::commands::tui::layout::{NO_STOP, set_display};

/// A row shown only while the item has a use for its field.
#[derive(Component, Debug)]
pub struct Dependent {
    key: &'static str,
    /// The key of the table a tick stands for, which the field is in.
    gate: Option<&'static str>,
}

/// The key of the tick among `fields` whose table the field `key` is in.
pub fn gate_of(fields: &[FieldSpec], key: &str) -> Option<&'static str> {
    let is_gate =
        |gate: &&FieldSpec| matches!(gate.kind, FieldKind::Presence(_)) && is_within(key, gate.key);
    fields.iter().find(is_gate).map(|gate| gate.key)
}

impl Dependent {
    /// What the row of `spec` depends on, where it does: the tick whose
    /// table its key reaches into, and what the field is itself shown by.
    pub fn of(fields: &[FieldSpec], spec: &FieldSpec) -> Option<Self> {
        let gate = gate_of(fields, spec.key);
        let key = spec.key;
        (gate.is_some() || spec.shown.is_some()).then_some(Self { key, gate })
    }

    fn is_shown(&self, editing: &Editing) -> bool {
        let is_held = |gate| get_path(&editing.snapshot, gate).is_some();
        self.gate.is_none_or(is_held)
            && editing.uses(self.key)
            && !editing.cleared.contains(&self.key)
    }
}

/// A dependent row takes room only while the item being edited has a use
/// for it.
pub fn place_dependents(
    session: Res<EditSession>,
    tree: Query<&Children>,
    mut rows: Query<(Ref<Dependent>, &mut Node)>,
) {
    if !session.is_changed() && !rows.iter().any(|(row, _)| row.is_added()) {
        return;
    }
    let Some((editing, form)) = session.0.as_ref().and_then(|held| Some((held, held.form?))) else {
        return;
    };
    for &row in tree.get(form).into_iter().flatten() {
        let Ok((dependent, mut node)) = rows.get_mut(row) else {
            continue;
        };
        set_display(&mut node, dependent.is_shown(editing));
    }
}

/// A field's widget is a stop of the tab order only while it is on show:
/// itself, which a trigger's unused operand is not, and the row it is in.
/// Nothing else writes a field's place in the order.
pub fn place_stops(
    moved: Query<(), (Changed<Node>, Or<(With<FormField>, With<Dependent>)>)>,
    mut stops: Query<(Entity, &Node, &mut TabIndex), (With<FormField>, Without<Dependent>)>,
    parents: Query<&ChildOf>,
    rows: Query<&Node, With<Dependent>>,
) {
    if moved.is_empty() {
        return;
    }
    let is_hidden = |node: &Node| node.display == Display::None;
    for (widget, own, mut tab) in &mut stops {
        let mut above = parents.iter_ancestors(widget);
        let is_shown = !is_hidden(own) && !above.any(|row| rows.get(row).is_ok_and(is_hidden));
        tab.set_if_neq(TabIndex(if is_shown { 0 } else { NO_STOP }));
    }
}

/// Each place of an order offers the words no other place of it holds, so
/// none can be picked twice. One the file stated twice is left where it is.
pub fn offer_unused(mut selects: Query<(Entity, &FormField, &mut Select)>, targets: FormTargets) {
    let is_place = |field: &FormField| matches!(field.spec.kind, FieldKind::Order(..));
    let mut places = selects.iter_mut();
    if !places.any(|(_, field, select)| is_place(field) && select.is_changed()) {
        return;
    }
    let held: Vec<(Entity, Option<Entity>, &str, Option<Value>)> = selects
        .iter()
        .filter(|(_, field, _)| is_place(field))
        .map(|(entity, field, select)| {
            (
                entity,
                targets.form_of(entity),
                field.spec.key,
                select.value(),
            )
        })
        .collect();
    for (entity, field, mut select) in &mut selects {
        let FieldKind::Order(vocabulary, _) = field.spec.kind else {
            continue;
        };
        let form = targets.form_of(entity);
        let own = select.value();
        let is_taken = |offer: &Offer| {
            let word = Some(Value::String(offer.value.clone()));
            word != own
                && held.iter().any(|(other, in_form, key, value)| {
                    *other != entity && *in_form == form && *key == field.spec.key && *value == word
                })
        };
        let unused = vocabulary
            .offers()
            .into_iter()
            .filter(|offer| !is_taken(offer));
        Select::fill(&mut select, Some(unused.collect()), own.as_ref());
    }
}

const EMPTY_ORDER: &str = "needs at least one of its rows picked";

/// The order `parts` make by their places, blanks closed up, or the complaint
/// where all are blank: an absent order is the schema's default, not none.
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

/// The entry of the list `value` at `place`, counted from its start.
pub fn nth(value: Option<&Value>, place: usize) -> Option<&Value> {
    value?.as_array()?.get(place)
}

/// The entry of the list `value` that sits `back` places from its end.
pub fn nth_back(value: Option<&Value>, back: usize) -> Option<&Value> {
    let list = value?.as_array()?;
    list.get(list.len().checked_sub(back + 1)?)
}

/// The list `parts` make, each placed by how far from its end it sits, or the
/// complaint where a place nearer the end than a filled one is blank.
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
