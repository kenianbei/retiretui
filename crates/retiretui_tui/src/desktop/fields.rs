//! Text fields in the window: upstream's text input, which bevy edits and
//! draws itself, its caret and selection in the accent; a form's dressed
//! as a box ringed in the accent while it holds the keyboard, the brackets
//! the terminal draws around it giving it their cells.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::prelude::{Add, Commands, Entity, IntoScheduleConfigs, On, Query, Res};
use bevy_input_focus::InputFocus;
use bevy_ui::{BorderColor, Node, Overflow, Val};
use plurimus::bui::bevy_text::TextCursorStyle;
use plurimus::bui::bevy_ui_widgets::TextInput as FieldInput;

use bevy_text::LineHeight;

use super::CELL;
use super::chrome::{HAIRLINE_PX, box_in};
use crate::edit::{Bracket, TextField};
use crate::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.add_observer(box_the_form_s_field);
    app.add_observer(blank_the_bracket);
    app.add_systems(Update, (paint_carets, ring_fields).in_set(Repainted));
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
    fields: Query<(Entity, Ref<TextField>)>,
    mut commands: Commands,
) {
    for (field, dressed) in &fields {
        if !focus.is_changed() && !theme.is_changed() && !dressed.is_added() {
            continue;
        }
        let is_held = focus.get() == Some(field);
        let ring = theme.ink(theme.edge(is_held));
        commands.entity(field).insert(BorderColor::all(ring));
    }
}

/// A form field's line: a cell less the hairline above and below it. A
/// line taller than the box never fits its caret, and upstream's field,
/// revealing the caret every layout, then flips between the line's top and
/// bottom, a frame at each.
const FIELD_LINE: LineHeight = LineHeight::Px(CELL.y - 2.0 * HAIRLINE_PX);

fn box_the_form_s_field(added: On<Add<TextField>>, mut commands: Commands) {
    commands
        .entity(added.entity)
        .insert(FIELD_LINE)
        .entry::<Node>()
        .or_default()
        .and_modify(|mut node| box_in(&mut node, 1.0));
}

/// The caret and the selection in the accent, the selected text in the
/// ground's colour over it.
fn caret(theme: &Theme) -> TextCursorStyle {
    let accent = theme.ink(theme.accented());
    TextCursorStyle {
        color: accent,
        selection_color: accent,
        unfocused_selection_color: accent,
        selected_text_color: Some(theme.ground()),
        ..TextCursorStyle::default()
    }
}

/// Every field's caret, as it comes and as the theme moves.
fn paint_carets(
    theme: Res<Theme>,
    fields: Query<(Entity, Ref<FieldInput>)>,
    mut commands: Commands,
) {
    for (field, spawned) in &fields {
        if theme.is_changed() || spawned.is_added() {
            commands.entity(field).try_insert(caret(&theme));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ui::UiRect;

    use super::*;
    use crate::desktop::chrome::HAIRLINE;
    use bevy_ecs::prelude::With;

    use crate::support::{SIZE, headless_app, press_ctrl, typed};

    #[test]
    fn the_new_plan_form_s_text_fields_are_dressed_for_the_window() {
        let mut app = headless_app(SIZE);
        app.add_plugins(plugin);
        press_ctrl(&mut app, typed('n'));
        app.update();
        let world = app.world_mut();
        let boxes: Vec<&Node> = world
            .query_filtered::<&Node, With<TextField>>()
            .iter(world)
            .collect();
        assert!(boxes.len() > 1, "the form's text fields stand");
        assert!(
            boxes
                .iter()
                .all(|node| node.border == UiRect::all(HAIRLINE)),
            "each is a box"
        );
        let lines: Vec<&LineHeight> = world
            .query_filtered::<&LineHeight, With<TextField>>()
            .iter(world)
            .collect();
        assert!(
            lines.iter().all(|&&line| line == FIELD_LINE),
            "each line fits inside its box's hairlines: {lines:?}"
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
}
