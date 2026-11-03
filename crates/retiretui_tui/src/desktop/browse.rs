//! The file picker in the window. plurimus's picker edits its field and
//! steps its list as it does in the terminal; the window says the prompt
//! and the field's state as text, with the caret the terminal shows, lays
//! the parts out as the picker places them in cells so the two agree, and
//! gives the list a node for the row skin.

use std::ops::Range;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{ComputedNode, Node, Overflow, PositionType, Val};
use plurimus::bui::bevy_picking::events::PointerClick;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::widgets::TextInput;
use plurimus_filepicker::{
    FilePickerAction, FilePickerCommand, FilePickerField, FilePickerList, FilePickerLook,
    FilePickerParts,
};

use crate::layout;
use crate::theme::{Repainted, Theme, say, text};

/// Clicks on one row that choose it, as the terminal's picker counts them.
const DOUBLE_CLICK: u8 = 2;

pub fn plugin(app: &mut App) {
    app.add_observer(place_the_parts);
    app.add_systems(Update, (paint_prompts, paint_fields).in_set(Repainted));
}

/// The prompt's node under a picker.
#[derive(Component, Debug)]
struct Prompt(Entity);

/// The field after the prompt on the first row and the list on the rows
/// under it, as `split_area` cuts the picker's cells; the prompt beside
/// the field.
fn place_the_parts(
    added: On<Add<FilePickerParts>>,
    pickers: Query<(&FilePickerParts, &FilePickerLook)>,
    mut commands: Commands,
) {
    let picker = added.entity;
    let Ok((parts, look)) = pickers.get(picker) else {
        return;
    };
    let prompt = look.prompt.width() as f32;
    commands.entity(parts.field).insert((
        text(),
        Node {
            left: Val::Em(prompt),
            right: Val::ZERO,
            overflow: Overflow::clip(),
            ..on_the_first_row()
        },
    ));
    commands
        .entity(parts.list)
        .entry::<Node>()
        .or_default()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.top = Val::Rem(1.0);
            node.left = Val::ZERO;
            node.right = Val::ZERO;
            node.bottom = Val::ZERO;
        });
    commands.entity(parts.list).observe(enter_on_double_click);
    let prompt = commands
        .spawn((text(), on_the_first_row(), ChildOf(picker)))
        .id();
    commands.entity(picker).insert(Prompt(prompt));
}

fn on_the_first_row() -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: Val::ZERO,
        left: Val::ZERO,
        height: Val::Rem(1.0),
        ..Node::default()
    }
}

fn paint_prompts(
    theme: Res<Theme>,
    pickers: Query<(Ref<Prompt>, Ref<FilePickerLook>)>,
    mut commands: Commands,
) {
    for (prompt, look) in &pickers {
        if !theme.is_changed() && !look.is_changed() && !prompt.is_added() {
            continue;
        }
        say(&mut commands, prompt.0, look.prompt.clone(), &theme);
    }
}

/// The field's text, scrolled so the caret is in view, the selection or
/// the caret in the accent over the ground while the field holds the
/// keyboard, as the terminal's field draws it.
fn paint_fields(
    theme: Res<Theme>,
    focus: Res<InputFocus>,
    fields: Query<(Entity, Ref<TextInput>, Ref<ComputedNode>), With<FilePickerField>>,
    mut commands: Commands,
) {
    for (field, state, computed) in &fields {
        let is_stale =
            state.is_changed() || theme.is_changed() || focus.is_changed() || computed.is_changed();
        if !is_stale {
            continue;
        }
        let marked = if focus.get() == Some(field) {
            state
                .selection()
                .unwrap_or(state.cursor()..state.cursor() + 1)
        } else {
            0..0
        };
        let line = field_line(&state, marked, layout::node_cols(&computed), &theme);
        say(&mut commands, field, line, &theme);
    }
}

/// `value` with `marked` characters in the accent over the ground, a
/// caret past the end a space, scrolled by whole characters so the mark's
/// end is within `cols`.
fn field_line(state: &TextInput, marked: Range<usize>, cols: u16, theme: &Theme) -> Line<'static> {
    let mut chars: Vec<char> = state.value().chars().collect();
    if marked.end > chars.len() {
        chars.push(' ');
    }
    let end = chars.len();
    let marked = marked.start.min(end)..marked.end.min(end);
    let scroll = marked.end.saturating_sub(usize::from(cols));
    let text = |range: Range<usize>| -> String {
        chars[range.start.max(scroll)..range.end.max(scroll)]
            .iter()
            .collect()
    };
    Line::from(vec![
        Span::raw(text(0..marked.start)),
        Span::styled(text(marked.clone()), theme.caret()),
        Span::raw(text(marked.end..end)),
    ])
}

