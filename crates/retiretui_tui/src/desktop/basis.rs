//! The window's switch between today's and future dollars, at the tab
//! row's far end: both bases named, the one in force lit, and a click
//! running the `basis` command the terminal's key runs.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_ui::{AlignSelf, Node, UiRect, Val};
use plurimus::bui::bevy_picking::events::PointerClick;

use super::chrome::MARGIN;
use super::pill::{SEGMENT_PADDING, lift, pill, segment};
use crate::command::{self, defer_named};
use crate::session::Basis;
use crate::tabbar::status::{Reserved, StatusCell};
use crate::theme::{Repainted, Theme, text};

/// What each base is called on the switch.
const TODAYS: &str = "Today's";
const FUTURE: &str = "Future";
/// Cells the switch takes with the body's margin after it, which the
/// status leaves it.
const COLS: u16 = (TODAYS.len() + FUTURE.len()) as u16 + 4 * SEGMENT_PADDING as u16 + MARGIN.0;

#[derive(Component, Debug)]
struct Switch;

/// A segment of the switch: its name, and the basis it stands for.
#[derive(Component, Debug)]
struct Segment {
    name: &'static str,
    is_nominal: bool,
}

pub fn plugin(app: &mut App) {
    app.insert_resource(Reserved(COLS));
    app.add_observer(spawn_beside_the_status);
    app.add_systems(Update, paint_switches.in_set(Repainted));
}

/// The switch follows the status on its row: a raised pill on the row's
/// middle, a segment for each basis.
fn spawn_beside_the_status(
    added: On<Add<StatusCell>>,
    rows: Query<&ChildOf, With<StatusCell>>,
    mut commands: Commands,
) {
    let Ok(row) = rows.get(added.entity) else {
        return;
    };
    let node = Node {
        align_self: AlignSelf::Center,
        margin: UiRect::right(Val::Em(f32::from(MARGIN.0))),
        ..Node::default()
    };
    let switch = commands
        .spawn((Switch, pill(node), ChildOf(row.parent())))
        .observe(toggle_on_click)
        .id();
    for (name, is_nominal) in [(TODAYS, false), (FUTURE, true)] {
        commands.spawn((
            Segment { name, is_nominal },
            text(),
            segment(),
            ChildOf(switch),
        ));
    }
}

fn toggle_on_click(_click: On<PointerClick>, mut commands: Commands) {
    defer_named(&mut commands, command::BASIS);
}

/// The segment in force lifted off the pill and said in the ink, the
/// other dimmed on it.
fn paint_switches(
    theme: Res<Theme>,
    basis: Res<Basis>,
    segments: Query<(Entity, Ref<Segment>)>,
    mut commands: Commands,
) {
    for (segment, which) in &segments {
        if !theme.is_changed() && !basis.is_changed() && !which.is_added() {
            continue;
        }
        let is_chosen = which.is_nominal == basis.nominal;
        lift(&mut commands, segment, which.name, is_chosen, &theme);
    }
}

#[cfg(test)]
mod tests {
    use bevy_color::Color;
    use bevy_ecs::observer::Observer;
    use bevy_ecs::prelude::World;
    use bevy_ecs::system::RunSystemOnce;

    use super::super::pill::lifted;
    use super::*;
    use crate::support::said_by;
    use bevy_ui::BackgroundColor;

    #[test]
    fn the_switch_follows_the_status_and_lifts_the_basis_in_force() {
        let mut world = World::new();
        world.insert_resource(Theme::default());
        world.insert_resource(Basis::default());
        world.spawn(Observer::new(spawn_beside_the_status));
        let row = world.spawn(Node::default()).id();
        world.spawn((StatusCell, Node::default(), ChildOf(row)));
        world.flush();
        let switch = world
            .query_filtered::<&ChildOf, With<Switch>>()
            .single(&world)
            .map(ChildOf::parent)
            .unwrap();
        assert_eq!(switch, row);
        world.run_system_once(paint_switches).unwrap();
        world.flush();
        let theme = world.resource::<Theme>().clone();
        let mut said: Vec<(bool, String, Color)> = world
            .query::<(Entity, &Segment)>()
            .iter(&world)
            .map(|(segment, which)| {
                let fill = world.get::<BackgroundColor>(segment).unwrap().0;
                (which.is_nominal, said_by(&world, segment), fill)
            })
            .collect();
        said.sort_by_key(|(is_nominal, ..)| *is_nominal);
        assert_eq!(
            said,
            [
                (false, TODAYS.to_owned(), lifted(&theme)),
                (true, FUTURE.to_owned(), Color::NONE),
            ]
        );
        assert_eq!(
            usize::from(COLS - MARGIN.0),
            TODAYS.len() + FUTURE.len() + 4,
            "two cells of padding a segment"
        );
    }
}
