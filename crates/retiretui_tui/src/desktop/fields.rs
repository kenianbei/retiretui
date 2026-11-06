//! A form's text field in the window. plurimus's text state is the value
//! both skins share; here bevy's text input edits it - the state shown
//! into bevy's buffer when the form moves it, the buffer read back into
//! the state and told to the form as it is typed, and Enter made the
//! field's submission, which bevy leaves to the app. The field is a box
//! ringed in the accent while it holds the keyboard, and the brackets the
//! terminal draws around it give it their cells.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::event::EntityEvent;
use bevy_ecs::prelude::{
    Add, Changed, Commands, Entity, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_input::ButtonState;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::{FocusedInput, InputFocus};
#[cfg(test)]
use bevy_ui::UiRect;
use bevy_ui::{BorderColor, Node, Overflow, Val};
use plurimus::bui::bevy_text::{
    EditableText as Buffer, TextColor, TextCursorStyle, TextEdit, TextEditChange, TextFont,
};
use plurimus::bui::bevy_ui_widgets::TextInput as Input;
use plurimus::core::ratatui_core::style::Style;
use plurimus::widgets::{Submit, TextInput, ValueChange};

#[cfg(test)]
use super::chrome::HAIRLINE;
use super::chrome::box_in;
use crate::edit::{Bracket, TextField, show_text};
use crate::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.add_observer(dress_the_field);
    app.add_observer(blank_the_bracket);
    app.add_observer(read_the_buffer);
    app.add_systems(
        Update,
        (show_the_state, paint_fields, ring_fields).in_set(Repainted),
    );
}

/// A bracket keeps no cells in the window: its input, which shares the
/// row, grows into them. A word of a sentence stays.
fn blank_the_bracket(added: On<Add<Bracket>>, mut brackets: Query<(&Bracket, &mut Node)>) {
    if let Ok((bracket, mut node)) = brackets.get_mut(added.entity)
        && bracket.is_bracket()
    {
        node.width = Val::ZERO;
        node.overflow = Overflow::clip();
    }
}

/// A field's border: the accent while it holds the keyboard.
fn ring_fields(
    focus: Res<InputFocus>,
    theme: Res<Theme>,
    fields: Query<(Entity, Ref<Input>), With<TextField>>,
    mut commands: Commands,
) {
    for (field, input) in &fields {
        if !focus.is_changed() && !theme.is_changed() && !input.is_added() {
            continue;
        }
        let is_held = focus.get() == Some(field);
        let ring = theme.ink(theme.edge(is_held));
        commands.entity(field).insert(BorderColor::all(ring));
    }
}

fn dress_the_field(
    added: On<Add<TextField>>,
    states: Query<&TextInput>,
    theme: Res<Theme>,
    mut commands: Commands,
) {
    let field = added.entity;
    let Ok(state) = states.get(field) else {
        return;
    };
    commands
        .entity(field)
        .insert((
            Input,
            Buffer::new(state.value()),
            TextFont::default(),
            ink(&theme),
        ))
        .observe(submit_on_enter)
        .entry::<Node>()
        .or_default()
        .and_modify(|mut node| box_in(&mut node, 1.0));
}

/// The theme's ink, and its accent for the caret and the selection, the
/// selected text in the ground's colour over it.
fn ink(theme: &Theme) -> (TextColor, TextCursorStyle) {
    let accent = theme.ink(theme.accented());
    (
        TextColor(theme.ink(Style::default())),
        TextCursorStyle {
            color: accent,
            selection_color: accent,
            unfocused_selection_color: accent,
            selected_text_color: Some(theme.ground()),
            ..TextCursorStyle::default()
        },
    )
}

/// A state the form moved - shown from the item, or restated once it
/// parses - reaches the buffer with the caret at its end; one the buffer
/// itself moved is already there.
fn show_the_state(
    mut fields: Query<(&TextInput, &mut Buffer), (With<TextField>, Changed<TextInput>)>,
) {
    for (state, mut buffer) in &mut fields {
        if buffer.value() == state.value() {
            continue;
        }
        buffer.editor_mut().set_text(state.value());
        buffer.queue_edit(TextEdit::TextEnd(false));
    }
}

