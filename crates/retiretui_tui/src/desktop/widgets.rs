//! The plurimus widgets the shell lays out, mirrored in the window: a
//! list's rows with its cursor, a button with its brackets, and a toast in
//! its box, each a node of text over the widget the terminal draws, and
//! each answering a click as the terminal's does.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, ResMut, With,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{BackgroundColor, BorderColor, BorderRadius, FlexDirection, Node, UiRect, Val};
use plurimus::bui::bevy_picking::events::PointerClick;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::{UiLabel, UiStyle};
use plurimus::widgets::{
    Activate, ActiveDescendant, Button, ListBox, ListBoxCursor, ListItem, ListItemTrailing,
};

use super::chrome::{HAIRLINE, HALF_CELL_WIDE};
use crate::layout::{BUTTON_BRACKETS, CURSOR_COLS};
use crate::theme::{Repainted, Theme, say, text};
use crate::toast::{Shown, Toasts};

/// The gutter beside a row the cursor is not on, as wide as the cursor.
const GUTTER: &str = "  ";
const _: () = assert!(GUTTER.len() == CURSOR_COLS as usize);

pub fn plugin(app: &mut App) {
    app.add_observer(stack_the_list);
    app.add_observer(row_the_item);
    app.add_observer(label_the_button);
    app.add_observer(box_the_toast);
    app.add_systems(Update, (paint_rows, paint_buttons).in_set(Repainted));
}

/// A list's rows stack down it, as the terminal's listbox draws them.
fn stack_the_list(added: On<Add<ListBox>>, mut lists: Query<&mut Node>) {
    if let Ok(mut node) = lists.get_mut(added.entity) {
        node.flex_direction = FlexDirection::Column;
    }
}

/// A row's label and trailing text, and whether it was last drawn under
/// the cursor: a cursor move repaints the two rows it leaves and reaches,
/// not the list.
#[derive(Component, Debug)]
struct Rowed {
    label: Entity,
    trailing: Entity,
    is_cursor: bool,
}

fn row_the_item(added: On<Add<ListItem>>, mut commands: Commands) {
    let row = added.entity;
    let label = commands
        .spawn((
            text(),
            Node {
                flex_grow: 1.0,
                ..Node::default()
            },
            ChildOf(row),
        ))
        .id();
    let trailing = commands.spawn((text(), ChildOf(row))).id();
    commands
        .entity(row)
        .insert((
            Node {
                width: Val::Percent(100.0),
                height: Val::Rem(1.0),
                flex_shrink: 0.0,
                ..Node::default()
            },
            Rowed {
                label,
                trailing,
                is_cursor: false,
            },
        ))
        .observe(point_on_click);
}

/// A click on a row puts the list's cursor on it, which is what a
/// terminal's press does.
fn point_on_click(
    click: On<PointerClick>,
    rows: Query<&ChildOf, With<ListItem>>,
    mut lists: Query<&mut ActiveDescendant>,
) {
    let row = click.entity;
    if let Ok(parent) = rows.get(row)
        && let Ok(mut cursor) = lists.get_mut(parent.parent())
        && cursor.0 != Some(row)
    {
        cursor.0 = Some(row);
    }
}

