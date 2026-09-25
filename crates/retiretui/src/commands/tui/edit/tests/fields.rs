//! Headless tests for the form one item is edited in: opening and
//! closing it, the tab ring, and its buttons.

use bevy_ecs::prelude::Entity;
use plurimus::term::KeyCode;

use super::{cursor, draft_plan, fixture_app, focused, is_editing, open, retype_balance};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::edit::build::FormButton;
use crate::commands::tui::edit::table::Row;
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{composed_frame, press_key, press_shift, show};

#[test]
fn enter_opens_the_row_and_esc_closes_it() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    let table = focused(&app);
    press_key(&mut app, KeyCode::Enter);
    assert!(is_editing(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("5%"), "the rate as a percent: {frame}");
    assert!(
        frame.contains("[ Brokerage ▾ ]"),
        "the kind is picked: {frame}"
    );
    assert!(
        frame.contains("[ Discard ]") && frame.contains("[ Apply ]"),
        "{frame}"
    );
    assert!(
        frame.contains("╭ Accounts ") && frame.contains("│  cash "),
        "the table stays visible around the overlay: {frame}"
    );
    press_key(&mut app, KeyCode::Esc);
    assert!(!is_editing(&app));
    assert_eq!(focused(&app), table, "focus returns to the table");
    let frame = composed_frame(&app);
    assert!(!frame.contains("[ Apply ]"), "the overlay is gone: {frame}");
}

fn focused_button(app: &bevy_app::App) -> Option<FormButton> {
    app.world().get::<FormButton>(focused(app)).copied()
}

pub(super) fn tab_to_button(app: &mut bevy_app::App, which: FormButton) {
    tab_until(app, &format!("{which:?}"), |app| {
        focused_button(app) == Some(which)
    });
}

/// Tabs on until the keyboard `is_there`, which is `named` where it never is.
pub(super) fn tab_until(
    app: &mut bevy_app::App,
    named: &str,
    is_there: impl Fn(&bevy_app::App) -> bool,
) {
    for _ in 0..TAB_LIMIT {
        if is_there(app) {
            return;
        }
        press_key(app, KeyCode::Tab);
    }
    panic!("{named} was never reached");
}

/// More stops than any form has, so a ring that never reaches its stop
/// fails rather than spinning.
const TAB_LIMIT: usize = 40;

/// Opens the second account and types a new balance into it, leaving the
/// keyboard on the field it typed into.
pub(super) fn open_and_retype_balance(app: &mut bevy_app::App) {
    show(app, Page::Accounts);
    app.update();
    press_key(app, KeyCode::Down);
    press_key(app, KeyCode::Enter);
    retype_balance(app);
}

#[test]
fn the_tab_ring_ends_on_discard_then_apply() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    press_shift(&mut app, KeyCode::Tab);
    assert_eq!(
        focused_button(&app),
        Some(FormButton::Apply),
        "the ring wraps back onto the last widget, which applies"
    );
    press_shift(&mut app, KeyCode::Tab);
    assert_eq!(focused_button(&app), Some(FormButton::Discard));
    press_shift(&mut app, KeyCode::Tab);
    assert_eq!(focused_button(&app), None, "the fields come before them");
}

#[test]
fn the_apply_button_stores_the_open_item() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    tab_to_button(&mut app, FormButton::Apply);
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app), "applying closes the overlay");
    assert_eq!(draft_plan(&app).accounts[1].balance, 123_456);
    assert_eq!(cursor(&mut app), Row(1), "the applied row keeps the cursor");
}

#[test]
fn the_discard_button_drops_the_open_item() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    tab_to_button(&mut app, FormButton::Discard);
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app), "discarding closes the overlay");
    assert_eq!(
        draft_plan(&app).accounts[1].balance,
        300_000,
        "the snapshot went nowhere"
    );
}

#[test]
fn shift_tab_steps_back_to_the_previous_field() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    let first = focused(&app);
    press_key(&mut app, KeyCode::Tab);
    assert_ne!(focused(&app), first, "tab moved forward");
    press_shift(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), first, "shift-tab comes back");
    press_key(&mut app, KeyCode::Esc);
}

#[test]
fn a_press_outside_the_overlay_discards_it() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    let form = app
        .world_mut()
        .query::<(Entity, &plurimus::ui::ModalOpen)>()
        .iter(app.world())
        .map(|(entity, _)| entity)
        .next()
        .expect("the open overlay is modal");
    let before = draft_plan(&app);
    app.world_mut()
        .trigger(plurimus::ui::ModalDismiss { entity: form });
    app.update();
    assert!(!is_editing(&app), "the guard's dismissal closes the item");
    assert_eq!(
        draft_plan(&app).accounts,
        before.accounts,
        "nothing applied"
    );
}

#[test]
fn applying_writes_the_whole_item_and_leaving_a_field_does_not() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(
        draft_plan(&app).accounts[1].balance,
        300_000,
        "leaving a field does not apply"
    );
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app));
    assert_eq!(draft_plan(&app).accounts[1].balance, 123_456);
    assert!(app.world().resource::<Draft>().is_dirty());
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("$123,456"), "the table follows: {frame}");
    assert_eq!(cursor(&mut app), Row(1), "the applied row keeps the cursor");
}

#[test]
fn an_open_item_is_named_in_its_own_title() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("Edit cash"), "the item it opened: {frame}");
    press_key(&mut app, KeyCode::Esc);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        !frame.contains("Edit cash"),
        "the closed pane names its domain again: {frame}"
    );
    press_key(&mut app, KeyCode::Char('a'));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("New Account"), "a new item: {frame}");
    press_key(&mut app, KeyCode::Esc);
    open(&mut app, Page::Transfers);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        !frame.contains("Edit pension-dc"),
        "a transfer is known by its id, not by the account its table leads \
         with: {frame}"
    );
    assert!(
        frame.contains("Edit dc-rollover"),
        "an unnamed item: {frame}"
    );
    press_key(&mut app, KeyCode::Esc);
}
