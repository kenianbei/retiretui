//! The bar beside a scrolled list in the window: the grid's bar hung in
//! the list's last column, kept in place as the rows scroll under it,
//! and said again whenever the rows, the list's height or its offset
//! move.

use bevy_app::{App, PreUpdate};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Add, Bundle, Commands, Component, Entity, On, Query, Res, With};
use bevy_math::BVec2;
use bevy_ui::{ComputedNode, IgnoreScroll, Node, PositionType, Val};
use plurimus::ui::{ScrollArea, ScrollOffset};
use plurimus::widgets::{ListBox, ListItem};

use crate::grid::bar::{self, Bar};
use crate::layout::{SCROLL_BAR_COLS, node_rows};
use crate::theme::Theme;

pub fn plugin(app: &mut App) {
    app.add_observer(bar_the_list);
    // Before the frame's fillers respawn a list's rows: a column found
    // among them here is still there when the bar is said down it.
    app.add_systems(PreUpdate, paint_bars);
}

/// A scrolled list's bar and the column it is said down, spawned again
/// once the rows it stood among are respawned.
#[derive(Component, Debug, Default)]
struct Barred {
    bar: Bar,
    column: Option<Entity>,
}

/// A scrolled list keeps its last column for the bar, as the terminal's
/// scroll area does.
fn bar_the_list(
    added: On<Add<ScrollArea>>,
    lists: Query<(), With<ListBox>>,
    mut commands: Commands,
) {
    let list = added.entity;
    if !lists.contains(list) {
        return;
    }
    commands
        .entity(list)
        .insert(Barred::default())
        .entry::<Node>()
        .or_default()
        .and_modify(|mut node| node.padding.right = Val::Em(f32::from(SCROLL_BAR_COLS)));
}

/// The bar's column: down the list's right edge, as tall as the list,
/// staying put as the rows scroll.
fn column_bundle() -> impl Bundle {
    let node = Node {
        position_type: PositionType::Absolute,
        right: Val::ZERO,
        top: Val::ZERO,
        bottom: Val::ZERO,
        ..bar::column()
    };
    (node, IgnoreScroll(BVec2::TRUE))
}

/// Says each list's bar again where its rows, height or offset moved, or
/// its column went with the rows; a theme change starts the column over.
fn paint_bars(
    theme: Res<Theme>,
    mut lists: Query<
        (
            Entity,
            Ref<ScrollOffset>,
            Ref<ComputedNode>,
            Option<Ref<Children>>,
            &mut Barred,
        ),
        With<ListBox>,
    >,
    items: Query<(), With<ListItem>>,
    mut commands: Commands,
) {
    for (list, offset, computed, children, mut barred) in &mut lists {
        let rows = children.as_deref();
        let column =
            (barred.column).filter(|column| rows.is_some_and(|rows| rows.contains(column)));
        let has_moved = offset.is_changed()
            || computed.is_changed()
            || children.as_ref().is_some_and(DetectChanges::is_changed);
        if column.is_some() && !has_moved && !theme.is_changed() {
            continue;
        }
        let column = match column {
            Some(column) if !theme.is_changed() => column,
            stale => {
                if let Some(stale) = stale {
                    commands.entity(stale).despawn();
                }
                barred.bar = Bar::default();
                let column = commands.spawn((column_bundle(), ChildOf(list))).id();
                barred.column = Some(column);
                column
            }
        };
        let rows = rows.map_or(0, |rows| {
            rows.iter().filter(|&&row| items.contains(row)).count()
        });
        let rows = u16::try_from(rows).unwrap_or(u16::MAX);
        barred.bar.redraw(
            (rows, node_rows(&computed), offset.0.y),
            column,
            &mut commands,
            &theme,
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::observer::Observer;
    use bevy_ecs::prelude::World;
    use bevy_ecs::system::SystemId;
    use bevy_math::Vec2;
    use plurimus::bui::bevy_text::RemSize;
    use plurimus::core::ratatui_core::layout::{Position, Size};
    use plurimus::core::ratatui_core::text::Line;
    use plurimus::widgets::list_item;

    use super::*;
    use crate::support::said_by;

    const REM: f32 = 20.0;

    fn rows_tall(rows: f32) -> ComputedNode {
        ComputedNode {
            rem_size: RemSize(REM),
            size: Vec2::new(40.0 * REM, rows * REM),
            ..ComputedNode::default()
        }
    }

    fn ten_rows(world: &mut World, list: Entity) {
        for row in 0..10 {
            world.spawn((list_item(Line::from(row.to_string())), ChildOf(list)));
        }
        world.flush();
    }

    /// One registered painter, so its second run sees only what moved
    /// since its first - as a scheduled system does.
    fn painter(world: &mut World) -> SystemId {
        world.insert_resource(Theme::default());
        world.spawn(Observer::new(bar_the_list));
        world.register_system(paint_bars)
    }

    fn paint(world: &mut World, painter: SystemId) {
        world.run_system(painter).unwrap();
        world.flush();
        world.increment_change_tick();
    }

    /// The glyphs said down `list`'s bar.
    fn bar_glyphs(world: &World, list: Entity) -> Vec<String> {
        let column = world.get::<Barred>(list).unwrap().column.unwrap();
        let rows = world.get::<Children>(column);
        rows.into_iter()
            .flat_map(|rows| rows.iter())
            .map(|&row| said_by(world, row))
            .collect()
    }

    #[test]
    fn a_list_taller_than_its_node_says_the_bar_beside_it_and_a_fitting_one_none() {
        let mut world = World::new();
        let painter = painter(&mut world);
        let list = world
            .spawn((
                ListBox,
                ScrollArea::new(Size::default()),
                ScrollOffset(Position::new(0, 2)),
                rows_tall(4.0),
            ))
            .id();
        ten_rows(&mut world, list);
        paint(&mut world, painter);
        assert_eq!(bar_glyphs(&world, list), bar::glyphs(6, 4, 2));
        let column = world.get::<Barred>(list).unwrap().column.unwrap();
        assert_eq!(world.get::<ChildOf>(column).unwrap().parent(), list);
        assert!(world.entity(column).contains::<IgnoreScroll>(), "stays put");
        assert_eq!(
            world.get::<Node>(list).unwrap().padding.right,
            Val::Em(1.0),
            "the rows keep clear of the bar"
        );

        world.entity_mut(list).insert(rows_tall(12.0));
        paint(&mut world, painter);
        assert!(bar_glyphs(&world, list).is_empty(), "every row fits");
    }

    #[test]
    fn a_list_whose_rows_are_respawned_gets_its_bar_back() {
        let mut world = World::new();
        let painter = painter(&mut world);
        let list = world
            .spawn((
                ListBox,
                ScrollArea::new(Size::default()),
                ScrollOffset(Position::new(0, 0)),
                rows_tall(4.0),
            ))
            .id();
        ten_rows(&mut world, list);
        paint(&mut world, painter);
        let first = world.get::<Barred>(list).unwrap().column.unwrap();
        world.entity_mut(list).despawn_related::<Children>();
        ten_rows(&mut world, list);
        paint(&mut world, painter);
        let second = world.get::<Barred>(list).unwrap().column.unwrap();
        assert_ne!(first, second, "the column went with the rows");
        assert_eq!(bar_glyphs(&world, list), bar::glyphs(6, 4, 0));
    }
}