fn paint_rows(
    theme: Res<Theme>,
    lists: Query<(&ActiveDescendant, Option<Ref<ListBoxCursor>>)>,
    mut rows: Query<
        (
            Entity,
            &ChildOf,
            Ref<UiLabel>,
            Option<Ref<ListItemTrailing>>,
            Option<Ref<UiStyle>>,
            &mut Rowed,
        ),
        With<ListItem>,
    >,
    mut commands: Commands,
) {
    for (row, parent, label, trailing, style, mut rowed) in &mut rows {
        let Ok((cursor, bar)) = lists.get(parent.parent()) else {
            continue;
        };
        let is_cursor = cursor.0 == Some(row);
        let has_moved = rowed.is_cursor != is_cursor
            || bar.as_ref().is_some_and(DetectChanges::is_changed)
            || trailing.as_ref().is_some_and(DetectChanges::is_changed)
            || style.as_ref().is_some_and(DetectChanges::is_changed);
        if !has_moved && !theme.is_changed() && !label.is_changed() && !rowed.is_added() {
            continue;
        }
        rowed.is_cursor = is_cursor;
        let mut line = Line::default();
        if is_cursor {
            let glyph = bar.map_or_else(|| Line::from(GUTTER), |bar| bar.0.clone());
            line.extend(
                glyph
                    .spans
                    .into_iter()
                    .map(|span| span.patch_style(theme.accented())),
            );
        } else {
            line.push_span(Span::raw(GUTTER));
        }
        line.extend(label.0.spans.iter().cloned());
        let line = match style {
            Some(style) => line.patch_style(style.0),
            None => line,
        };
        say(&mut commands, rowed.label, line, &theme);
        let trailing = trailing.map_or_else(Line::default, |trailing| trailing.0.clone());
        say(&mut commands, rowed.trailing, trailing, &theme);
    }
}

/// A button's label.
#[derive(Component, Debug)]
struct Labelled(Entity);

fn label_the_button(added: On<Add<Button>>, mut commands: Commands) {
    let button = added.entity;
    let label = commands.spawn((text(), ChildOf(button))).id();
    commands
        .entity(button)
        .insert(Labelled(label))
        .observe(activate_on_click);
}

fn activate_on_click(click: On<PointerClick>, mut commands: Commands) {
    commands.trigger(Activate {
        entity: click.entity,
    });
}

/// `[ label ]`, bold in the accent while the keyboard is on it, and in
/// the style its emphasis gives it otherwise.
fn paint_buttons(
    theme: Res<Theme>,
    focus: Res<InputFocus>,
    buttons: Query<(Entity, Ref<UiLabel>, Option<Ref<UiStyle>>, Ref<Labelled>), With<Button>>,
    mut commands: Commands,
) {
    for (button, label, style, labelled) in &buttons {
        let has_changed = focus.is_changed()
            || theme.is_changed()
            || label.is_changed()
            || labelled.is_added()
            || style.as_ref().is_some_and(DetectChanges::is_changed);
        if !has_changed {
            continue;
        }
        let worn = if focus.get() == Some(button) {
            theme.focused()
        } else {
            style.map_or_else(Default::default, |style| style.0)
        };
        let [open, close] = BUTTON_BRACKETS;
        let mut line = Line::from(open);
        line.extend(label.0.spans.iter().cloned());
        line.push_span(close);
        say(&mut commands, labelled.0, line.patch_style(worn), &theme);
    }
}

/// A toast is its line in a box of its own colour on the ground, which
/// a click takes down.
fn box_the_toast(
    added: On<Add<Shown>>,
    toasts: Query<&UiLabel>,
    theme: Res<Theme>,
    mut commands: Commands,
) {
    let toast = added.entity;
    let Ok(label) = toasts.get(toast) else {
        return;
    };
    let edge = theme.ink(label.0.style);
    let line = commands.spawn((text(), ChildOf(toast))).id();
    commands
        .entity(toast)
        .insert((
            Node {
                padding: UiRect::axes(Val::Em(1.0), Val::Rem(1.0)),
                border: UiRect::all(HAIRLINE),
                border_radius: BorderRadius::all(HALF_CELL_WIDE),
                ..Node::default()
            },
            BorderColor::all(edge),
            BackgroundColor(theme.ground()),
        ))
        .observe(dismiss_on_click);
    say(&mut commands, line, label.0.clone(), &theme);
}

fn dismiss_on_click(click: On<PointerClick>, shown: Query<&Shown>, mut toasts: ResMut<Toasts>) {
    if let Ok(shown) = shown.get(click.entity) {
        toasts.dismiss(shown.0);
    }
}
