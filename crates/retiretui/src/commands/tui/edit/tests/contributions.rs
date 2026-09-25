//! Headless tests for the Contributions domain: the pick no file holds
//! that says which way an amount is stated, and what applying writes.

use plurimus::term::KeyCode;

use super::{draft_plan, fixture_app, open, shows_row, tab_to_field};
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{composed_frame, press_key, type_text};

/// Stops from a contribution's id to the pick that says how much.
const AMOUNT_AS: usize = 4;

fn rows_shown(frame: &str, labels: &[&str]) -> Vec<bool> {
    labels.iter().map(|label| shows_row(frame, label)).collect()
}

const ROWS: [&str; 6] = [
    "Amount",
    "Rate",
    "Of",
    "Steps up by",
    "Match rate",
    "Growth",
];

#[test]
fn the_amount_pick_shows_the_rows_of_its_form() {
    let mut app = fixture_app();
    open(&mut app, Page::Contributions);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Dollars ▾ ]"), "{frame}");
    assert_eq!(
        rows_shown(&frame, &ROWS),
        [true, false, false, false, false, true]
    );
    tab_to_field(&mut app, AMOUNT_AS);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Share of income ▾ ]"), "{frame}");
    assert_eq!(
        rows_shown(&frame, &ROWS),
        [false, true, true, true, false, false]
    );
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ The maximum ▾ ]"), "{frame}");
    assert_eq!(rows_shown(&frame, &ROWS), [false; 6]);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Employer match ▾ ]"), "{frame}");
    assert_eq!(
        rows_shown(&frame, &ROWS),
        [false, false, true, false, true, false]
    );
}

#[test]
fn the_maximum_is_written_from_the_pick_and_read_back_into_it() {
    let mut app = fixture_app();
    open(&mut app, Page::Contributions);
    tab_to_field(&mut app, AMOUNT_AS);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    let first = &draft_plan(&app).contributions[0];
    assert!(first.max && first.amount.is_none(), "{first:?}");
    let frame = composed_frame(&app);
    assert!(frame.contains("the maximum"), "the table says it: {frame}");
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ The maximum ▾ ]"), "{frame}");
}

#[test]
fn a_share_names_its_income_and_switched_back_leaves_no_rate_behind() {
    let mut app = fixture_app();
    open(&mut app, Page::Contributions);
    tab_to_field(&mut app, AMOUNT_AS);
    press_key(&mut app, KeyCode::Right);
    // The rate is a slider and its text; the income pick comes after both.
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Right);
    tab_to_field(&mut app, 2);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    let first = &draft_plan(&app).contributions[0];
    assert_eq!(first.of.as_deref(), Some("salary"), "{first:?}");
    assert!(first.rate.is_some() && first.amount.is_none(), "{first:?}");
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, AMOUNT_AS);
    press_key(&mut app, KeyCode::Left);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "5000");
    press_key(&mut app, KeyCode::Enter);
    let first = &draft_plan(&app).contributions[0];
    assert_eq!(first.amount, Some(5_000), "{first:?}");
    assert!(first.rate.is_none() && first.of.is_none(), "{first:?}");
}
