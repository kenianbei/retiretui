//! Fields that mean something only beside others: rows shown while the
//! table they sit in is present, and rows that hold one list between them.

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Changed, Component, Entity, Or, Query, Ref, Res, With, Without};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui::{Display, Node};
use toml::Value;

use super::build::{FormField, FormFields};
use super::codec::get_path;
use super::domain::{FieldKind, FieldSpec};
use super::editing::{EditSession, Editing};
use super::form::FormTargets;
use super::select::Select;
use crate::layout::{NO_STOP, set_display};
use retiretui_client::forms::lists::{gate_of, unused};

/// A row shown only while the item has a use for its field.
#[derive(Component, Debug)]
pub struct Dependent {
    key: &'static str,
    /// The key of the table a tick stands for, which the field is in.
    gate: Option<&'static str>,
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
        self.gate.is_none_or(is_held) && editing.is_on_show(self.key)
    }
}

/// A dependent row takes room only while the item being edited has a use
/// for it.
pub fn place_dependents(
    session: Res<EditSession>,
    columns: Query<&ChildOf, With<FormFields>>,
    mut rows: Query<(Ref<Dependent>, &ChildOf, &mut Node)>,
) {
    if !session.is_changed() && !rows.iter().any(|(row, ..)| row.is_added()) {
        return;
    }
    let Some((editing, form)) = session.0.as_ref().and_then(|held| Some((held, held.form?))) else {
        return;
    };
    for (dependent, column, mut node) in &mut rows {
        if columns
            .get(column.parent())
            .is_ok_and(|held| held.parent() == form)
        {
            set_display(&mut node, dependent.is_shown(editing));
        }
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
/// none can be picked twice.
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
        let others: Vec<Option<Value>> = held
            .iter()
            .filter(|(other, in_form, key, _)| {
                *other != entity && *in_form == form && *key == field.spec.key
            })
            .map(|(.., value)| value.clone())
            .collect();
        let own = select.value();
        let offers = unused(vocabulary, own.as_ref(), &others);
        Select::fill(&mut select, Some(offers), own.as_ref());
    }
}
