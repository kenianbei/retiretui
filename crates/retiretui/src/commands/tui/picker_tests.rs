//! Headless tests of the pickers the shell finds things through: the
//! command palette, help, and go-to.

use bevy_app::App;
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;

use super::nav::{self, Page};
use super::picker::Picking;
use super::support::{
    SIZE, active_page as active, commit_edit, composed_frame, headless_app, press_ctrl, press_key,
    said, show, type_text,
};

pub(super) fn is_palette_open(app: &App) -> bool {
    app.world().resource::<Picking>().is_open()
}

#[test]
fn palette_swallows_bindings_while_typing_and_escapes() {
    let mut app = headless_app(SIZE);
    let focus_before = app.world().resource::<InputFocus>().get();
    press_key(&mut app, KeyCode::Char(':'));
    assert!(is_palette_open(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Commands"), "{frame}");
    assert!(
        frame.contains("leave the dashboard"),
        "rows say what they do: {frame}"
    );
    press_key(&mut app, KeyCode::Char('q'));
    assert!(app.should_exit().is_none(), "q typed, not run");
    assert_eq!(active(&app), Page::Overview);
    press_key(&mut app, KeyCode::Esc);
    assert!(!is_palette_open(&app));
    assert_eq!(app.world().resource::<InputFocus>().get(), focus_before);
    assert!(!composed_frame(&app).contains("╭ Commands"));
}

#[test]
fn palette_runs_the_typed_command_and_reports_misses() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "ledg");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_palette_open(&app));
    assert_eq!(active(&app), Page::Ledger);
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "zzz");
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("no command matches \"zzz\""), "{frame}");
    assert!(is_palette_open(&app), "a miss chooses nothing");
    press_key(&mut app, KeyCode::Esc);
    press_ctrl(&mut app, KeyCode::Up);
    assert_eq!(active(&app), Page::Overview, "bindings live again");
}

#[test]
fn palette_arrows_pick_a_suggestion() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "tab-");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        active(&app).tab(),
        nav::Group::Plan.tab(),
        "tab-previous wraps to the last tab"
    );
}

#[test]
fn help_finds_a_command_by_what_it_does_and_names_its_key() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char('?'));
    type_text(&mut app, "dollars");
    let frame = composed_frame(&app);
    let row = frame.lines().find(|row| row.contains("nominal dollars"));
    assert!(row.is_some_and(|row| row.contains(" n │")), "{frame}");
    press_key(&mut app, KeyCode::Enter);
    assert!(app.world().resource::<super::session::Basis>().nominal);
}

#[test]
fn go_to_shows_the_page_typed() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char('g'));
    type_text(&mut app, "resid");
    press_key(&mut app, KeyCode::Enter);
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
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "quit");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert!(composed_frame(&app).contains("╭ Confirm"));
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(app.world().resource::<InputFocus>().get(), table);
}

#[test]
fn a_command_run_from_the_palette_acts_on_what_held_the_keyboard() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "add");
    press_key(&mut app, KeyCode::Enter);
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
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "ledger");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let held = app.world().resource::<InputFocus>().get().unwrap();
    assert!(
        app.world()
            .get::<super::ledger::LedgerTable>(held)
            .is_some()
    );
}
