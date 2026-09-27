//! The Overview: the page the app opens on, answering in bands what a
//! loaded plan comes to - the verdict, when the big things happen, what
//! needs attention, the money year by year beside what to do in the
//! cursor's year, and what could do better.

mod attention;
mod better;
mod charts;
mod milestones;
mod rows;
mod todo;
mod verdict;

#[cfg(test)]
mod better_tests;
#[cfg(test)]
mod tests;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Entity, IntoScheduleConfigs, Query, Res, ResMut, With};
use bevy_ui::{FlexDirection, Node, Val};

pub(crate) use better::Better;
#[cfg(test)]
pub(crate) use charts::OverviewChart;
pub(crate) use charts::cycle_chart;
use rows::List;
pub(crate) use rows::edit_row;
pub(crate) use todo::WARNING_MARK;

use super::command::Outcome;
use super::hints::Hints;
use super::layout::{self, Body};
use super::nav::{self, Page};
use super::pane::Pane;
use super::session::{Projected, Today, YearCursor, cursor_year, span};

pub fn plugin(app: &mut App) {
    app.init_resource::<charts::ChartView>();
    app.init_resource::<Better>();
    app.add_systems(Startup, spawn_overview.after(layout::spawn_frame));
    app.add_systems(
        Update,
        (
            verdict::refresh,
            todo::refresh,
            (better::work, rows::refresh).chain(),
            charts::refresh,
        ),
    );
}

/// Walking the year is the page's, from whichever pane holds the keys.
const YEAR_HINTS: Hints = Hints(&[("←→", "year")]);
const SCROLL_HINTS: Hints = Hints(&[("↑↓", "scroll")]);
/// What a pane says before anything fills it.
const PENDING: &str = "…";

/// The rows each band keeps however short the terminal, borders
/// included, and the share of what is left over it takes.
const LISTS_BAND: (f32, f32) = (6.0, 1.0);
const CHART_BAND: (f32, f32) = (10.0, 3.0);
const BETTER_BAND: (f32, f32) = (4.0, 1.0);
/// How the chart and To do split their band.
const CHART_SHARE: f32 = 3.0;
const TODO_SHARE: f32 = 2.0;

fn spawn_overview(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let view = nav::spawn_surface(&mut commands, body, Some(Page::Overview));
    commands.entity(view).insert(YEAR_HINTS);
    verdict::spawn(&mut commands, view);
    let lists = spawn_band(&mut commands, view, LISTS_BAND);
    rows::spawn(&mut commands, lists, &[List::Milestones, List::Attention]);
    let charted = spawn_band(&mut commands, view, CHART_BAND);
    charts::spawn(&mut commands, charted, CHART_SHARE);
    todo::spawn(&mut commands, charted, TODO_SHARE);
    let better = spawn_band(&mut commands, view, BETTER_BAND);
    rows::spawn(&mut commands, better, &[List::Better]);
}

/// A row of panes under `view`, `(least, share)` of the page tall.
fn spawn_band(commands: &mut Commands, view: Entity, (least, share): (f32, f32)) -> Entity {
    let band = Node {
        flex_direction: FlexDirection::Row,
        flex_grow: share,
        flex_basis: Val::Px(0.0),
        min_height: Val::Px(least),
        ..Node::default()
    };
    commands.spawn((band, ChildOf(view))).id()
}

/// A list pane in `band` sharing it evenly, its rows scrolled through.
fn spawn_list(commands: &mut Commands, band: Entity, title: &str, share: f32) -> Entity {
    let pane = Pane::new(title).sharing(share).spawn(commands, band);
    layout::spawn_scrolled_list(commands, pane, SCROLL_HINTS)
}

/// Moves the year cursor `step` years within the projected ones.
fn step_year(
    projected: &Projected,
    today: Today,
    cursor: &mut ResMut<YearCursor>,
    step: i16,
) -> Outcome {
    let planned = span(&projected.projection.years);
    let year = cursor_year(projected, today, **cursor);
    let stepped = (year + step).clamp(planned.0, planned.1);
    if stepped != year {
        cursor.set_if_neq(YearCursor(Some(stepped)));
    }
    Outcome::Done
}

/// The `overview-year-next` command.
pub(crate) fn next_year(
    projected: Res<Projected>,
    today: Res<Today>,
    mut cursor: ResMut<YearCursor>,
) -> Outcome {
    step_year(&projected, *today, &mut cursor, 1)
}

/// The `overview-year-previous` command.
pub(crate) fn previous_year(
    projected: Res<Projected>,
    today: Res<Today>,
    mut cursor: ResMut<YearCursor>,
) -> Outcome {
    step_year(&projected, *today, &mut cursor, -1)
}
