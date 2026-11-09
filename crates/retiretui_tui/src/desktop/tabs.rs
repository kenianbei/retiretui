//! The window's tab row and key row: the tab row's height and baseline,
//! the status on its middle, each tab's label and underline, which a click
//! chooses as a terminal's does, and the key row over a hairline, its keys
//! on its middle and in from the edge as far as the tabs.

use bevy_app::{App, Update};
use bevy_color::Color;
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Commands, Component, Entity, Has, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_ui::{BorderColor, FlexDirection, JustifyContent, Node, UiRect, Val};
use plurimus::ui::{InteractionDisabled, Selected, UiLabel};

use super::chrome::{HAIRLINE, MARGIN, Ruled};
use crate::hints::EDGE;
use crate::layout::{HintRow, TabRow};
use crate::tabbar::BarTab;
use crate::tabbar::status::StatusCell;
use crate::theme::{Repainted, Theme, say, text};

/// The line under the chosen tab.
#[expect(clippy::disallowed_methods, reason = "an underline is two pixels")]
const UNDERLINE: Val = Val::Px(2.0);
/// Rows the tab row takes in the window, and the key row: a line on
/// either is padded onto the middle row; the keys are in from the edge
/// by the body's margin less the edge the row keeps itself.
const TAB_ROWS: f32 = 2.0;
const ON_MIDDLE: Val = Val::Rem(TAB_ROWS / 2.0 - 0.5);
const KEY_INSET: u16 = MARGIN.0 - EDGE as u16;
/// Cells between one tab and the next, and before the first, which is
/// the body's margin.
const TAB_GAP: f32 = MARGIN.0 as f32;

pub fn plugin(app: &mut App) {
    app.add_observer(line_the_tab_row);
    app.add_observer(line_the_key_row);
    app.add_observer(centre_the_status);
    app.add_observer(underline_the_tab);
    app.add_systems(Update, paint_tabs.in_set(Repainted));
}

/// The tab row is two rows tall with a hairline along its foot, the
/// baseline the chosen tab's underline sits on.
fn line_the_tab_row(added: On<Add<TabRow>>, mut nodes: Query<&mut Node>, mut commands: Commands) {
    if let Ok(mut node) = nodes.get_mut(added.entity) {
        node.height = Val::Rem(TAB_ROWS);
        node.border = UiRect::bottom(HAIRLINE);
    }
    commands.entity(added.entity).insert(Ruled);
}

/// The key row is as tall as the tab row under a hairline, its keys on its
/// middle and in from the edge by what the row's fitting leaves clear.
fn line_the_key_row(added: On<Add<HintRow>>, mut nodes: Query<&mut Node>, mut commands: Commands) {
    if let Ok(mut node) = nodes.get_mut(added.entity) {
        node.height = Val::Rem(TAB_ROWS);
        node.padding = UiRect::horizontal(Val::Em(f32::from(KEY_INSET))).with_top(ON_MIDDLE);
        node.border = UiRect::top(HAIRLINE);
    }
    commands.entity(added.entity).insert(Ruled);
}

/// The status says its line on the row's middle, as the tabs do.
fn centre_the_status(added: On<Add<StatusCell>>, mut nodes: Query<&mut Node>) {
    if let Ok(mut node) = nodes.get_mut(added.entity) {
        node.padding = UiRect::top(ON_MIDDLE);
    }
}

/// A tab's label, and how the tab was last drawn.
#[derive(Component, Debug)]
struct Underlined {
    label: Entity,
    is_lit: bool,
    is_dead: bool,
}

/// A tab is its label, as tall as the row and centred on it, a gap in
/// from the tab before it, with an underline along the row's foot that
/// shows only on the chosen tab.
fn underline_the_tab(added: On<Add<BarTab>>, mut commands: Commands) {
    let tab = added.entity;
    let label = commands.spawn((text(), ChildOf(tab))).id();
    commands.entity(tab).insert((
        Node {
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            margin: UiRect::left(Val::Em(TAB_GAP)),
            border: UiRect::bottom(UNDERLINE),
            ..Node::default()
        },
        Underlined {
            label,
            is_lit: false,
            is_dead: false,
        },
    ));
}

