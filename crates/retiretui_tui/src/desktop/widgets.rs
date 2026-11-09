//! The plurimus widgets the shell lays out, mirrored in the window: a
//! list's rows with its cursor, a button as a box - a select as a box
//! across its row with its chevron at the far end - and a toast in its
//! box, each a node of text over the widget the terminal draws, and each
//! answering a click as the terminal's does.

use bevy_app::{App, Update};
use bevy_color::Color;
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Changed, Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{
    BackgroundColor, BorderColor, BorderRadius, ComputedNode, FlexDirection, Node, Overflow,
    ScrollPosition, UiRect, Val,
};
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::{ScrollOffset, UiLabel, UiStyle};
use plurimus::widgets::{
    ActiveDescendant, Button, ListBox, ListBoxCursor, ListItem, ListItemTrailing,
};

use super::chrome::{HAIRLINE, HALF_CELL_WIDE, box_in};
use crate::edit::{CHEVRON, SelectBox};
use crate::layout::{BUTTON_BRACKETS, Emphasis, GUTTER};
use crate::theme::{Repainted, Theme, say, text};
use crate::toast::Shown;

/// Cells a button's box keeps either side of its label: what its
/// brackets take in the terminal.
const BUTTON_PADDING: f32 = BUTTON_BRACKETS[0].len() as f32;

pub fn plugin(app: &mut App) {
    app.add_observer(stack_the_list);
    app.add_observer(row_the_item);
    app.add_observer(label_the_button);
    app.add_observer(box_the_toast);
    app.add_systems(
        Update,
        (paint_rows, paint_buttons, scroll_the_list).in_set(Repainted),
    );
}

/// A list's rows stack down it, as the terminal's listbox draws them, and
/// the rows past its node scroll rather than spill; a list spawned with
/// no node of its own gets one.
fn stack_the_list(added: On<Add<ListBox>>, mut commands: Commands) {
    commands
        .entity(added.entity)
        .entry::<Node>()
        .or_default()
        .and_modify(|mut node| {
            node.flex_direction = FlexDirection::Column;
            node.overflow = Overflow::scroll_y();
        });
}

