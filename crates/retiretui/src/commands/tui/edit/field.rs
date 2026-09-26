//! What each field kind is edited with: text for values with no closed
//! shape, a select for everything the schema states as a set, and a slider
//! beside exact text for a rate.
//!
//! Nothing here consumes Enter, so it reaches the form and applies the
//! item: a flag's checkbox is bound to space alone rather than to the
//! Enter its activation would otherwise take as well.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Bundle, Changed, Commands, Component, Entity, Has, Query, Ref, Res, With, Without,
};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::Key;
use bevy_input_focus::InputFocus;
use bevy_ui::{Node, UiRect, Val};
use plurimus::core::UiWidget;
use plurimus::ui::{Checked, KeyBinding};
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use plurimus::widgets::{
    ActivateKeys, Checkbox, SliderStep, SliderValue, TextInput, checkbox, checkbox_self_update,
    editable_text, slider,
};
use retiretui_engine::plan::Plan;
use toml::Value;

use super::build::FormField;
use super::cells::{field_text, parse_field};
use super::codec::get_path;
use super::domain::{FieldKind, FieldSpec};
use super::editing::Editing;
use super::group::{nth, nth_back};
use super::select::{Select, spawn_select};
use super::trigger::Slot;
use crate::commands::tui::layout::{placed, sized};
use crate::commands::tui::theme::Theme;

/// A widget activated by space alone, leaving Enter to apply the item.
pub fn space() -> ActivateKeys {
    ActivateKeys(vec![KeyBinding::new(Key::Character(" ".into()))])
}

/// The highest rate a slider offers; a higher one is still typable.
const RATE_CEILING: f32 = 0.15;
const RATE_STEP: f32 = 0.0025;
/// A share's slider reaches the whole in these steps.
const SHARE_CEILING: f32 = 1.0;
const SHARE_STEP: f32 = 0.05;
/// Cells the slider takes, leaving the rest of the field for the text.
const SLIDER_WIDTH: f32 = 20.0;
/// Cells kept clear between the slider and the text beside it.
const SLIDER_GAP: f32 = 1.0;

/// Marks a flag's box, which is a component rather than a value.
fn show_check(entity: Entity, wanted: bool, is_checked: bool, commands: &mut Commands) {
    if wanted == is_checked {
        return;
    }
    let mut widget = commands.entity(entity);
    if wanted {
        widget.insert(Checked);
    } else {
        widget.remove::<Checked>();
    }
}

/// Fills a text field, leaving the caret alone when the value it already
/// shows is the wanted one - which is what stops the caret jumping while
/// the draft moves under a field being typed into.
pub fn show_text(text: &mut TextInput, wanted: String) {
    if text.value() != wanted {
        *text = TextInput::new(wanted);
        text.move_start();
    }
}

/// A widget taking an even share of what its row has left.
pub fn sharing() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_width: Val::Px(0.0),
        height: Val::Px(1.0),
        ..Node::default()
    }
}

/// What is drawn beside a text input and goes where it goes: one of its
/// two brackets, which show it is there while it holds nothing, or a word
/// of the sentence it is part of.
#[derive(Component, Debug)]
pub struct Bracket {
    input: Entity,
    drawn: &'static str,
}

/// The closing one keeps a cell clear of whatever follows it on the row.
const BRACKETS: [&str; 2] = ["[ ", " ] "];

/// `drawn` along `row`, beside `input`, where there is anything to draw.
/// It is ASCII, so bytes are cells.
pub fn spawn_beside(commands: &mut Commands, row: Entity, input: Entity, drawn: &'static str) {
    if drawn.is_empty() {
        return;
    }
    commands.spawn((
        Bracket { input, drawn },
        sized(drawn.len() as f32, 1.0),
        UiWidget::new(Paragraph::new(drawn)),
        placed(),
        ChildOf(row),
    ));
}

/// A text input along `row`, carrying `field`, laid out as `node` between
/// its brackets.
pub fn spawn_text(commands: &mut Commands, row: Entity, node: Node, field: impl Bundle) {
    let input = commands.spawn_empty().id();
    spawn_text_as(commands, row, input, (node, field));
}

/// As [`spawn_text`], of an entity made before its place on the row came.
pub fn spawn_text_as(commands: &mut Commands, row: Entity, input: Entity, field: impl Bundle) {
    spawn_beside(commands, row, input, BRACKETS[0]);
    commands
        .entity(input)
        .insert((editable_text(""), field, placed(), ChildOf(row)));
    spawn_beside(commands, row, input, BRACKETS[1]);
}

