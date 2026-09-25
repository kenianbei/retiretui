//! Headless tests for which keys an open item keeps and which reach the
//! shell.

use plurimus::term::KeyCode;

use super::{clear_field, draft_plan, fixture_app, is_editing, open, tab_to_field};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::nav::Page;
use crate::commands::tui::session::Basis;
use crate::commands::tui::support::{
    SIZE, composed_frame, headless_app, press_ctrl, press_key, said, show, type_text,
};

/// Whether the shell heard `n`, its plain key for the dollar basis.
fn is_nominal(app: &bevy_app::App) -> bool {
    app.world().resource::<Basis>().nominal
}

#[test]
fn an_open_item_owns_the_keyboard() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    tab_to_field(&mut app, 2);
    press_key(&mut app, KeyCode::Char('n'));
    assert!(is_editing(&app), "a plain key is typed, not run");
    assert!(!is_nominal(&app));
    press_ctrl(&mut app, KeyCode::Char('s'));
    assert!(is_editing(&app), "nor does a chord reach the shell");
    assert!(
        !said(&app).iter().any(|text| text.contains("saved")),
        "the save never ran"
    );
}

#[test]
fn a_field_being_edited_keeps_the_keys_the_shell_would_take() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Household);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        frame.contains("Filing status"),
        "the settings form: {frame}"
    );
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Char('n'));
    assert!(
        !is_nominal(&app),
        "a plain key on a pick inside the form does not reach the shell"
    );
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Char('n'));
    assert!(
        is_nominal(&app),
        "esc leaves the form, so the shell hears keys again"
    );
}

#[test]
fn a_chord_reaches_the_shell_from_a_field_being_typed_into() {
    let mut app = headless_app(SIZE);
    open(&mut app, Page::Settings);
    tab_to_field(&mut app, 2);
    clear_field(&mut app);
    type_text(&mut app, "80");
    press_key(&mut app, KeyCode::Enter);
    assert!(app.world().resource::<Draft>().is_dirty());
    press_ctrl(&mut app, KeyCode::Char('s'));
    assert!(
        !app.world().resource::<Draft>().is_dirty(),
        "ctrl-s saves from inside the field it was typed in"
    );
}

#[test]
fn single_forms_commit_each_field_and_refuse_add() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Settings);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("Plan to age"), "{frame}");
    press_key(&mut app, KeyCode::Char('a'));
    let frame = composed_frame(&app);
    assert!(frame.contains("nothing to add"), "{frame}");
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, 2);
    clear_field(&mut app);
    type_text(&mut app, "80");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(draft_plan(&app).plan.horizon_age, 80);
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Char('n'));
    assert!(
        is_nominal(&app),
        "esc leaves the form, so the shell hears keys again"
    );
}
