//! The year cursor from every writer to every reader: the Ledger's years
//! and the year under them, a press on the Overview's chart, and Compare.

use bevy_app::App;
use plurimus::term::KeyCode;

use super::nav::Page;
use super::session::{Today, YearCursor};
use super::support::{
    ROOMY, TODAY, active_page, assert_at_rest, cell_of, click, click_year, composed_frame,
    headless_app, headless_app_in, ledger_year, overview_chart, press_key, redrawn, scratch_plan,
    show,
};

/// The test plan's last projected year: born 1980, to age 70.
const LAST_YEAR: i16 = 2050;

fn cursor(app: &App) -> YearCursor {
    *app.world().resource::<YearCursor>()
}

/// Shows the Ledger, asserting its table of years and the year in full
/// under it agree on the year.
fn ledger_shows(app: &mut App) -> i16 {
    show(app, Page::Ledger);
    let year = ledger_year(app);
    let frame = redrawn(app);
    let to_do = format!("╭ To do in {year} ");
    assert!(frame.contains(&to_do), "{year}: {frame}");
    year
}

#[test]
fn a_year_moved_in_the_ledger_moves_the_year_under_it() {
    let mut app = headless_app(ROOMY);
    assert_eq!(ledger_shows(&mut app), TODAY.0);
    press_key(&mut app, KeyCode::Down);
    assert_eq!(ledger_shows(&mut app), TODAY.0 + 1);
    press_key(&mut app, KeyCode::End);
    assert_eq!(ledger_shows(&mut app), LAST_YEAR);
    let (column, row) = cell_of(&app, " 2040 ");
    click(&mut app, column, row);
    assert_eq!(ledger_shows(&mut app), 2040);
}

#[test]
fn a_chart_click_on_the_overview_opens_the_ledger_at_that_year() {
    let mut app = headless_app(ROOMY);
    ledger_shows(&mut app);
    show(&mut app, Page::Overview);
    let chart = overview_chart(&mut app);
    click_year(&mut app, chart, 2040);
    app.update();
    assert_eq!(active_page(&app), Page::Ledger, "the press turned to it");
    assert_eq!(ledger_shows(&mut app), 2040);
    press_key(&mut app, KeyCode::Down);
    assert_eq!(cursor(&app), YearCursor(Some(2041)), "↓ steps on from 2040");
    assert_eq!(ledger_shows(&mut app), 2041);
    assert_at_rest(&mut app);
}

#[test]
fn the_ledger_reveals_a_year_set_elsewhere() {
    let mut app = headless_app(ROOMY);
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
    let mut app = headless_app_in(scratch_plan(), ROOMY, today);
    assert_eq!(ledger_shows(&mut app), year);
    assert_at_rest(&mut app);
    assert_eq!(cursor(&app), YearCursor(None), "the visit wrote no year");
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