/// Brackets are lit as a select's are: dim at rest, and in the focused
/// style while their input holds the keyboard.
pub fn light_brackets(
    focus: Res<InputFocus>,
    theme: Res<Theme>,
    mut brackets: Query<(Ref<Bracket>, &mut UiWidget)>,
) {
    let is_stale = focus.is_changed() || theme.is_changed();
    if !is_stale && !brackets.iter().any(|(bracket, _)| bracket.is_added()) {
        return;
    }
    let lit = theme.ui_theme().focused;
    for (bracket, mut widget) in &mut brackets {
        if !is_stale && !bracket.is_added() {
            continue;
        }
        let style = if focus.get() == Some(bracket.input) {
            lit
        } else {
            theme.dimmed()
        };
        *widget = UiWidget::new(Paragraph::new(bracket.drawn).style(style));
    }
}

/// A bracket takes room only while its input does, which a trigger's
/// operand does not always.
pub fn place_brackets(
    inputs: Query<&Node, (Changed<Node>, Without<Bracket>)>,
    mut brackets: Query<(&Bracket, &mut Node)>,
) {
    for (bracket, mut node) in &mut brackets {
        if let Ok(input) = inputs.get(bracket.input)
            && node.display != input.display
        {
            node.display = input.display;
        }
    }
}

/// Spawns the widgets `spec`'s kind is edited with along `row`, in tab
/// order. How the row is shared between them is the kind's own business.
pub fn spawn_field(commands: &mut Commands, row: Entity, spec: FieldSpec) {
    let field = FormField { spec };
    match spec.kind {
        FieldKind::Text
        | FieldKind::Money
        | FieldKind::Whole
        | FieldKind::Growth
        | FieldKind::Listed(_) => {
            spawn_text(commands, row, sharing(), field);
        }
        FieldKind::Flag | FieldKind::Presence(_) => {
            commands
                .spawn((
                    checkbox(""),
                    space(),
                    field,
                    sharing(),
                    placed(),
                    ChildOf(row),
                ))
                .observe(checkbox_self_update);
        }
        FieldKind::Rate | FieldKind::Share => spawn_slider(commands, row, field),
        FieldKind::Trigger => super::trigger::spawn(commands, row, field),
        kind => {
            let select = Select::new(kind, spec.blank_word()).required(spec.is_required());
            let select = spawn_select(commands, select, field);
            commands.entity(select).insert((sharing(), ChildOf(row)));
        }
    }
}

/// A rate's or a share's slider, and the text beside it that says it
/// exactly.
fn spawn_slider(commands: &mut Commands, row: Entity, field: FormField) {
    let (ceiling, step) = if field.spec.kind == FieldKind::Share {
        (SHARE_CEILING, SHARE_STEP)
    } else {
        (RATE_CEILING, RATE_STEP)
    };
    commands.spawn((
        slider(0.0, ceiling, 0.0),
        SliderStep(step),
        field,
        Node {
            margin: UiRect::right(Val::Px(SLIDER_GAP)),
            ..sized(SLIDER_WIDTH, 1.0)
        },
        placed(),
        ChildOf(row),
    ));
    spawn_text(commands, row, sharing(), field);
}

/// A form's tree, for finding the widgets in it.
#[derive(SystemParam)]
pub struct FormTree<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    editable: Query<'w, 's, &'static FormField>,
}

impl FormTree<'_, '_> {
    /// The key of the field `widget` edits, where it edits one.
    pub fn key_of(&self, widget: Entity) -> Option<&'static str> {
        self.editable.get(widget).ok().map(|field| field.spec.key)
    }

    pub fn first(&self, form: Entity) -> Option<Entity> {
        self.children
            .iter_descendants_depth_first(form)
            .find(|&widget| self.editable.contains(widget))
    }
}

