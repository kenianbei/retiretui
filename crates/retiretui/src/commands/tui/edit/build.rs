//! A domain's form as a tree: one row per field down a column, each row a
//! label beside whatever its field kind edits the value with, and the
//! buttons that apply or discard the item.

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity};
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::UiWidget;
use plurimus::widgets::button;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;

use super::domain::{FieldSpec, Ops};
use super::field;
use super::form::{handle_button, handle_form_key};
use super::group::{Dependent, gate_of};
use crate::commands::tui::layout::{
    Emphasis, button_node, fixed, growing, placed, rule, sized, spawn_button_row,
};
use crate::commands::tui::overlay::{self, Centred};
use crate::commands::tui::scope::KeyScope;

/// The form over one item of a domain.
#[derive(Component)]
pub struct EditForm {
    pub ops: Ops,
}

/// A widget editing the item's field `spec`. Its form is the one it sits
/// in.
#[derive(Component, Clone, Copy)]
#[require(KeyScope = KeyScope::Plain)]
pub struct FormField {
    pub spec: FieldSpec,
}

/// The two actions at the foot of a form.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
#[require(KeyScope = KeyScope::Plain)]
pub enum FormButton {
    Discard,
    Apply,
}

/// Columns a form takes, borders included: beside the longest label, a
/// trigger's longest sentence with a pick of 15 in it.
pub const FORM_COLS: u16 = 67;

/// Cells kept after the longest label: a gap, the issue mark, a gap.
const LABEL_MARGIN: u16 = 3;

/// Rows a form keeps for its help, between the rules of its foot.
const HELP_ROWS: u16 = 3;

/// Rows the foot takes beside the help: a rule either side, the buttons.
const FOOT_ROWS: u16 = 3;

/// Columns a form's frame takes of its width: a border each side.
const FRAME_COLS: usize = 2;

/// Cells a wrapped row may leave unused where a word would not fit.
const WRAP_SLACK: usize = 10;

/// Whether every field of `ops` says what it means in the rows a form
/// keeps for that; help is ASCII, so bytes are cells.
pub const fn help_fits(ops: Ops) -> bool {
    let room = (FORM_COLS as usize - FRAME_COLS - WRAP_SLACK) * HELP_ROWS as usize;
    let mut at = 0;
    while at < ops.fields.len() {
        let spec = ops.fields[at];
        if spec.help.is_empty() || spec.help.len() > room {
            return false;
        }
        at += 1;
    }
    true
}

/// The box a form standing alone is centred in.
pub fn centred(ops: Ops) -> (Node, Centred) {
    overlay::centred(FORM_COLS, rows_of(ops))
}

/// The rows a form's contents take inside its frame: its fields and its
/// foot.
pub(super) const fn rows_of(ops: Ops) -> u16 {
    (ops.fields.len() as u16).saturating_add(HELP_ROWS + FOOT_ROWS)
}

/// What is drawn before the label of a field in a table a tick stands
/// for, ruling the table's rows under the tick; a cell in from the form's
/// own border, which it would otherwise run into.
const GUTTER: &str = " │ ";
const GUTTER_COLS: u16 = 3;

fn gutter_cols(fields: &[FieldSpec], spec: &FieldSpec) -> u16 {
    gate_of(fields, spec.key).map_or(0, |_| GUTTER_COLS)
}

/// The column every label of `fields` fits in, behind its gutter where it
/// has one. Labels are ASCII, so bytes are cells.
fn label_cols(fields: &[FieldSpec]) -> u16 {
    let cols = fields
        .iter()
        .map(|spec| spec.label.len() as u16 + gutter_cols(fields, spec));
    cols.max().unwrap_or(0).saturating_add(LABEL_MARGIN)
}

/// A field's label, which says so where the draft holds an issue against
/// the field.
#[derive(Component, Clone, Copy)]
pub struct FieldLabel {
    pub key: &'static str,
    pub label: &'static str,
    /// Whether the label is drawn marked, so it is redrawn only when that
    /// moves.
    pub is_marked: bool,
}

/// Where a form says what its focused field means.
#[derive(Component)]
pub struct HelpFoot;

/// Makes `form` the form over `ops`: a row per field, and the buttons that
/// apply or discard the item - `is_alone` where it stands with nothing to
/// cancel back to.
pub fn spawn_form(commands: &mut Commands, form: Entity, ops: Ops, is_alone: bool) {
    let label_cols = label_cols(ops.fields);
    commands
        .entity(form)
        .insert((EditForm { ops }, TabGroup::modal()))
        .observe(handle_form_key);
    for &spec in ops.fields {
        let gutter = gutter_cols(ops.fields, &spec);
        let row = spawn_row(commands, form, spec, gutter, label_cols - gutter);
        if let Some(dependent) = Dependent::of(ops.fields, &spec) {
            commands.entity(row).insert(dependent);
        }
    }
    commands.spawn((growing(), ChildOf(form)));
    commands.spawn((rule(), ChildOf(form)));
    commands.spawn((
        HelpFoot,
        fixed(f32::from(HELP_ROWS)),
        UiWidget::new(Paragraph::new("")),
        placed(),
        ChildOf(form),
    ));
    commands.spawn((rule(), ChildOf(form)));
    spawn_buttons(commands, form, ops.actions, is_alone);
}

/// One field of a form: its label in `label_cols` behind a gutter of
/// `gutter` cells, and the widget its kind is entered by.
fn spawn_row(
    commands: &mut Commands,
    form: Entity,
    spec: FieldSpec,
    gutter: u16,
    label_cols: u16,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                ..fixed(1.0)
            },
            ChildOf(form),
        ))
        .id();
    if gutter > 0 {
        commands.spawn((
            sized(f32::from(gutter), 1.0),
            UiWidget::new(Paragraph::new(GUTTER)),
            placed(),
            ChildOf(row),
        ));
    }
    commands.spawn((
        FieldLabel {
            key: spec.key,
            label: spec.label,
            is_marked: false,
        },
        sized(f32::from(label_cols), 1.0),
        UiWidget::new(Paragraph::new(spec.label)),
        placed(),
        ChildOf(row),
    ));
    let value = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                min_width: Val::Px(0.0),
                ..growing()
            },
            ChildOf(row),
        ))
        .id();
    field::spawn_field(commands, value, spec);
    row
}

/// The buttons at the form's foot; a form standing alone has nothing to
/// discard back to, so only the one that applies.
fn spawn_buttons(
    commands: &mut Commands,
    form: Entity,
    actions: [&'static str; 2],
    is_alone: bool,
) {
    let row = spawn_button_row(commands, form);
    for (which, label) in [FormButton::Discard, FormButton::Apply]
        .into_iter()
        .zip(actions)
        .filter(|(which, _)| !(is_alone && *which == FormButton::Discard))
    {
        let mut spawned = commands.spawn((
            button(label),
            which,
            button_node(label),
            placed(),
            ChildOf(row),
        ));
        spawned.observe(handle_button);
        if which == FormButton::Apply {
            spawned.insert(Emphasis::Primary);
        }
    }
}
