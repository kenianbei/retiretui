//! Where the year cursor stands, as each view shows it.

use bevy_app::App;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Entity, With};
use plurimus::ui::ComputedWidgetArea;
use plurimus::widgets::ActiveDescendant;

use crate::chart::SeriesChart;
use crate::ledger::LedgerTable;
use crate::overview::OverviewChart;
use crate::session::{RowYear, YearCursor};

use super::click;

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
