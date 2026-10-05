//! The Overview: the page the app opens on, answering what a loaded plan
//! comes to as a whole - the verdict, then when the big things happen,
//! what needs attention and what could do better, then the money through
//! the years beside what they add up to and what it all rests on. Nothing
//! on it is chosen by a year.

mod attention;
mod better;
mod charts;
mod rows;
mod totals;
mod verdict;

#[cfg(test)]
mod better_tests;
#[cfg(test)]
mod tests;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Entity, IntoScheduleConfigs, Query, ResMut, Resource, With};
use bevy_ui::{FlexDirection, Node, Val};
use retiretui_client::overview::View;

pub(crate) use better::Better;
#[cfg(test)]
pub(crate) use charts::OverviewChart;
pub(crate) use charts::cycle_chart;
use rows::List;
pub(crate) use rows::edit_row;

use super::layout::{self, Body};
use super::nav::{self, Page};
use super::session::Shown;

pub fn plugin(app: &mut App) {
    app.init_resource::<charts::ChartView>();
    app.init_resource::<Better>();
    app.init_resource::<Viewed>();
    app.add_systems(Startup, spawn_overview.after(layout::spawn_frame));
    app.add_systems(
        Update,
        (
            view,
            (
                verdict::refresh,
                (better::work, rows::refresh, rows::read_beneath).chain(),
                (charts::refresh, charts::read_key).chain(),
            ),
        )
            .chain(),
    );
}

/// The page as the client says it, of the plan and the basis shown.
#[derive(Resource, Default)]
pub(crate) struct Viewed(Option<View>);

/// Says the page again whenever the plan or the basis moves.
fn view(shown: Shown, mut viewed: ResMut<Viewed>) {
    if shown.projected.is_changed() || shown.basis.is_changed() || viewed.0.is_none() {
        viewed.0 = Some(View::new(&shown.projected, shown.basis.nominal));
    }
}

/// What a pane says before anything fills it.
const PENDING: &str = "…";

/// The rows each band keeps however short the terminal, borders
/// included, and the share of the page it takes. Three to ten never
/// leaves half a row, which would round both bands up past the page.
const LISTS_BAND: (f32, f32) = (6.0, 3.0);
const CHART_BAND: (f32, f32) = (10.0, 10.0);
/// How the chart and what is beside it split their band.
const CHART_SHARE: f32 = 3.0;
const BESIDE_SHARE: f32 = 2.0;

fn spawn_overview(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let view = nav::spawn_surface(&mut commands, body, Some(Page::Overview));
    verdict::spawn(&mut commands, view);
    let lists = spawn_band(&mut commands, view, LISTS_BAND);
    let across = [List::Milestones, List::Attention, List::Better];
    rows::spawn(&mut commands, lists, &across);
    let charted = spawn_band(&mut commands, view, CHART_BAND);
    charts::spawn(&mut commands, charted, CHART_SHARE);
    let beside = Node {
        flex_direction: FlexDirection::Column,
        flex_grow: BESIDE_SHARE,
        flex_basis: Val::Px(0.0),
        ..Node::default()
    };
    let beside = commands.spawn((beside, ChildOf(charted))).id();
    rows::spawn(&mut commands, beside, &[List::Totals, List::RestsOn]);
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
