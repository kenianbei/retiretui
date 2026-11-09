//! What each field kind is edited with: text for values with no closed
//! shape, a select for everything the schema states as a set, and a slider
//! beside exact text for a rate.
//!
//! Nothing here consumes Enter, so it reaches the form and applies the
//! item.

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut, Mut, Ref};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Bundle, Changed, Commands, Component, Entity, Has, Query, Res, With, Without,
};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::InputFocus;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui::{Node, UiRect, Val};
use plurimus::bui::bevy_text::{EditableText, TextColor, TextEdit};
use plurimus::bui::bevy_ui_widgets::SliderValue;
use plurimus::bui::bevy_ui_widgets::TextInput as FieldInput;
use plurimus::core::UiArea;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::Checked;
use retiretui_engine::plan::Plan;
use toml::Value;

use super::build::FormField;
use super::cells::{field_text, parse_field};
use super::codec::{get_path, share_left};
use super::domain::{FieldKind, FieldSpec};
use super::editing::{EditSession, Editing};
use super::flag::{self, FlagBox};
use super::select::{Select, spawn_select};
use super::slider;
use super::trigger::Slot;
use crate::layout::{placed, sized};
use crate::present;
use crate::theme::{Theme, say, text};
use retiretui_client::forms::cells::{nth, nth_back};
use retiretui_client::forms::offers::{candidates, offered};

/// Where a remainder's text starts: level with the shares' text, past
/// their slider and bracket.
const REMAINDER_INDENT: f32 = slider::WIDTH as f32 + slider::GAP + BRACKETS[0].len() as f32;
/// What a remainder is drawn with, and the share it last drew.
#[derive(Component, Default)]
pub struct Remaining {
    left: Option<f64>,
}

/// A remainder shows what the shares beside it leave, marked where that
/// is less than none or more than the whole.
pub fn show_remainders(
    session: Res<EditSession>,
    theme: Res<Theme>,
    mut shown: Query<(Entity, &mut Remaining, &FormField)>,
    mut commands: Commands,
) {
    if !session.is_changed() && !theme.is_changed() {
        return;
    }
    let Some(editing) = session.0.as_ref() else {
        return;
    };
    for (entity, mut remaining, field) in &mut shown {
        let left = share_left(editing.snapshot(), field.spec.key);
        if remaining.left == left && !theme.is_changed() {
            continue;
        }
        remaining.left = left;
        let style = if left.is_some_and(|left| (0.0..=1.0).contains(&left)) {
            Style::default()
        } else {
            theme.exceeded()
        };
        let said = left.map(present::rate).unwrap_or_default();
        say(&mut commands, entity, Line::styled(said, style), &theme);
    }
}

/// Fills a text field, leaving the caret alone when the value it already
/// shows is the wanted one - which is what stops the caret jumping while
/// the draft moves under a field being typed into.
pub fn show_text(text: &mut Mut<EditableText>, shown: &mut ShownText, wanted: String) {
    if text.value() != wanted.as_str() {
        text.editor_mut().set_text(&wanted);
        text.queue_edit(TextEdit::TextStart(false));
    }
    shown.0 = wanted;
}

/// Every text field's text is in the theme's ink, in either skin: a form's
/// and the file picker's.
pub fn ink_fields(theme: Res<Theme>, mut fields: Query<(Ref<FieldInput>, &mut TextColor)>) {
    for (spawned, mut colour) in &mut fields {
        if theme.is_changed() || spawned.is_added() {
            colour.set_if_neq(TextColor(theme.ink(Style::default())));
        }
    }
}

/// What the form last showed in a field or read back from it, so that what
/// the form writes there is not read back as typed.
#[derive(Component, Debug, Default)]
pub struct ShownText(pub(crate) String);

/// A widget taking an even share of what its row has left.
pub fn sharing() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: Val::ZERO,
        min_width: Val::ZERO,
        height: Val::Rem(1.0),
        ..Node::default()
    }
}

/// What is drawn beside a text input and goes where it goes: one of its
/// two brackets, which show it is there while it holds nothing, or a word
/// of the sentence it is part of; and whether it was last drawn lit.
#[derive(Component, Debug)]
pub struct Bracket {
    input: Entity,
    drawn: &'static str,
    drawn_lit: Option<bool>,
}

/// The closing one keeps a cell clear of whatever follows it on the row.
const BRACKETS: [&str; 2] = ["[ ", " ] "];

impl Bracket {
    /// Whether this is one of the two brackets rather than a word.
    #[cfg(any(feature = "desktop", test))]
    pub(crate) fn is_bracket(&self) -> bool {
        BRACKETS.contains(&self.drawn)
    }
}

/// `drawn` along `row`, beside `input`, where there is anything to draw.
/// It is ASCII, so bytes are cells.
pub fn spawn_beside(commands: &mut Commands, row: Entity, input: Entity, drawn: &'static str) {
    if drawn.is_empty() {
        return;
    }
    commands.spawn((
        Bracket {
            input,
            drawn,
            drawn_lit: None,
        },
        text(),
        sized(drawn.len() as f32, 1.0),
        ChildOf(row),
    ));
}

/// A form's text field: upstream's text input in both skins, edited and
/// drawn on a terminal's cells by plurimus and in a window by bevy.
#[derive(Component, Debug, Default)]
#[require(UiArea = placed(), FieldInput, ShownText)]
pub struct TextField;

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
        .insert((TextField, TabIndex(0), field, ChildOf(row)));
    spawn_beside(commands, row, input, BRACKETS[1]);
}

