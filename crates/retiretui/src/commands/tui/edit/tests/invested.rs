//! Headless tests for how an account is invested: the pick no file holds,
//! and what a mix applied writes.

use plurimus::term::KeyCode;
use retiretui_engine::plan::Allocation;

use super::{draft_plan, fixture_app, shows_row, tab_to_field};
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{composed_frame, press_key, show, type_text};

/// Stops along a brokerage account's form to the pick that says how it is
/// invested.
const INVESTED: usize = 6;

#[test]
fn one_mix_shows_its_shares_in_place_of_a_return_and_applies_as_an_allocation() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Fixed return ▾ ]"), "{frame}");
    assert!(
        shows_row(&frame, "Expected return") && !shows_row(&frame, "Stocks"),
        "{frame}"
    );
    tab_to_field(&mut app, INVESTED);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ One mix ▾ ]"), "{frame}");
    assert!(
        !shows_row(&frame, "Expected return") && shows_row(&frame, "Stocks"),
        "{frame}"
    );
    // Each share is a slider, then its text.
    tab_to_field(&mut app, 2);
    type_text(&mut app, "70");
    tab_to_field(&mut app, 2);
    type_text(&mut app, "20");
    press_key(&mut app, KeyCode::Enter);
    let account = &draft_plan(&app).accounts[1];
    assert_eq!(account.expected_return, None, "{account:?}");
    let Some(Allocation::Mix(mix)) = account.allocation else {
        panic!("one mix: {account:?}");
    };
    assert!((mix.stocks - 0.7).abs() < 1e-9 && (mix.bonds - 0.2).abs() < 1e-9);
    assert!(
        (mix.cash - 0.1).abs() < 1e-9,
        "cash is what is left: {mix:?}"
    );
}

#[test]
fn a_glide_path_shows_its_two_steps() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, INVESTED);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Glide path ▾ ]"), "{frame}");
    assert!(shows_row(&frame, "First mix from") && shows_row(&frame, "Then from"));
    assert!(!shows_row(&frame, "Expected return"));
}