fn paint_tabs(
    theme: Res<Theme>,
    mut tabs: Query<
        (
            Entity,
            Ref<UiLabel>,
            Has<Selected>,
            Has<InteractionDisabled>,
            &mut Underlined,
        ),
        With<BarTab>,
    >,
    mut commands: Commands,
) {
    for (tab, label, is_lit, is_dead, mut drawn) in &mut tabs {
        let is_as_drawn = drawn.is_lit == is_lit && drawn.is_dead == is_dead;
        if is_as_drawn && !theme.is_changed() && !label.is_changed() && !drawn.is_added() {
            continue;
        }
        drawn.is_lit = is_lit;
        drawn.is_dead = is_dead;
        let underline = if is_lit {
            theme.ink(theme.accented())
        } else {
            Color::NONE
        };
        commands.entity(tab).insert(BorderColor::all(underline));
        let line = if is_lit {
            label.0.clone().patch_style(theme.focused())
        } else if is_dead {
            label.0.clone().patch_style(theme.dimmed())
        } else {
            label.0.clone()
        };
        say(&mut commands, drawn.label, line, &theme);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::observer::Observer;
    use bevy_ecs::prelude::World;
    use bevy_ecs::system::RunSystemOnce;
    use plurimus::core::ratatui_core::text::Line;

    use super::*;
    use crate::layout::fixed;

    fn tabbed() -> World {
        let mut world = World::new();
        world.insert_resource(Theme::default());
        for observer in [
            Observer::new(line_the_tab_row),
            Observer::new(line_the_key_row),
            Observer::new(centre_the_status),
            Observer::new(underline_the_tab),
        ] {
            world.spawn(observer);
        }
        world
    }

    #[test]
    fn the_tab_row_is_two_rows_over_a_baseline_and_the_status_sits_on_its_middle() {
        let mut world = tabbed();
        let row = world.spawn((TabRow, fixed(3.0))).id();
        let status = world.spawn((StatusCell, fixed(1.0))).id();
        world.flush();
        let row = world.get::<Node>(row).unwrap();
        assert_eq!(row.height, Val::Rem(2.0));
        assert_eq!(row.border, UiRect::bottom(HAIRLINE));
        let status = world.get::<Node>(status).unwrap();
        assert_eq!(status.padding.top, Val::Rem(0.5));
    }

    #[test]
    fn the_key_row_is_as_tall_over_a_hairline_with_its_keys_in_from_the_edge() {
        let mut world = tabbed();
        let row = world.spawn((HintRow, fixed(1.0))).id();
        world.flush();
        let row = world.get::<Node>(row).unwrap();
        assert_eq!(row.height, Val::Rem(2.0));
        assert_eq!(row.border, UiRect::top(HAIRLINE));
        assert_eq!(
            (row.padding.left, row.padding.top),
            (Val::Em(2.0), Val::Rem(0.5))
        );
    }

    #[test]
    fn a_tab_is_its_label_underlined_only_while_chosen() {
        let mut world = tabbed();
        let bar = world.spawn(Node::default()).id();
        let chosen = world
            .spawn((
                BarTab(0),
                UiLabel(Line::from("1 Overview")),
                Selected,
                ChildOf(bar),
            ))
            .id();
        let other = world
            .spawn((BarTab(1), UiLabel(Line::from("2 Ledger")), ChildOf(bar)))
            .id();
        world.flush();
        world.run_system_once(paint_tabs).unwrap();
        world.flush();
        let node = world.get::<Node>(chosen).unwrap();
        assert_eq!(node.border, UiRect::bottom(UNDERLINE));
        assert_eq!(node.padding, UiRect::ZERO, "no box");
        assert_eq!(node.margin, UiRect::left(Val::Em(TAB_GAP)));
        let theme = world.resource::<Theme>().clone();
        let accent = theme.ink(theme.accented());
        let edge = |tab| world.get::<BorderColor>(tab).unwrap().bottom;
        assert_eq!(edge(chosen), accent);
        assert_eq!(edge(other), Color::NONE);
    }
}