/// Brackets are lit as a select's are: dim at rest, and in the focused
/// style while their input holds the keyboard.
pub fn light_brackets(
    focus: Res<InputFocus>,
    theme: Res<Theme>,
    mut brackets: Query<(Entity, &mut Bracket)>,
    mut commands: Commands,
) {
    for (entity, mut bracket) in &mut brackets {
        let is_lit = focus.get() == Some(bracket.input);
        if bracket.drawn_lit == Some(is_lit) && !theme.is_changed() {
            continue;
        }
        bracket.drawn_lit = Some(is_lit);
        let style = if is_lit {
            theme.focused()
        } else {
            theme.dimmed()
        };
        say(
            &mut commands,
            entity,
            Line::styled(bracket.drawn, style),
            &theme,
        );
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
        FieldKind::Flag | FieldKind::Presence(_) => flag::spawn(commands, row, field),
        FieldKind::Rate | FieldKind::Share => slider::spawn(commands, row, field),
        FieldKind::Remainder => {
            commands.spawn((
                Remaining::default(),
                field,
                Node {
                    margin: UiRect::left(Val::Em(REMAINDER_INDENT)),
                    ..sharing()
                },
                text(),
                ChildOf(row),
            ));
        }
        FieldKind::Trigger => super::trigger::spawn(commands, row, field),
        kind => {
            // What it chooses between is filled in with the item.
            let select = Select::new(kind, offered(&spec, Vec::new(), None));
            let select = spawn_select(commands, select, field);
            commands.entity(select).insert((sharing(), ChildOf(row)));
        }
    }
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
    texts: Query<
        'w,
        's,
        (
            &'static FormField,
            &'static mut EditableText,
            &'static mut ShownText,
        ),
        Without<Slot>,
    >,
    selects: Query<'w, 's, (&'static FormField, &'static mut Select), Without<Slot>>,
    sliders: Query<'w, 's, (&'static FormField, &'static SliderValue)>,
    checks: Query<'w, 's, (&'static FormField, Has<Checked>), With<FlagBox>>,
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
    /// holds it - dressed as its kind reads, where it parses.
    fn show_text_at(&mut self, widget: Entity, editing: &Editing, is_focused: bool) {
        let Ok((field, mut text, mut shown)) = self.texts.get_mut(widget) else {
            return;
        };
        if editing.is_incomplete(field.spec.key) {
            // A trigger's operand is typed as the file states it.
            let kind = field.spec.kind;
            let typed = text.value().to_string();
            let read = parse_field(kind, &typed).filter(|_| kind != FieldKind::Trigger);
            if let Some(read) = read {
                let own = field_text(kind, Some(&read), is_focused);
                show_text(&mut text, &mut shown, own);
            }
            return;
        }
        let value = get_path(editing.snapshot(), field.spec.key);
        let value = match field.spec.kind {
            FieldKind::Listed(back) => nth_back(value, back),
            _ => value,
        };
        let wanted = field_text(field.spec.kind, value, is_focused);
        show_text(&mut text, &mut shown, wanted);
    }

    /// Shows each rate on both of its widgets, but for a text being typed
    /// into, which is read back once the keyboard leaves it.
    pub fn show_rates(&mut self, form: Entity, editing: &Editing, focused: Option<Entity>) {
        let widgets: Vec<Entity> = self.tree.children.iter_descendants(form).collect();
        for widget in widgets {
            self.show_slider_at(widget, editing);
            let is_rate = |(field, ..): (&FormField, _, _)| {
                matches!(field.spec.kind, FieldKind::Rate | FieldKind::Share)
            };
            if focused != Some(widget) && self.texts.get(widget).is_ok_and(is_rate) {
                self.show_text_at(widget, editing, false);
            }
        }
    }

    /// The stock slider's value is immutable, so a change is an insert.
    fn show_slider_at(&mut self, widget: Entity, editing: &Editing) {
        if let Ok((field, shown)) = self.sliders.get(widget) {
            let rate = get_path(editing.snapshot(), field.spec.key).and_then(Value::as_float);
            let wanted = SliderValue(rate.unwrap_or(0.0) as f32);
            if *shown != wanted {
                self.commands.entity(widget).insert(wanted);
            }
        }
    }

    pub fn show(&mut self, form: Entity, editing: &Editing, plan: &Plan, focused: Option<Entity>) {
        let table = editing.snapshot();
        let widgets: Vec<Entity> = self.tree.children.iter_descendants(form).collect();
        for widget in widgets {
            self.show_text_at(widget, editing, focused == Some(widget));
            self.show_slider_at(widget, editing);
            if let Ok((field, mut select)) = self.selects.get_mut(widget) {
                let value = get_path(table, field.spec.key);
                // An order's place is offered every word again, since what
                // the other places leave it is decided once all are filled.
                let value = match field.spec.kind {
                    FieldKind::Order(_, place) => nth(value, place),
                    _ => value,
                };
                let options = candidates(field.spec.kind, plan);
                Select::fill(&mut select, Some(options), value);
            }
            if let Ok((field, is_checked)) = self.checks.get(widget) {
                let value = get_path(table, field.spec.key);
                let wanted = match field.spec.kind {
                    FieldKind::Presence(_) => value.is_some_and(Value::is_table),
                    _ => value.and_then(Value::as_bool).unwrap_or(false),
                };
                flag::show(widget, wanted, is_checked, &mut self.commands);
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
