//! Headless tests for a form's height and scrolling: it stands as tall as
//! its rows on show, and what the body cannot hold scrolls.

use plurimus::term::KeyCode;

use super::{fixture_app, open, shows_row, tab_to_field, tab_to_key};
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{composed_frame, press_key, show};

/// Stops along a brokerage account's form to the pick that says how it is
/// invested.
const INVESTED: usize = 6;

/// The rows the open form's box takes, its frame included.
fn form_rows(frame: &str) -> usize {
    let lines: Vec<Vec<char>> = frame.lines().map(|line| line.chars().collect()).collect();
    let top = lines.iter().position(|line| {
        let line: String = line.iter().collect();
        line.contains("╭ Edit")
    });
    let top = top.unwrap_or_else(|| panic!("no form: {frame}"));
    let left = lines[top].iter().position(|&cell| cell == '╭').unwrap();
    let bottom = lines[top..]
        .iter()
        .position(|line| line.get(left) == Some(&'╰'));
    bottom.unwrap_or_else(|| panic!("no foot: {frame}")) + 1
}

#[test]
fn a_form_stands_as_tall_as_its_rows_on_show() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    let fixed = form_rows(&composed_frame(&app));
    // ID, name, type, owner, balance, basis, invested, return, locked,
    // drain; help and foot; the frame.
    assert_eq!(fixed, 10 + 6 + 2);
    tab_to_field(&mut app, INVESTED);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let mixed = form_rows(&composed_frame(&app));
    assert_eq!(mixed, fixed + 1, "a return gives way to stocks and bonds");
}

/// What the Market form holds first and last, which the body cannot
/// hold together at the least terminal the shell takes.
const FIRST: &str = "Leave at least";
const LAST: &str = "  wrap";

#[test]
fn a_form_taller_than_the_body_scrolls_to_the_field_holding_the_keyboard() {
    let mut app = fixture_app();
    open(&mut app, Page::Market);
    let frame = composed_frame(&app);
    assert!(
        shows_row(&frame, FIRST) && !shows_row(&frame, LAST),
        "{frame}"
    );
    tab_to_key(&mut app, "wraps");
    app.update();
    let frame = composed_frame(&app);
    assert!(
        shows_row(&frame, LAST) && !shows_row(&frame, FIRST),
        "{frame}"
    );
    assert!(frame.contains("[ Apply ]"), "the foot stays: {frame}");
    tab_to_key(&mut app, "leave_at_least");
    app.update();
    let frame = composed_frame(&app);
    assert!(shows_row(&frame, FIRST), "back up: {frame}");
}

#[test]
fn a_bar_is_drawn_only_while_the_fields_overflow() {
    let mut app = fixture_app();
    open(&mut app, Page::Market);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains('▲') && frame.contains('▼'), "{frame}");
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let frame = composed_frame(&app);
    assert!(!frame.contains('▲'), "{frame}");
}
