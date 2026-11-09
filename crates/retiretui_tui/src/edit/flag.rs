//! A flag's box: a text node both skins draw, ticked by space or a click.
//!
//! It is the form's own rather than a stock widget's because a stock
//! checkbox toggles on Enter as well, from a global observer that runs
//! before anything the form could observe on the box - and Enter in a
//! form applies the item.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Has, On, Query, Res};
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::FocusedInput;
use bevy_input_focus::InputFocus;
use bevy_input_focus::tab_navigation::TabIndex;
use plurimus::core::UiArea;
use plurimus::core::ratatui_core::text::Line;
use plurimus::term::HeldModifiers;
use plurimus::ui::bevy_picking::events::PointerClick;
use plurimus::ui::{Checked, KeyBinding, ValueChange, first_bound};

use super::build::FormField;
use super::field::sharing;
use crate::layout::placed;
use crate::theme::{Theme, say, text};

/// A flag's box, and what it last drew: whether ticked, and whether held.
#[derive(Component, Debug, Default)]
#[require(UiArea = placed())]
pub struct FlagBox {
    drawn: Option<(bool, bool)>,
}

const MARKS: [&str; 2] = ["[ ] ", "[x] "];

pub fn spawn(commands: &mut Commands, row: Entity, field: FormField) {
    commands
        .spawn((
            FlagBox::default(),
            field,
            TabIndex(0),
            text(),
            sharing(),
            ChildOf(row),
        ))
        .observe(toggle_on_key)
        .observe(toggle_on_click);
}

fn toggle_on_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    boxes: Query<Has<Checked>>,
    mut commands: Commands,
) {
    let entity = input.focused_entity;
    let Ok(is_checked) = boxes.get(entity) else {
        return;
    };
    let tick = [(KeyBinding::new(Key::Character(" ".into())), ())];
    if first_bound(&tick, &input.input, held.get()).is_none() {
        return;
    }
    input.propagate(false);
    toggle(entity, is_checked, &mut commands);
}

fn toggle_on_click(click: On<PointerClick>, boxes: Query<Has<Checked>>, mut commands: Commands) {
    if let Ok(is_checked) = boxes.get(click.entity) {
        toggle(click.entity, is_checked, &mut commands);
    }
}

/// Marks the box and tells the form, which enters the tick on the item.
fn toggle(entity: Entity, is_checked: bool, commands: &mut Commands) {
    show(entity, !is_checked, is_checked, commands);
    commands.trigger(ValueChange {
        source: entity,
        value: !is_checked,
        is_final: true,
    });
}

/// Marks a flag's box, which is a component rather than a value.
pub fn show(entity: Entity, wanted: bool, is_checked: bool, commands: &mut Commands) {
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

/// `[x] ` or `[ ] `, in the focused style while the box holds the
/// keyboard.
pub fn paint(
    theme: Res<Theme>,
    focus: Res<InputFocus>,
    mut boxes: Query<(Entity, &mut FlagBox, Has<Checked>)>,
    mut commands: Commands,
) {
    for (entity, mut flag, is_checked) in &mut boxes {
        let state = (is_checked, focus.get() == Some(entity));
        if flag.drawn == Some(state) && !theme.is_changed() {
            continue;
        }
        flag.drawn = Some(state);
        let mark = MARKS[usize::from(is_checked)];
        let line = Line::styled(mark, theme.held(state.1));
        say(&mut commands, entity, line, &theme);
    }
}
