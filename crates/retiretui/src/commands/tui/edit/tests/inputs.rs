//! Headless tests for what a form's fields are entered with: text,
//! picks, flags and sliders.

use plurimus::term::KeyCode;
use plurimus::widgets::SliderValue;
use retiretui_engine::plan::AccountKind;

use super::{
    BALANCE_FIELD, clear_field, draft_plan, fixture_app, focused, is_editing, open, tab_to_field,
};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::nav::Page;
use crate::commands::tui::present;
use crate::commands::tui::support::{
    cell_fg, cell_of, composed_frame, press_key, redrawn, show, type_text,
};
use crate::commands::tui::theme::Theme;

/// Stops along a brokerage account's form to its rate's slider: a cash or
/// brokerage account has no Roth to stop at.
const RATE_FIELD: usize = 7;

#[test]
fn an_input_is_bracketed_and_its_brackets_follow_the_keyboard() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    let theme = app.world().resource::<Theme>();
    let (accent, dim) = (theme.accent, theme.dim);
    let bracket_fg = |app: &bevy_app::App, drawn: &str| {
        let (column, row) = cell_of(app, drawn);
        cell_fg(app, column, row)
    };
    assert_eq!(bracket_fg(&app, "[ cash"), Some(accent), "held");
    assert_eq!(bracket_fg(&app, "[ $40,000"), Some(dim), "at rest");
    press_key(&mut app, KeyCode::Tab);
    app.update();
    assert_eq!(bracket_fg(&app, "[ cash"), Some(dim), "and let go");
}

#[test]
fn closed_sets_are_picked_and_flags_are_ticked() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    tab_to_field(&mut app, 2);
    press_key(&mut app, KeyCode::Right);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ 401(k) ▾ ]"),
        "past the last kind is the first: {frame}"
    );
    press_key(&mut app, KeyCode::Tab);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ ]"), "a flag is a box: {frame}");
    press_key(&mut app, KeyCode::Char(' '));
    let frame = composed_frame(&app);
    assert!(frame.contains("[x]"), "space ticks it: {frame}");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app), "enter applies rather than toggling");
    let account = &draft_plan(&app).accounts[0];
    assert_eq!(account.kind, AccountKind::K401k);
    assert!(account.roth);
}

#[test]
fn a_reference_offers_the_plan_s_own_ids() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    // A cash account has no Roth to stop at between its type and owner.
    tab_to_field(&mut app, 3);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ jordan ▾ ]"), "{frame}");
    press_key(&mut app, KeyCode::Right);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ alex ▾ ]"),
        "the household's other person: {frame}"
    );
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(draft_plan(&app).accounts[0].owner, "alex");
}

#[test]
fn a_rate_moves_on_its_slider() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, RATE_FIELD);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Right);
    let frame = redrawn(&mut app);
    assert!(frame.contains("5.5%"), "the text beside it: {frame}");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app));
    let returns = draft_plan(&app).accounts[1]
        .expected_return
        .expect("the fixture states a rate");
    assert!(
        (returns - 0.055).abs() < 1e-9,
        "each step from the last, two above 0.05: {returns}"
    );
}

#[test]
fn a_typed_rate_moves_its_slider() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, RATE_FIELD);
    let slider = focused(&app);
    press_key(&mut app, KeyCode::Tab);
    clear_field(&mut app);
    type_text(&mut app, "7%");
    app.update();
    let shown = app.world().get::<SliderValue>(slider).map(|value| value.0);
    assert!(
        shown.is_some_and(|shown| (shown - 0.07).abs() < 1e-6),
        "{shown:?}"
    );
}

#[test]
fn a_rejected_value_keeps_the_item_open() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    tab_to_field(&mut app, BALANCE_FIELD);
    clear_field(&mut app);
    type_text(&mut app, "lots");
    press_key(&mut app, KeyCode::Enter);
    assert!(is_editing(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("balance:"), "{frame}");
    assert_eq!(draft_plan(&app).accounts[0].balance, 40_000);
    assert!(!app.world().resource::<Draft>().is_dirty());
}

#[test]
fn a_kind_s_menu_lists_every_kind_the_schema_declares() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    tab_to_field(&mut app, 2);
    press_key(&mut app, KeyCode::Char(' '));
    app.update();
    let frame = composed_frame(&app);
    for kind in AccountKind::ALL {
        assert!(
            frame.contains(&format!(" {} ", present::account_kind(*kind))),
            "{kind:?}: {frame}"
        );
    }
}

#[test]
fn a_menu_opens_on_the_value_held_and_enter_picks_the_row_reached() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, 2);
    let select = focused(&app);
    press_key(&mut app, KeyCode::Char(' '));
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(focused(&app), select, "the keyboard is back on the select");
    assert!(is_editing(&app), "picking applies nothing");
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ Cash ▾ ]"),
        "the row after brokerage: {frame}"
    );
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(draft_plan(&app).accounts[1].kind, AccountKind::Cash);
}

#[test]
fn esc_closes_an_open_menu_and_not_the_item_under_it() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    tab_to_field(&mut app, 2);
    let select = focused(&app);
    press_key(&mut app, KeyCode::Char(' '));
    assert_ne!(focused(&app), select, "the menu has the keyboard");
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(focused(&app), select);
    assert!(is_editing(&app));
}

#[test]
fn a_field_reads_plainly_while_typed_in_and_dressed_once_left() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    let form_row = |app: &bevy_app::App, label: &str| {
        let frame = composed_frame(app);
        // The form's own cells: the table beside it shares the line.
        let row = frame.lines().find_map(|line| line.split_once(label));
        let (_, held) = row.unwrap_or_else(|| panic!("{label}: {frame}"));
        held.split('│').next().unwrap_or_default().to_owned()
    };
    assert!(form_row(&app, "│Balance").contains("$300,000"));
    tab_to_field(&mut app, BALANCE_FIELD);
    app.update();
    let typed_in = form_row(&app, "│Balance");
    assert!(
        typed_in.contains("300000") && !typed_in.contains('$'),
        "{typed_in}"
    );
    press_key(&mut app, KeyCode::Tab);
    app.update();
    assert!(
        form_row(&app, "│Balance").contains("$300,000"),
        "dressed again"
    );
    // From the cost basis: how it is invested, the rate's slider, then its
    // text.
    tab_to_field(&mut app, 3);
    clear_field(&mut app);
    type_text(&mut app, "7");
    press_key(&mut app, KeyCode::Tab);
    app.update();
    assert!(
        form_row(&app, "│Expected return").contains("7%"),
        "a bare rate is a percent"
    );
}
