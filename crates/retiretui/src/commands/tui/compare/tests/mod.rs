use bevy_app::App;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::{Component, Or};
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;
use plurimus::widgets::ActiveDescendant;

use super::by_year::ByYearTable;
use super::changes::ChangesList;
use super::plans::PlansTable;
use super::views::{CompareChart, GridChart};
use super::*;
use crate::commands::tui::pane::Framed;
use crate::commands::tui::session::{Projected, RowYear, YearCursor};
use crate::commands::tui::support::{
    ROOMY, SIZE, cell_fg, cell_of, commit_edit, composed_frame, headless_app_at, press_key,
    redrawn, run_command, said, scratch_workspace, show, test_plan_briefly_run, type_text,
};

fn compared(app: &App) -> Vec<String> {
    app.world()
        .resource::<Compared>()
        .docs
        .iter()
        .map(|doc| session::file_name(&doc.path).into_owned())
        .collect()
}

/// Runs the Compare with… picker and chooses `name`.
fn compare_with(app: &mut App, name: &str) {
    run_command(app, "compare-with");
    type_text(app, name);
    press_key(app, KeyCode::Enter);
    app.update();
}

fn series_count(app: &mut App) -> usize {
    let mut charts = app
        .world_mut()
        .query_filtered::<&SeriesChart, With<CompareChart>>();
    charts.single(app.world()).unwrap().series.len()
}

/// The Compare page over the test plan, `variant.toml` compared with it.
fn comparing_variant(
    size: plurimus::core::TerminalSize,
) -> crate::commands::tui::support::Headless {
    let dir = scratch_workspace(&test_plan_briefly_run());
    let mut app = headless_app_at(dir.join("plan.toml"), size);
    show(&mut app, Page::Compare);
    compare_with(&mut app, "variant");
    redrawn(&mut app);
    app
}

fn single<C: Component>(app: &mut App) -> Entity {
    let mut query = app.world_mut().query_filtered::<Entity, With<C>>();
    query.single(app.world()).unwrap()
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn title_of(app: &mut App, marker: Entity) -> String {
    let pane = app.world().get::<ChildOf>(marker).unwrap().parent();
    app.world().get::<Framed>(pane).unwrap().title.clone()
}

fn view_title(app: &mut App) -> String {
    let chart = single::<CompareChart>(app);
    title_of(app, chart)
}

fn is_shown(app: &App, part: Entity) -> bool {
    app.world().get::<bevy_ui::Node>(part).unwrap().display != bevy_ui::Display::None
}

fn grid_part(app: &mut App) -> Entity {
    let mut charts = app
        .world_mut()
        .query_filtered::<&ChildOf, With<GridChart>>();
    let small = charts.iter(app.world()).next().unwrap().parent();
    let across = app.world().get::<ChildOf>(small).unwrap().parent();
    app.world().get::<ChildOf>(across).unwrap().parent()
}

fn cursor_year(app: &App) -> Option<i16> {
    app.world().resource::<YearCursor>().0
}

fn set_cursor(app: &mut App, year: i16) {
    app.world_mut().resource_mut::<YearCursor>().0 = Some(year);
    redrawn(app);
}

mod changes;
mod cursor;
mod difference;
mod disk;
mod opening;
mod plans_table;
mod success;
mod view_pane;
