//! Headless tests for a form's height and scrolling: it stands as tall as
//! its rows on show, and what the body cannot hold scrolls.

use plurimus::term::KeyCode;

use super::{INVESTED, fixture_app, form_box, open, shows_row, tab_to_field, tab_to_key};
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{composed_frame, press_key, show};

/// The rows the open form's box takes, its frame included.
fn form_rows(frame: &str) -> usize {
    let (top, left) = form_box(frame);
    let mut below = frame.lines().skip(top);
    let bottom = below.position(|line| line.chars().nth(left) == Some('╰'));
    bottom.unwrap_or_else(|| panic!("no foot: {frame}")) + 1
}

/// The bar along the open form's right edge, from its top arrow down.
fn bar(frame: &str) -> String {
    let lines: Vec<&str> = frame.lines().collect();
    let arrow = lines
        .iter()
        .enumerate()
        .find_map(|(top, line)| Some((top, line.chars().position(|cell| cell == '▲')?)));
    let (top, column) = arrow.unwrap_or_else(|| panic!("no bar: {frame}"));
    let cells = lines[top..]
        .iter()
        .filter_map(|line| line.chars().nth(column));
    cells.take_while(|&cell| cell != '▼').collect()
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
    assert_eq!(
        mixed,
        fixed + 2,
        "a return gives way to stocks, bonds and cash"
    );
}

/// What the Market form holds first and last, which the body cannot
/// hold together at the least terminal the shell takes.
const FIRST: &str = "Leave at least";
const LAST: &str = "  wrap";

#[test]
fn a_form_taller_than_the_body_scrolls_to_the_field_holding_the_keyboard() {
    let mut app = fixture_app();
    open(&mut app, Page::Market);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        shows_row(&frame, FIRST) && !shows_row(&frame, LAST),
        "{frame}"
    );
    let at_top = bar(&frame);
    tab_to_key(&mut app, "wraps");
    app.update();
    let frame = composed_frame(&app);
    assert!(
        shows_row(&frame, LAST) && !shows_row(&frame, FIRST),
        "{frame}"
    );
    assert_ne!(bar(&frame), at_top, "the bar follows: {frame}");
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
    open(&mut app, Page::Accounts);
    app.update();
    let frame = composed_frame(&app);
    assert!(!frame.contains('▲'), "{frame}");
}
