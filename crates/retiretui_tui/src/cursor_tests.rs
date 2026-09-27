//! The year cursor from every writer to every reader: the Ledger's table,
//! the Overview's charts and actions, and Compare.

use bevy_app::App;
use bevy_ecs::change_detection::DetectChanges;
use plurimus::term::KeyCode;

use super::nav::Page;
use super::session::{Today, YearCursor};
use super::support::{
    ROOMY, SIZE, TODAY, assert_at_rest, cell_of, click, click_year, composed_frame, headless_app,
    headless_app_in, ledger_year, overview_chart, overview_year, press_key, redrawn, scratch_plan,
    show,
};

/// The test plan's last projected year: born 1980, to age 70.
const LAST_YEAR: i16 = 2050;

fn cursor(app: &App) -> YearCursor {
    *app.world().resource::<YearCursor>()
}

/// Shows the Ledger, asserting its table and its Flows agree on the year.
fn ledger_shows(app: &mut App) -> i16 {
    show(app, Page::Ledger);
    let year = ledger_year(app);
    let frame = composed_frame(app);
    assert!(frame.contains(&format!("{year} Flows")), "{year}: {frame}");
    year
}

#[test]
fn a_year_moved_in_the_ledger_reaches_the_overview() {
    let mut app = headless_app(SIZE);
    assert_eq!(ledger_shows(&mut app), TODAY.0);
    press_key(&mut app, KeyCode::Down);
    assert_eq!(overview_year(&mut app), TODAY.0 + 1);
    show(&mut app, Page::Ledger);
    press_key(&mut app, KeyCode::End);
    assert_eq!(overview_year(&mut app), LAST_YEAR);
    show(&mut app, Page::Ledger);
    let (column, row) = cell_of(&app, " 2040 ");
    click(&mut app, column, row);
    assert_eq!(overview_year(&mut app), 2040);
}

#[test]
fn a_chart_click_on_the_overview_moves_the_ledger_table_and_its_flows() {
    let mut app = headless_app(ROOMY);
    ledger_shows(&mut app);
    show(&mut app, Page::Overview);
    let chart = overview_chart(&mut app);
    click_year(&mut app, chart, 2040);
    assert_eq!(overview_year(&mut app), 2040);
    let clicked = app.world().resource_ref::<YearCursor>().last_changed();
    click_year(&mut app, chart, 2040);
    let again = app.world().resource_ref::<YearCursor>().last_changed();
    assert_eq!(
        clicked, again,
        "a press on the year already on writes nothing"
    );
    assert_eq!(ledger_shows(&mut app), 2040);
    press_key(&mut app, KeyCode::Down);
    assert_eq!(cursor(&app), YearCursor(Some(2041)), "↓ steps on from 2040");
    assert_eq!(ledger_shows(&mut app), 2041);
    assert_at_rest(&mut app);
}

#[test]
fn the_ledger_reveals_a_year_set_elsewhere() {
    let mut app = headless_app(SIZE);
    ledger_shows(&mut app);
    show(&mut app, Page::Overview);
    let chart = overview_chart(&mut app);
    click_year(&mut app, chart, LAST_YEAR);
    assert_eq!(ledger_shows(&mut app), LAST_YEAR);
    let frame = redrawn(&mut app);
    assert!(frame.contains(&format!("▌ {LAST_YEAR} ")), "{frame}");
}

/// Every view, opened in `today`, shows `year`, and no visit moves it.
fn every_view_agrees_in(today: Today, year: i16) {
    let mut app = headless_app_in(scratch_plan(), SIZE, today);
    assert_eq!(overview_year(&mut app), year);
    assert_eq!(ledger_shows(&mut app), year);
    assert_at_rest(&mut app);
    assert_eq!(cursor(&app), YearCursor(None), "the visit wrote no year");
    assert_eq!(overview_year(&mut app), year);
    show(&mut app, Page::Compare);
    let frame = composed_frame(&app);
    assert!(frame.contains(&format!("dollars · {year} ─")), "{frame}");
}

#[test]
fn today_before_the_plan_is_its_first_year_everywhere() {
    every_view_agrees_in(Today(2010), TODAY.0);
}

#[test]
fn today_past_the_horizon_is_its_last_year_everywhere() {
    every_view_agrees_in(Today(2080), LAST_YEAR);
}
