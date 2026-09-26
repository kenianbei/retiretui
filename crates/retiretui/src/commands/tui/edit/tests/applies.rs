//! Headless tests for the rows a form shows only while the item has a use
//! for them.

use plurimus::term::KeyCode;

use super::{
    clear_field, draft_plan, fixture_app, focused, is_editing, open, open_travel, shows_row,
    tab_to_field,
};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::edit::build::FormField;
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{
    composed_frame, press_key, press_shift, said, show, type_text,
};

/// Stops from an expense's id to whether it recurs.
const HAPPENS: usize = 3;

#[test]
fn what_happens_once_shows_when_and_what_recurs_shows_its_window() {
    let mut app = fixture_app();
    open_travel(&mut app);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Every year ▾ ]"), "{frame}");
    assert!(shows_row(&frame, "Starts") && shows_row(&frame, "Ends"));
    assert!(!shows_row(&frame, "On"), "{frame}");
    tab_to_field(&mut app, HAPPENS);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Once ▾ ]"), "{frame}");
    assert!(shows_row(&frame, "On"), "{frame}");
    assert!(!shows_row(&frame, "Starts") && !shows_row(&frame, "Ends"));
}

#[test]
fn a_hidden_value_is_kept_while_editing_and_dropped_on_apply() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, HAPPENS);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Left);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ retire ▾ ]"),
        "the start came back: {frame}"
    );
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "2032-01-01");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app), "{:?}", composed_frame(&app));
    let travel = draft_plan(&app).expenses[1].clone();
    assert!(travel.on.is_some(), "once, on a date");
    assert_eq!((travel.start, travel.end), (None, None), "and no window");
}

#[test]
fn a_row_that_comes_back_keeps_its_unused_operands_out_of_the_tab_order() {
    let mut app = fixture_app();
    open_travel(&mut app);
    tab_to_field(&mut app, HAPPENS);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Left);
    app.update();
    // Start is an event (kind, event, offset), and the age of `end` is the
    // stop after its kind: five from whether it recurs.
    tab_to_field(&mut app, 5);
    clear_field(&mut app);
    type_text(&mut app, "85");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app));
    let end = draft_plan(&app).expenses[1].end.clone().expect("an end");
    assert_eq!(end.age, Some(85));
}

#[test]
fn a_windfall_is_not_asked_whether_it_recurs() {
    let mut app = fixture_app();
    show(&mut app, Page::Income);
    app.update();
    for _ in 0..3 {
        press_key(&mut app, KeyCode::Down);
    }
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Windfall ▾ ]"), "{frame}");
    assert!(shows_row(&frame, "On"), "{frame}");
    assert!(!shows_row(&frame, "Happens") && !shows_row(&frame, "Starts"));
}

#[test]
fn an_account_shows_what_its_kind_can_hold() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Cash ▾ ]"), "{frame}");
    for hidden in ["Roth", "Basis"] {
        assert!(!shows_row(&frame, hidden), "{hidden}: {frame}");
    }
    tab_to_field(&mut app, 2);
    press_key(&mut app, KeyCode::Left);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Brokerage ▾ ]"), "{frame}");
    assert!(shows_row(&frame, "Basis"), "{frame}");
}

#[test]
fn a_value_the_file_holds_stays_on_show_where_it_has_no_use() {
    let mut app = fixture_app();
    app.world_mut().resource_mut::<Draft>().plan.accounts[0].basis = Some(1_000);
    open(&mut app, Page::Accounts);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Cash ▾ ]"), "{frame}");
    assert!(shows_row(&frame, "Basis"), "to be cleared by hand: {frame}");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(draft_plan(&app).accounts[0].basis, Some(1_000), "untouched");
}

#[test]
fn a_value_with_no_use_cleared_by_hand_leaves_with_the_keyboard() {
    let mut app = fixture_app();
    app.world_mut().resource_mut::<Draft>().plan.accounts[0].basis = Some(1_000);
    open(&mut app, Page::Accounts);
    super::fields::tab_until(&mut app, "basis", |app| {
        let field = app.world().get::<FormField>(focused(app));
        field.is_some_and(|field| field.spec.key == "basis")
    });
    clear_field(&mut app);
    assert!(
        shows_row(&composed_frame(&app), "Basis"),
        "held while typed in"
    );
    press_shift(&mut app, KeyCode::Tab);
    assert!(!shows_row(&composed_frame(&app), "Basis"), "gone once left");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(draft_plan(&app).accounts[0].basis, None);
}

#[test]
fn a_state_is_asked_of_the_united_states_alone() {
    let mut app = fixture_app();
    open(&mut app, Page::Residency);
    assert!(shows_row(&composed_frame(&app), "State"));
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(!shows_row(&frame, "State"), "{frame}");
}

#[test]
fn a_hidden_row_that_makes_no_value_refuses_nothing() {
    let mut app = fixture_app();
    open_travel(&mut app);
    // The start's kind steps to an income, and names none.
    tab_to_field(&mut app, HAPPENS + 1);
    press_key(&mut app, KeyCode::Right);
    press_shift(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "2032-01-01");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app), "{:?}", said(&app));
    assert!(draft_plan(&app).expenses[1].on.is_some());
}

#[test]
fn the_rows_a_tick_reveals_are_ruled_under_it() {
    let mut app = fixture_app();
    open(&mut app, Page::Household);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Char(' '));
    app.update();
    let frame = composed_frame(&app);
    // The form's row, not the details' behind it: the one with a widget.
    let column_of = |label: &str| {
        let row = frame
            .lines()
            .find(|line| line.contains(label) && line.contains('['));
        let row = row.unwrap_or_else(|| panic!("{label}: {frame}"));
        row.find('[').map(|at| row[..at].chars().count())
    };
    assert!(frame.contains(" │ Include Part D"), "{frame}");
    assert!(!frame.contains(" │ Medicare surcharges"), "the tick itself");
    assert_eq!(
        column_of("Include Part D"),
        column_of("Filing status"),
        "the values still line up: {frame}"
    );
}

#[test]
fn once_with_no_date_is_refused_rather_than_left_as_every_year() {
    let mut app = fixture_app();
    open(&mut app, Page::Expenses);
    tab_to_field(&mut app, HAPPENS);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    assert!(is_editing(&app), "nothing says when");
    let heard = said(&app).join("\n");
    assert!(heard.contains("On: happens once"), "{heard}");
}
