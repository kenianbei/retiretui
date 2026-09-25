//! Headless tests for the keyboard's walk between a table, its details
//! and the sidebar, and for a form, which is entered on purpose and only
//! esc or an apply comes out of.

use plurimus::term::KeyCode;

use super::{fixture_app_sized, is_editing, is_on, retype_balance};
use crate::commands::tui::edit::details::DetailsTable;
use crate::commands::tui::edit::table::DomainTable;
use crate::commands::tui::nav::Page;
use crate::commands::tui::sidebar::Sidebar;
use crate::commands::tui::support::{
    self, SIZE, headless_app_at, is_asking, press_ctrl, press_key, press_shift, run_command, said,
    show,
};

/// More presses than any form has stops.
const ROUND_THE_FORM: usize = 40;

#[test]
fn tab_walks_the_table_its_details_and_the_sidebar_round() {
    let mut app = fixture_app_sized(SIZE);
    show(&mut app, Page::Accounts);
    assert!(is_on::<DomainTable>(&app));
    press_key(&mut app, KeyCode::Tab);
    assert!(
        is_on::<DetailsTable>(&app),
        "the details come after the table"
    );
    press_key(&mut app, KeyCode::Tab);
    assert!(is_on::<Sidebar>(&app), "and wrap to the sidebar");
    press_shift(&mut app, KeyCode::Tab);
    assert!(is_on::<DetailsTable>(&app));
    press_shift(&mut app, KeyCode::Tab);
    assert!(is_on::<DomainTable>(&app));
    assert!(!is_editing(&app), "walking opens nothing");
}

#[test]
fn the_command_does_what_its_key_does() {
    let mut app = fixture_app_sized(SIZE);
    show(&mut app, Page::Accounts);
    run_command(&mut app, "focus-next");
    assert!(is_on::<DetailsTable>(&app));
    run_command(&mut app, "focus-previous");
    assert!(is_on::<DomainTable>(&app));
}

#[test]
fn tab_in_a_form_holding_edits_asks_nothing_and_stays_in_it() {
    let mut app = fixture_app_sized(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Enter);
    retype_balance(&mut app);
    for _ in 0..ROUND_THE_FORM {
        press_key(&mut app, KeyCode::Tab);
        assert!(!is_asking(&app));
        assert!(!is_on::<DomainTable>(&app) && !is_on::<Sidebar>(&app));
    }
}

#[test]
fn an_open_menu_keeps_the_chords_that_would_turn_the_page() {
    let mut app = fixture_app_sized(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Char(' '));
    press_ctrl(&mut app, KeyCode::Down);
    assert_eq!(support::active_page(&app), Page::Accounts);
}

#[test]
fn tab_in_a_read_only_session_reaches_the_details_and_says_nothing() {
    let scenario = support::scratch_scenario();
    let mut app = headless_app_at(scenario, SIZE);
    show(&mut app, Page::Accounts);
    app.update();
    let heard = said(&app);
    press_key(&mut app, KeyCode::Tab);
    assert!(is_on::<DetailsTable>(&app));
    assert_eq!(said(&app), heard);
}

#[test]
fn a_single_items_page_is_its_details_and_enter_opens_its_form() {
    let mut app = fixture_app_sized(SIZE);
    show(&mut app, Page::Household);
    assert!(is_on::<DetailsTable>(&app), "the details have the keyboard");
    press_key(&mut app, KeyCode::Tab);
    assert!(is_on::<Sidebar>(&app), "the page has the one pane");
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Enter);
    assert!(is_editing(&app), "in the form");
    press_key(&mut app, KeyCode::Esc);
    assert!(is_on::<DetailsTable>(&app), "esc comes back to the details");
}
