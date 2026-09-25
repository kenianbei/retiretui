//! Where the year cursor stands, as each view shows it.

use bevy_app::App;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Entity, With};
use plurimus::ui::ComputedWidgetArea;
use plurimus::widgets::ActiveDescendant;

use crate::commands::tui::chart::SeriesChart;
use crate::commands::tui::ledger::LedgerTable;
use crate::commands::tui::nav::Page;
use crate::commands::tui::overview::OverviewChart;
use crate::commands::tui::session::{RowYear, YearCursor};

use super::{click, composed_frame, show};

/// Clicks `chart` in the column that reads as `year`.
pub fn click_year(app: &mut App, chart: Entity, year: i16) {
    let area = app.world().get::<ComputedWidgetArea>(chart).unwrap().0;
    let chart_drawn = app.world().get::<SeriesChart>(chart).unwrap();
    let column = chart_drawn
        .column_of(area, year)
        .expect("the year is charted");
    click(app, column, area.y + 1);
}

/// The year the Ledger's table is on.
pub fn ledger_year(app: &mut App) -> i16 {
    let mut tables = app
        .world_mut()
        .query_filtered::<&ActiveDescendant, With<LedgerTable>>();
    let row = tables.single(app.world()).unwrap().0.expect("a row is on");
    app.world().get::<RowYear>(row).unwrap().0
}

/// The year the Overview shows, which its To do and its chart's cursor
/// mark agree on.
pub fn overview_year(app: &mut App) -> i16 {
    show(app, Page::Overview);
    let frame = composed_frame(app);
    let (heading, _) = (frame.lines())
        .find_map(|line| line.split_once(" · to do"))
        .unwrap_or_else(|| panic!("the To do's year: {frame}"));
    let year: i16 = heading[heading.len() - 4..].parse().unwrap();
    let chart = overview_chart(app);
    let label = year.to_string();
    let chart = app.world().get::<SeriesChart>(chart).unwrap();
    let is_marked = chart.marks.iter().any(|mark| mark.label == label);
    assert!(is_marked, "the cursor mark is on {year}");
    year
}

pub fn overview_chart(app: &mut App) -> Entity {
    let mut charts = app
        .world_mut()
        .query_filtered::<Entity, With<OverviewChart>>();
    charts.single(app.world()).unwrap()
}

const TICKS_AT_REST: usize = 10;

/// Asserts the cursor stays unchanged through frames with nothing to do.
pub fn assert_at_rest(app: &mut App) {
    let settled = app.world().resource_ref::<YearCursor>().last_changed();
    for _ in 0..TICKS_AT_REST {
        app.update();
    }
    let rested = app.world().resource_ref::<YearCursor>().last_changed();
    assert_eq!(settled, rested, "the cursor was written at rest");
}
