use bevy_app::App;
use plurimus::term::KeyCode;

use super::*;
use crate::commands::tui::session::Today;
use crate::commands::tui::support::{
    SIZE, TEST_PLAN, TODAY, assert_at_rest, headless_app_in, ledger_year, overview_year, redrawn,
};

/// The By-year view, ↓ from the plan's first year.
fn by_year_down(app: &mut App) {
    press_key(app, KeyCode::Char('v'));
    press_key(app, KeyCode::Char('v'));
    let table = single::<ByYearTable>(app);
    while focused(app) != Some(table) {
        press_key(app, KeyCode::Tab);
    }
    press_key(app, KeyCode::Down);
    redrawn(app);
}

#[test]
fn the_by_year_cursor_reaches_the_ledger_and_the_overview() {
    let mut app = comparing_variant(SIZE);
    show(&mut app, Page::Ledger);
    show(&mut app, Page::Compare);
    by_year_down(&mut app);
    assert_eq!(cursor_year(&app), Some(2027));
    assert_at_rest(&mut app);
    assert_eq!(overview_year(&mut app), 2027);
    show(&mut app, Page::Ledger);
    assert_eq!(ledger_year(&mut app), 2027);
    assert_at_rest(&mut app);
}

/// The Compare page in `today`, comparing a plan that runs past the
/// document's horizon.
fn comparing_longer(today: Today) -> crate::commands::tui::support::Headless {
    let longer = TEST_PLAN.replace("horizon_age = 70", "horizon_age = 75");
    let dir = scratch_workspace(TEST_PLAN);
    std::fs::write(dir.join("longer.toml"), longer).unwrap();
    let mut app = headless_app_in(dir.join("plan.toml"), SIZE, today);
    show(&mut app, Page::Compare);
    compare_with(&mut app, "longer");
    redrawn(&mut app);
    app
}

#[test]
fn compare_reaches_years_the_document_does_not() {
    let mut app = comparing_longer(TODAY);
    by_year_down(&mut app);
    press_key(&mut app, KeyCode::End);
    let frame = redrawn(&mut app);
    assert_eq!(cursor_year(&app), Some(2055));
    assert!(frame.contains("dollars · 2055 ─"), "{frame}");
    assert_at_rest(&mut app);
    assert_eq!(overview_year(&mut app), 2050, "as near as the plan reaches");
    show(&mut app, Page::Ledger);
    assert_eq!(ledger_year(&mut app), 2050);
    assert_eq!(
        cursor_year(&app),
        Some(2055),
        "the Ledger wrote nothing back"
    );
}

#[test]
fn today_past_the_document_is_its_last_year_in_compare_too() {
    let mut app = comparing_longer(Today(2053));
    let frame = redrawn(&mut app);
    assert!(frame.contains("dollars · 2050 ─"), "{frame}");
    assert_eq!(overview_year(&mut app), 2050);
}