/// A double click on a row is Enter on it, as the terminal's is; the
/// first click put the cursor there, as the widget skin answers any row.
fn enter_on_double_click(
    click: On<PointerClick>,
    lists: Query<&ChildOf, With<FilePickerList>>,
    mut commands: Commands,
) {
    if click.count < DOUBLE_CLICK {
        return;
    }
    if let Ok(parent) = lists.get(click.entity) {
        commands.trigger(FilePickerCommand {
            entity: parent.parent(),
            action: FilePickerAction::Enter,
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::hierarchy::Children;
    use plurimus::bui::bevy_ui_widgets::TextInput as Input;
    use plurimus::core::UiArea;
    use plurimus::core::ratatui_core::layout::Rect;
    use plurimus::term::KeyCode;
    use plurimus::widgets::ActiveDescendant;

    use super::*;
    use crate::support::{
        Headless, SIZE, headless_app_at, is_browsing, press_key, said_by, scratch_workspace,
        type_text,
    };

    /// The window skin over a shell launched on a directory, which opens
    /// the picker.
    fn browsing() -> Headless {
        let mut app = headless_app_at(scratch_workspace(""), SIZE);
        app.add_plugins((plugin, super::super::widgets::plugin));
        app.update();
        app.update();
        assert!(is_browsing(&app));
        app
    }

    fn parts(app: &mut Headless) -> (Entity, FilePickerParts) {
        let world = app.world_mut();
        world
            .query::<(Entity, &FilePickerParts)>()
            .single(world)
            .map(|(picker, parts)| (picker, *parts))
            .expect("one picker is open")
    }

    fn area(app: &Headless, entity: Entity) -> Rect {
        match app.world().get::<UiArea>(entity) {
            Some(UiArea::Fixed(rect)) => *rect,
            other => panic!("{entity} has no fixed area: {other:?}"),
        }
    }

    fn texts<'a>(line: &'a Line<'a>) -> Vec<&'a str> {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn the_parts_lie_where_the_picker_places_them_and_the_field_is_the_engine_s() {
        let mut app = browsing();
        let (picker, parts) = parts(&mut app);
        let whole = area(&app, picker);
        let field = area(&app, parts.field);
        let list = area(&app, parts.list);
        assert!(!whole.is_empty(), "{whole:?}");
        assert_eq!(
            (field.y, field.height, field.right()),
            (whole.y, 1, whole.right())
        );
        assert_eq!(
            (list.x, list.y, list.width, list.height),
            (whole.x, whole.y + 1, whole.width, whole.height - 1)
        );
        let world = app.world();
        assert!(
            !world.entity(parts.field).contains::<Input>(),
            "the field is edited by the engine, not bevy, so its keys bubble"
        );
        let rows = world.get::<Children>(parts.list).expect("rows");
        assert!(rows.len() > 1);
        assert!(rows.iter().all(|&row| world.entity(row).contains::<Node>()));
    }

    #[test]
    fn the_field_says_its_state_with_the_caret_and_the_keys_stay_the_picker_s() {
        let mut app = browsing();
        let (picker, parts) = parts(&mut app);
        let cursor_before = app.world().get::<ActiveDescendant>(parts.list).unwrap().0;
        press_key(&mut app, KeyCode::Down);
        let cursor_after = app.world().get::<ActiveDescendant>(parts.list).unwrap().0;
        assert_ne!(
            cursor_before, cursor_after,
            "Down steps the list from the field"
        );

        type_text(&mut app, "pl");
        app.update();
        let value = app
            .world()
            .get::<TextInput>(parts.field)
            .unwrap()
            .value()
            .to_string();
        assert!(value.ends_with("pl"), "{value}");
        assert_eq!(
            said_by(app.world(), parts.field),
            format!("{value} "),
            "the caret past the end"
        );
        let prompt = app.world().get::<Prompt>(picker).unwrap().0;
        assert!(!said_by(app.world(), prompt).is_empty());
    }

    #[test]
    fn a_line_marks_the_caret_and_scrolls_to_keep_it_in_view() {
        let theme = Theme::default();
        let state = TextInput::new("abcdef");
        assert_eq!(
            texts(&field_line(&state, 6..7, 10, &theme)),
            ["abcdef", " ", ""]
        );
        assert_eq!(
            texts(&field_line(&state, 6..7, 4, &theme)),
            ["def", " ", ""],
            "the first three characters are off the left"
        );
        assert_eq!(
            texts(&field_line(&state, 0..0, 10, &theme)),
            ["", "", "abcdef"]
        );
    }
}
