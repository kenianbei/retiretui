//! Headless tests for the composite trigger editor.

use plurimus::term::KeyCode;

use super::{clear_field, draft_plan, fixture_app, is_editing, open, open_travel, tab_to_field};

use crate::commands::tui::nav::Page;

/// Stops from an expense's id to the kind of its start: the name, the amount and
/// whether it recurs come between.
const START_KIND: usize = 4;
use crate::commands::tui::support::{composed_frame, press_key, said, type_text};

#[test]
fn a_trigger_reads_as_a_sentence_of_its_kind_and_operands() {
    let mut app = fixture_app();
    open_travel(&mut app);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ Event ▾ ]  [ retire ▾ ]   offset [ 1    ] years"),
        "what it follows, and how far off: {frame}"
    );
    assert!(
        frame.contains("[ Age ▾ ]    [ 80   ] of [ jordan ▾ ]"),
        "an age names whose it is: {frame}"
    );
    assert!(!frame.contains("yyyy-mm-dd"), "no date, no word of one");
}

#[test]
fn each_kind_keeps_the_operands_entered_for_it() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, START_KIND);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ Income ▾ ]"),
        "event steps to income: {frame}"
    );
    assert!(
        !frame.contains("[ retire ▾ ]"),
        "the operands of the old kind are hidden: {frame}"
    );
    press_key(&mut app, KeyCode::Left);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ Event ▾ ]  [ retire ▾ ]"),
        "and come back with it: {frame}"
    );
}

#[test]
fn the_arrows_on_a_kind_never_reach_no_trigger() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, START_KIND);
    for _ in 0..5 {
        press_key(&mut app, KeyCode::Right);
        app.update();
        let frame = composed_frame(&app);
        assert!(!frame.contains("[ Plan start ▾ ]"), "{frame}");
    }
}

#[test]
fn the_foot_says_what_the_operand_holding_the_keyboard_means() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, START_KIND);
    app.update();
    assert!(composed_frame(&app).contains("When it begins."));
    press_key(&mut app, KeyCode::Tab);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("The event it follows."), "{frame}");
}

#[test]
fn the_operands_compose_into_the_schema_s_own_trigger() {
    let mut app = fixture_app();
    open_travel(&mut app);
    // Start is an event (kind, event, offset), so the age of `end` is four
    // stops past its kind.
    tab_to_field(&mut app, START_KIND + 4);
    clear_field(&mut app);
    type_text(&mut app, "85");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app));
    let end = draft_plan(&app).expenses[1].end.clone().expect("an end");
    assert_eq!(end.age, Some(85));
    assert_eq!(end.owner.as_deref(), Some("jordan"));
    assert_eq!(end.event, None, "the kind's own operands only");
    let start = draft_plan(&app).expenses[1].start.clone().expect("a start");
    assert_eq!(start.event.as_deref(), Some("retire"), "untouched");
}

#[test]
fn clearing_a_trigger_s_kind_clears_the_trigger() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, START_KIND);
    // The menu opens on Event, the third of four kinds; the row that
    // empties the field is after the last.
    press_key(&mut app, KeyCode::Char(' '));
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app));
    assert_eq!(draft_plan(&app).expenses[1].start, None);
}

#[test]
fn stepping_to_a_kind_with_more_operands_shows_them() {
    let mut app = fixture_app();
    open(&mut app, Page::Expenses);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("Starts          [ Plan start ▾"),
        "an absent trigger offers only its kind: {frame}"
    );
    // No kind has no operands; a date has one, an age has two.
    tab_to_field(&mut app, START_KIND);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Date ▾ ]"), "{frame}");
    assert!(frame.contains("] yyyy-mm-dd"), "{frame}");
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Age ▾ ]"), "{frame}");
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "70");
    app.update();
    let frame = composed_frame(&app);
    let row = frame
        .lines()
        .find(|line| line.contains("│Starts "))
        .expect("the row of the start");
    assert!(
        row.contains("[ Age ▾ ]") && row.contains("70"),
        "the operand a wider kind needs is shown, not left hidden: {row}"
    );
}

#[test]
fn a_form_an_odd_number_of_cells_narrower_than_the_body_keeps_its_border() {
    let mut app = fixture_app();
    open_travel(&mut app);
    let frame = composed_frame(&app);
    let (_, from_the_form) = frame
        .lines()
        .find_map(|line| line.split_once("│Starts  "))
        .expect("the row of a trigger whose last operand is typed");
    let borders = from_the_form.matches('│').count();
    assert_eq!(borders, 2, "the form's, then the pane's: {frame}");
}

#[test]
fn a_kind_chosen_without_its_operand_is_refused_rather_than_dropped() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, START_KIND);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    assert!(is_editing(&app), "an income trigger naming no income");
    let heard = said(&app).join("\n");
    assert!(heard.contains("Starts: the trigger names"), "{heard}");
    assert!(draft_plan(&app).expenses[1].start.is_some(), "and kept");
}

#[test]
fn a_reference_s_menu_lists_what_the_kind_now_refers_to() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, START_KIND);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Char(' '));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("│ salary"), "an income's menu: {frame}");
    assert!(!frame.contains("│ retire"), "not an event's: {frame}");
}