/// Every widget a form's fields are edited with.
#[derive(SystemParam)]
pub struct Fields<'w, 's> {
    pub tree: FormTree<'w, 's>,
    texts: Query<'w, 's, (&'static FormField, &'static mut TextInput), Without<Slot>>,
    selects: Query<'w, 's, (&'static FormField, &'static mut Select), Without<Slot>>,
    sliders: Query<'w, 's, (&'static FormField, &'static mut SliderValue)>,
    checks: Query<'w, 's, (&'static FormField, Has<Checked>), With<Checkbox>>,
    kinds: Query<'w, 's, (&'static FormField, &'static Slot, &'static ChildOf)>,
    slots: super::trigger::SlotsMut<'w, 's>,
    commands: Commands<'w, 's>,
}

impl Fields<'_, '_> {
    /// Fills `form`'s text fields from the item, `focused` as it is typed.
    pub fn show_texts(&mut self, form: Entity, editing: &Editing, focused: Option<Entity>) {
        let widgets: Vec<Entity> = self.tree.children.iter_descendants(form).collect();
        for widget in widgets {
            self.show_text_at(widget, editing, focused == Some(widget));
        }
    }

    /// A field that does not yet make a value keeps its text, since no item
    /// holds it - read as money where it is.
    fn show_text_at(&mut self, widget: Entity, editing: &Editing, is_focused: bool) {
        let Ok((field, mut text)) = self.texts.get_mut(widget) else {
            return;
        };
        if editing.incomplete.contains_key(field.spec.key) {
            // A trigger's operand is typed as the file states it.
            let kind = field.spec.kind;
            let read = parse_field(kind, text.value()).filter(|_| kind != FieldKind::Trigger);
            if let Some(read) = read {
                let own = field_text(kind, Some(&read), is_focused);
                show_text(&mut text, own);
            }
            return;
        }
        let value = get_path(&editing.snapshot, field.spec.key);
        let value = match field.spec.kind {
            FieldKind::Listed(back) => nth_back(value, back),
            _ => value,
        };
        show_text(&mut text, field_text(field.spec.kind, value, is_focused));
    }

    /// Shows each rate on both of its widgets, but for a text being typed
    /// into, which is read back once the keyboard leaves it.
    pub fn show_rates(&mut self, form: Entity, editing: &Editing, focused: Option<Entity>) {
        let widgets: Vec<Entity> = self.tree.children.iter_descendants(form).collect();
        for widget in widgets {
            self.show_slider_at(widget, editing);
            let is_rate = |(field, _): (&FormField, _)| {
                matches!(field.spec.kind, FieldKind::Rate | FieldKind::Share)
            };
            if focused != Some(widget) && self.texts.get(widget).is_ok_and(is_rate) {
                self.show_text_at(widget, editing, false);
            }
        }
    }

    fn show_slider_at(&mut self, widget: Entity, editing: &Editing) {
        if let Ok((field, mut value)) = self.sliders.get_mut(widget) {
            let rate = get_path(&editing.snapshot, field.spec.key).and_then(Value::as_float);
            *value = SliderValue(rate.unwrap_or(0.0) as f32);
        }
    }

    pub fn show(&mut self, form: Entity, editing: &Editing, plan: &Plan, focused: Option<Entity>) {
        let table = &editing.snapshot;
        let widgets: Vec<Entity> = self.tree.children.iter_descendants(form).collect();
        for widget in widgets {
            self.show_text_at(widget, editing, focused == Some(widget));
            self.show_slider_at(widget, editing);
            if let Ok((field, mut select)) = self.selects.get_mut(widget) {
                let value = get_path(table, field.spec.key);
                // An order's place is offered every word again, since what
                // the other places leave it is decided once all are filled.
                let (options, value) = match field.spec.kind {
                    FieldKind::Order(vocabulary, place) => {
                        (Some(vocabulary.offers()), nth(value, place))
                    }
                    _ => (select.referred(plan), value),
                };
                Select::fill(&mut select, options, value);
            }
            if let Ok((field, is_checked)) = self.checks.get(widget) {
                let value = get_path(table, field.spec.key);
                let wanted = match field.spec.kind {
                    FieldKind::Presence(_) => value.is_some_and(Value::is_table),
                    _ => value.and_then(Value::as_bool).unwrap_or(false),
                };
                show_check(widget, wanted, is_checked, &mut self.commands);
            }
            if let Ok((field, Slot::Kind, row)) = self.kinds.get(widget)
                && let Ok(group) = self.tree.children.get(row.parent())
            {
                super::trigger::show_trigger(
                    get_path(table, field.spec.key),
                    plan,
                    group,
                    &mut self.slots,
                );
            }
        }
    }
}

/// A slider's rate reaches the field as a float, rounded to its own step
/// so the text beside it reads as the slider moved.
pub fn slider_value(value: f32) -> Value {
    let stepped = f64::from((value / RATE_STEP).round() * RATE_STEP);
    Value::Float((stepped * 1e6).round() / 1e6)
}
