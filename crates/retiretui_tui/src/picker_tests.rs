//! Headless tests of the pickers the shell finds things through: the
//! command palette, help, and go-to.

use crate::support::typed;
use bevy_app::App;
use bevy_input::keyboard::Key;
use bevy_input_focus::InputFocus;
use plurimus::term::PasteMessage;

use super::nav::{self, Page};
use super::picker::Picking;
use super::support::{
    SIZE, active_page as active, commit_edit, composed_frame, headless_app, press_ctrl, press_key,
    press_shift, said, show, type_text,
};

pub(super) fn is_palette_open(app: &App) -> bool {
    app.world().resource::<Picking>().is_open()
}

#[test]
fn palette_swallows_bindings_while_typing_and_escapes() {
    let mut app = headless_app(SIZE);
    let focus_before = app.world().resource::<InputFocus>().get();
    press_key(&mut app, typed(':'));
    assert!(is_palette_open(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Commands"), "{frame}");
    assert!(
        frame.contains("leave the dashboard"),
        "rows say what they do: {frame}"
    );
    press_key(&mut app, typed('q'));
    assert!(app.should_exit().is_none(), "q typed, not run");
    assert_eq!(active(&app), Page::Overview);
    press_key(&mut app, Key::Escape);
    assert!(!is_palette_open(&app));
    assert_eq!(app.world().resource::<InputFocus>().get(), focus_before);
    assert!(!composed_frame(&app).contains("╭ Commands"));
}

#[test]
fn palette_runs_the_typed_command_and_reports_misses() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed(':'));
    type_text(&mut app, "ledg");
    press_key(&mut app, Key::Enter);
    assert!(!is_palette_open(&app));
    assert_eq!(active(&app), Page::Ledger);
    press_key(&mut app, typed(':'));
    type_text(&mut app, "zzz");
    press_key(&mut app, Key::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("no command matches \"zzz\""), "{frame}");
    assert!(is_palette_open(&app), "a miss chooses nothing");
    press_key(&mut app, Key::Escape);
    press_ctrl(&mut app, Key::ArrowUp);
    assert_eq!(active(&app), Page::Overview, "bindings live again");
}

#[test]
fn palette_arrows_pick_a_suggestion() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed(':'));
    type_text(&mut app, "tab-");
    press_key(&mut app, Key::ArrowDown);
    press_key(&mut app, Key::Enter);
    assert_eq!(
        active(&app).tab(),
        nav::Group::Plan.tab(),
        "tab-previous wraps to the last tab"
    );
}

#[test]
fn help_finds_a_command_by_what_it_does_and_names_its_key() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed('?'));
    type_text(&mut app, "dollars");
    let frame = composed_frame(&app);
    let row = frame.lines().find(|row| row.contains("nominal dollars"));
    assert!(row.is_some_and(|row| row.contains(" n │")), "{frame}");
    press_key(&mut app, Key::Enter);
    assert!(app.world().resource::<super::session::Basis>().nominal);
}

#[test]
fn go_to_shows_the_page_typed() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed('g'));
    type_text(&mut app, "resid");
    press_key(&mut app, Key::Enter);
    assert_eq!(active(&app), Page::Residency);
    assert!(app.world().resource::<InputFocus>().get().is_some());
}

#[test]
fn a_choice_that_asks_takes_the_keyboard_from_the_page_not_the_picker() {
    let mut app = headless_app(SIZE);
    commit_edit(&mut app, |plan| {
        plan.plan.name = Some("draft-plan".to_owned());
    });
    app.update();
    show(&mut app, Page::Accounts);
    let table = app.world().resource::<InputFocus>().get();
    press_key(&mut app, typed(':'));
    type_text(&mut app, "quit");
    press_key(&mut app, Key::Enter);
    app.update();
    assert!(composed_frame(&app).contains("╭ Confirm"));
    press_key(&mut app, Key::Escape);
    assert_eq!(app.world().resource::<InputFocus>().get(), table);
}

#[test]
fn a_command_run_from_the_palette_acts_on_what_held_the_keyboard() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, typed(':'));
    type_text(&mut app, "add");
    press_key(&mut app, Key::Enter);
    app.update();
    assert!(said(&app).is_empty(), "refused: {:?}", said(&app));
    assert!(
        app.world()
            .resource::<super::edit::EditSession>()
            .editing_table()
            .is_some(),
        "add opened a new account"
    );
}

#[test]
fn a_page_shown_from_the_palette_gets_the_keyboard() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, typed(':'));
    type_text(&mut app, "ledger");
    press_key(&mut app, Key::Enter);
    app.update();
    let held = app.world().resource::<InputFocus>().get().unwrap();
    assert!(
        app.world()
            .get::<super::ledger::LedgerTable>(held)
            .is_some()
    );
}

#[test]
fn a_paste_goes_into_the_query_without_its_control_characters() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed(':'));
    app.world_mut()
        .write_message(PasteMessage("qu\nit".to_owned()));
    app.update();
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("> quit"), "{frame}");
    assert!(!frame.contains("re-read the plan"), "rows follow: {frame}");
}

#[test]
fn the_query_selects_nothing_its_row_cannot_show() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed(':'));
    type_text(&mut app, "ta");
    press_shift(&mut app, Key::ArrowLeft);
    type_text(&mut app, "x");
    let frame = composed_frame(&app);
    assert!(frame.contains("> tax"), "nothing was replaced: {frame}");
}

#[test]
fn an_arrow_at_the_end_of_the_rows_stays_in_the_picker() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed(':'));
    let list = app.world().resource::<InputFocus>().get();
    press_key(&mut app, Key::ArrowUp);
    type_text(&mut app, "quit");
    press_key(&mut app, Key::ArrowDown);
    assert!(is_palette_open(&app));
    assert_eq!(app.world().resource::<InputFocus>().get(), list);
}

#[test]
fn an_arrow_the_query_has_no_use_for_stays_on_the_palette() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, typed(':'));
    type_text(&mut app, "ta");
    press_shift(&mut app, Key::ArrowLeft);
    type_text(&mut app, "x");
    let frame = composed_frame(&app);
    assert!(frame.contains("> tax"), "over a page of panes: {frame}");
}

/// The page go-to shows after `keys` move its cursor from the first row.
fn gone_to_after(keys: &[Key]) -> Page {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed('g'));
    for key in keys {
        press_key(&mut app, key.clone());
    }
    press_key(&mut app, Key::Enter);
    active(&app)
}

#[test]
fn a_page_key_moves_the_picker_s_cursor_a_list_s_height() {
    let paged = gone_to_after(&[Key::PageDown]);
    let stepped = gone_to_after(&vec![Key::ArrowDown; 10]);
    assert_eq!(paged, stepped);
    assert_ne!(paged, gone_to_after(&[]), "it moved");
    assert_eq!(
        gone_to_after(&[Key::PageDown, Key::PageUp]),
        gone_to_after(&[]),
        "and back up to the first"
    );
}

#[test]
fn a_space_typed_into_a_picker_is_part_of_the_query() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, typed('g'));
    type_text(&mut app, "tax t");
    assert!(is_palette_open(&app), "the space chose nothing");
    assert!(composed_frame(&app).contains("> tax t"));
    press_key(&mut app, Key::Enter);
    assert_eq!(active(&app), Page::TaxTables);
}
