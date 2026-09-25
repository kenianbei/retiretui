//! Headless tests for taking back applied items and putting them back.

use plurimus::term::KeyCode;

use super::fields::open_and_retype_balance;
use super::{draft_plan, fixture_app};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{
    SIZE, commit_edit as commit, composed_frame, headless_app, press_ctrl, press_key, said, show,
    type_text,
};

const ORIGINAL: i64 = 300_000;
const RETYPED: i64 = 123_456;

fn balance(app: &bevy_app::App, index: usize) -> i64 {
    draft_plan(app).accounts[index].balance
}

fn is_dirty(app: &bevy_app::App) -> bool {
    app.world().resource::<Draft>().is_dirty()
}

fn last_said(app: &bevy_app::App) -> String {
    said(app).last().cloned().unwrap_or_default()
}

#[test]
fn undo_takes_back_one_applied_item_and_redo_puts_it_back() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(balance(&app, 1), RETYPED);
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(balance(&app, 1), ORIGINAL);
    assert!(is_dirty(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("$300,000"), "the table follows: {frame}");
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(last_said(&app), "nothing to undo");
    press_ctrl(&mut app, KeyCode::Char('y'));
    assert_eq!(balance(&app, 1), RETYPED);
    press_ctrl(&mut app, KeyCode::Char('y'));
    assert_eq!(last_said(&app), "nothing to redo");
}

#[test]
fn deleting_and_adding_are_steps() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Char('d'));
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(draft_plan(&app).accounts.len(), 1);
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(draft_plan(&app).accounts.len(), 2);
    press_key(&mut app, KeyCode::Char('a'));
    type_text(&mut app, "brokerage");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(draft_plan(&app).accounts.len(), 3);
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(draft_plan(&app).accounts.len(), 2);
}

#[test]
fn an_edit_after_undo_forgets_what_was_undone() {
    let mut app = headless_app(SIZE);
    commit(&mut app, |plan| plan.plan.name = Some("first".to_owned()));
    press_ctrl(&mut app, KeyCode::Char('z'));
    commit(&mut app, |plan| plan.plan.name = Some("second".to_owned()));
    press_ctrl(&mut app, KeyCode::Char('y'));
    assert_eq!(last_said(&app), "nothing to redo");
    assert_eq!(draft_plan(&app).plan.name.as_deref(), Some("second"));
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(
        draft_plan(&app).plan.name.as_deref(),
        Some("test-plan"),
        "the second edit stepped from where undo left the draft"
    );
}

#[test]
fn history_reaches_back_a_hundred_items() {
    let mut app = headless_app(SIZE);
    for step in 1..=101 {
        commit(&mut app, move |plan| {
            plan.plan.name = Some(step.to_string());
        });
    }
    for _ in 0..100 {
        press_ctrl(&mut app, KeyCode::Char('z'));
    }
    assert_eq!(draft_plan(&app).plan.name.as_deref(), Some("1"));
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(last_said(&app), "nothing to undo");
}

#[test]
fn undo_past_a_save_dirties_the_draft_again() {
    let mut app = headless_app(SIZE);
    commit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    press_ctrl(&mut app, KeyCode::Char('s'));
    assert!(!is_dirty(&app), "{:?}", said(&app));
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert!(is_dirty(&app));
    assert_eq!(draft_plan(&app).plan.name.as_deref(), Some("test-plan"));
}

#[test]
fn a_reload_empties_the_history() {
    let mut app = headless_app(SIZE);
    commit(&mut app, |plan| plan.plan.name = Some("first".to_owned()));
    press_key(&mut app, KeyCode::Char('r'));
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(last_said(&app), "nothing to undo");
    assert_eq!(draft_plan(&app).plan.name.as_deref(), Some("test-plan"));
}
