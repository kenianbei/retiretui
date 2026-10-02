//! Headless tests of the keys the shell shares with its widgets: copy
//! beside quit, and an arrow at the end of a list.

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_input_focus::InputFocus;
use plurimus::term::{KeyCode, LastCopied};

use super::nav::Page;
use super::support::{SIZE, headless_app, ledger_year, press_ctrl, press_key, show};

fn copied(app: &App) -> String {
    let last = app.world().resource::<LastCopied>();
    last.0.clone().unwrap_or_default()
}

fn holder(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

#[test]
fn ctrl_c_copies_the_row_under_the_cursor_and_ctrl_q_quits() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_ctrl(&mut app, KeyCode::Char('c'));
    assert!(copied(&app).starts_with("cash\t"), "{:?}", copied(&app));
    show(&mut app, Page::Ledger);
    press_ctrl(&mut app, KeyCode::Char('c'));
    let year = ledger_year(&mut app).to_string();
    assert!(copied(&app).starts_with(&year), "{:?}", copied(&app));
    assert!(app.should_exit().is_none(), "a copy is not a quit");
    press_ctrl(&mut app, KeyCode::Char('q'));
    assert!(app.should_exit().is_some());
}

#[test]
fn an_arrow_at_the_end_of_a_list_keeps_the_keyboard() {
    let mut app = headless_app(SIZE);
    for page in [Page::Accounts, Page::Ledger, Page::TaxTables] {
        show(&mut app, page);
        let pane = holder(&app);
        for (end, past) in [(KeyCode::Home, KeyCode::Up), (KeyCode::End, KeyCode::Down)] {
            press_key(&mut app, end);
            press_key(&mut app, past);
            assert_eq!(holder(&app), pane, "{page:?} {past:?}");
        }
    }
    press_key(&mut app, KeyCode::Esc);
    let sidebar = holder(&app);
    for (end, past) in [(KeyCode::Home, KeyCode::Up), (KeyCode::End, KeyCode::Down)] {
        press_key(&mut app, end);
        press_key(&mut app, past);
        assert_eq!(holder(&app), sidebar, "the sidebar {past:?}");
    }
}