/// What is typed reaches the state and the form as the terminal's field
/// reports it. The buffer reports a caret move too, which moves no value.
fn read_the_buffer(
    changed: On<TextEditChange>,
    mut fields: Query<(&Buffer, &mut TextInput), With<TextField>>,
    mut commands: Commands,
) {
    let field = changed.event_target();
    let Ok((buffer, mut state)) = fields.get_mut(field) else {
        return;
    };
    if buffer.value() == state.value() {
        return;
    }
    let value = buffer.value().to_string();
    show_text(&mut state, value.clone());
    commands.trigger(ValueChange {
        source: field,
        value,
        is_final: false,
    });
}

/// Enter submits the field, as the terminal's does, with the final change
/// beside it.
fn submit_on_enter(
    mut input: On<FocusedInput<KeyboardInput>>,
    states: Query<&TextInput, With<TextField>>,
    mut commands: Commands,
) {
    let field = input.focused_entity;
    let Ok(state) = states.get(field) else {
        return;
    };
    let key = &input.input;
    if key.logical_key != Key::Enter || key.state != ButtonState::Pressed || key.repeat {
        return;
    }
    input.propagate(false);
    let value = state.value().to_owned();
    commands.trigger(ValueChange {
        source: field,
        value: value.clone(),
        is_final: true,
    });
    commands.trigger(Submit::new(field, value));
}

fn paint_fields(
    theme: Res<Theme>,
    fields: Query<Entity, (With<TextField>, With<Input>)>,
    mut commands: Commands,
) {
    if !theme.is_changed() {
        return;
    }
    for field in &fields {
        commands.entity(field).insert(ink(&theme));
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::World;
    use bevy_ecs::system::RunSystemOnce;

    use super::*;
    use crate::support::{SIZE, headless_app, press_ctrl};
    use bevy_ecs::prelude::Has;
    use plurimus::term::KeyCode;

    #[test]
    fn the_new_plan_form_s_text_fields_are_dressed_for_the_window() {
        let mut app = headless_app(SIZE);
        app.add_plugins(plugin);
        press_ctrl(&mut app, KeyCode::Char('n'));
        app.update();
        let world = app.world_mut();
        let dressed: Vec<(bool, &Node)> = world
            .query_filtered::<(Has<Input>, &Node), With<TextField>>()
            .iter(world)
            .collect();
        assert!(dressed.len() > 1, "the form's text fields stand");
        assert!(
            dressed.iter().all(|&(is_dressed, _)| is_dressed),
            "each has bevy's input"
        );
        assert!(
            dressed
                .iter()
                .all(|(_, node)| node.border == UiRect::all(HAIRLINE)),
            "each is a box"
        );
        let blanked: Vec<Val> = world
            .query::<(&Bracket, &Node)>()
            .iter(world)
            .filter(|(bracket, _)| bracket.is_bracket())
            .map(|(_, node)| node.width)
            .collect();
        assert!(!blanked.is_empty(), "the fields have brackets");
        assert!(blanked.iter().all(|&width| width == Val::ZERO));
    }

    #[test]
    fn a_state_the_form_moved_reaches_the_buffer_and_one_it_did_not_is_left() {
        let mut world = World::new();
        let moved = world
            .spawn((TextField, TextInput::new("7%"), Buffer::new("")))
            .id();
        let mut said = Buffer::new("abc");
        said.pending_edits.clear();
        let typed = world.spawn((TextField, TextInput::new("abc"), said)).id();
        world.run_system_once(show_the_state).unwrap();
        let buffer = |entity: Entity| world.get::<Buffer>(entity).unwrap().value().to_string();
        assert_eq!(buffer(moved), "7%");
        assert!(
            world.get::<Buffer>(typed).unwrap().pending_edits.is_empty(),
            "a buffer already saying the state is not touched"
        );
    }
}
