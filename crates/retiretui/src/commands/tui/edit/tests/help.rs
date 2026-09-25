//! Headless tests for what a form says under its fields: the focused
//! field's help, or the issue the draft holds against it.

use plurimus::term::KeyCode;

use super::{BALANCE_FIELD, fixture_app, open, tab_to_field};
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{commit_edit, composed_frame, press_key};

#[test]
fn a_form_says_what_the_field_holding_the_keyboard_means() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    let frame = composed_frame(&app);
    assert!(frame.contains("A short unique handle"), "the id: {frame}");
    tab_to_field(&mut app, BALANCE_FIELD);
    let frame = composed_frame(&app);
    assert!(frame.contains("What it holds at the start"), "{frame}");
    assert!(!frame.contains("A short unique handle"), "{frame}");
}

#[test]
fn a_field_the_draft_holds_an_issue_against_says_so_in_its_form() {
    let mut app = fixture_app();
    commit_edit(&mut app, |plan| plan.accounts[1].balance = -1);
    open(&mut app, Page::Accounts);
    let frame = composed_frame(&app);
    assert!(!frame.contains("Balance !"), "cash is sound: {frame}");
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("Balance !"), "brokerage is marked: {frame}");
    assert!(
        frame.contains("A short unique handle"),
        "the id is sound: {frame}"
    );
    tab_to_field(&mut app, BALANCE_FIELD);
    let frame = composed_frame(&app);
    assert!(frame.contains("must not be negative"), "{frame}");
    assert!(!frame.contains("What it holds at the start"), "{frame}");
}