/// The rows the engine scrolled a list by - revealing its cursor, or a
/// wheel tick in the terminal - scroll its node the same, a rem a row.
fn scroll_the_list(
    mut lists: Query<
        (&ScrollOffset, &ComputedNode, &mut ScrollPosition),
        (With<ListBox>, Changed<ScrollOffset>),
    >,
) {
    for (offset, computed, mut scroll) in &mut lists {
        scroll.0.y = f32::from(offset.0.y) * computed.rem_size.0;
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
    commands.entity(row).insert((
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
    ));
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

/// A button's label, and a select's chevron.
#[derive(Component, Debug)]
struct Labelled {
    label: Entity,
    chevron: Option<Entity>,
}

/// A button is a box around its label, the cells its brackets took as
/// padding; a select - spawned with its button, so it is one by now - is
/// a box across its row, its chevron at the far end.
fn label_the_button(
    added: On<Add<Button>>,
    selects: Query<(), With<SelectBox>>,
    mut commands: Commands,
) {
    let button = added.entity;
    let is_select = selects.contains(button);
    let label = commands.spawn((text(), ChildOf(button))).id();
    let chevron = is_select.then(|| {
        commands
            .spawn((
                text(),
                Node {
                    margin: UiRect::left(Val::Auto),
                    ..Node::default()
                },
                ChildOf(button),
            ))
            .id()
    });
    let padding = if is_select { 1.0 } else { BUTTON_PADDING };
    commands
        .entity(button)
        .insert(Labelled { label, chevron })
        .entry::<Node>()
        .or_default()
        .and_modify(move |mut node| {
            box_in(&mut node, padding);
            if is_select {
                node.flex_grow = 1.0;
            }
        });
}

/// The label in its box: ringed in the ink while the keyboard is on it,
/// and bold in the accent too unless the box is filled; a primary answer
/// filled with the accent and said in the ground's ink, a destructive one
/// edged in the warning's; otherwise in the style its emphasis gives it.
fn paint_buttons(
    theme: Res<Theme>,
    focus: Res<InputFocus>,
    buttons: Query<
        (
            Entity,
            Ref<UiLabel>,
            Option<Ref<UiStyle>>,
            Option<&Emphasis>,
            Ref<Labelled>,
        ),
        With<Button>,
    >,
    mut commands: Commands,
) {
    for (button, label, style, emphasis, labelled) in &buttons {
        let has_changed = focus.is_changed()
            || theme.is_changed()
            || label.is_changed()
            || labelled.is_added()
            || style.as_ref().is_some_and(DetectChanges::is_changed);
        if !has_changed {
            continue;
        }
        let is_held = focus.get() == Some(button);
        let is_filled = emphasis == Some(&Emphasis::Primary);
        let edge = match emphasis {
            _ if is_held => theme.ink(Style::new()),
            Some(Emphasis::Primary) => theme.ink(theme.accented()),
            Some(Emphasis::Destructive) => theme.ink(theme.exceeded()),
            None => theme.ink(theme.bordered()),
        };
        let fill = if is_filled {
            theme.ink(theme.accented())
        } else {
            Color::NONE
        };
        commands
            .entity(button)
            .insert((BorderColor::all(edge), BackgroundColor(fill)));
        let worn = if is_filled {
            Style::new().fg(theme.bg)
        } else if is_held && labelled.chevron.is_none() {
            theme.focused()
        } else {
            style.map_or_else(Default::default, |style| style.0)
        };
        let mut line = label.0.clone();
        if let Some(chevron) = labelled.chevron {
            line.spans.pop();
            if theme.is_changed() || labelled.is_added() {
                let chevron_line = Line::styled(CHEVRON, theme.dimmed());
                say(&mut commands, chevron, chevron_line, &theme);
            }
        }
        say(
            &mut commands,
            labelled.label,
            line.patch_style(worn),
            &theme,
        );
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
    commands.entity(toast).insert((
        Node {
            padding: UiRect::axes(Val::Em(1.0), Val::Rem(1.0)),
            border: UiRect::all(HAIRLINE),
            border_radius: BorderRadius::all(HALF_CELL_WIDE),
            ..Node::default()
        },
        BorderColor::all(edge),
        BackgroundColor(theme.ground()),
    ));
    say(&mut commands, line, label.0.clone(), &theme);
}

#[cfg(test)]
mod tests {
    use crate::support::typed;
    use bevy_ecs::hierarchy::Children;
    use bevy_ecs::observer::Observer;
    use bevy_ecs::prelude::World;
    use bevy_ecs::system::RunSystemOnce;
    use bevy_ui::OverflowAxis;
    use plurimus::bui::bevy_text::RemSize;
    use plurimus::core::ratatui_core::layout::Position;
    use plurimus::ui::{ModalOpen, ScrollArea};

    use super::*;
    use crate::support::{SIZE, headless_app, press_key, said_by};

    const REM: f32 = 20.0;

    #[test]
    fn a_button_is_a_box_without_brackets_and_a_primary_one_is_filled() {
        let mut world = World::new();
        world.insert_resource(Theme::default());
        world.insert_resource(InputFocus::default());
        world.spawn(Observer::new(label_the_button));
        let plain = world
            .spawn((Button, UiLabel(Line::from("Cancel")), Node::default()))
            .id();
        let primary = world
            .spawn((
                Button,
                Emphasis::Primary,
                UiLabel(Line::from("Apply")),
                Node::default(),
            ))
            .id();
        world.flush();
        world.run_system_once(paint_buttons).unwrap();
        world.flush();
        let theme = world.resource::<Theme>().clone();
        let label = |button| world.get::<Labelled>(button).unwrap().label;
        assert_eq!(said_by(&world, label(plain)), "Cancel");
        assert_eq!(said_by(&world, label(primary)), "Apply");
        let node = world.get::<Node>(plain).unwrap();
        assert_eq!(node.border, UiRect::all(HAIRLINE));
        assert_eq!(node.padding, UiRect::horizontal(Val::Em(2.0)));
        let fill = |button| world.get::<BackgroundColor>(button).unwrap().0;
        assert_eq!(fill(plain), Color::NONE);
        assert_eq!(fill(primary), theme.ink(theme.accented()));
    }

    #[test]
    fn the_rows_a_list_is_scrolled_by_scroll_its_node_a_rem_each() {
        let mut world = World::new();
        let list = world
            .spawn((
                ListBox,
                ScrollOffset(Position::new(0, 3)),
                ComputedNode {
                    rem_size: RemSize(REM),
                    ..ComputedNode::default()
                },
                ScrollPosition::default(),
            ))
            .id();
        world.run_system_once(scroll_the_list).unwrap();
        let top = world.get::<ScrollPosition>(list).unwrap().0.y;
        assert!((top - 3.0 * REM).abs() < f32::EPSILON, "{top}");
    }

    #[test]
    fn the_issues_panel_s_list_stacks_its_rows_as_nodes_that_scroll() {
        let mut app = headless_app(SIZE);
        app.add_plugins(plugin);
        press_key(&mut app, typed('i'));
        let world = app.world_mut();
        let (node, children) = world
            .query_filtered::<(&Node, &Children, &ChildOf), (With<ListBox>, With<ScrollArea>)>()
            .iter(world)
            .find(|(_, _, parent)| world.entity(parent.parent()).contains::<ModalOpen>())
            .map(|(node, children, _)| (node, children))
            .expect("the issues panel's list is the one scrolled list under an overlay");
        assert_eq!(node.flex_direction, FlexDirection::Column);
        assert_eq!(node.overflow.y, OverflowAxis::Scroll);
        assert!(!children.is_empty());
        assert!(
            children
                .iter()
                .all(|&row| world.entity(row).contains::<Node>()),
            "every row is a node under the list"
        );
    }
}
