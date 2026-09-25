//! What is said to the user: journaled, toasted, and listed in the drawer.

use std::time::Duration;

use bevy_app::App;
use bevy_input_focus::InputFocus;
use plurimus::core::ratatui_core::style::Color;
use plurimus::term::KeyCode;
use retiretui_engine::plan::Plan;

use bevy_ecs::system::RunSystemOnce;

use super::edit::DraftEditor;
use super::journal;
use super::layout;
use super::nav::Page;
use super::support::{
    SIZE, click, composed_buffer, composed_frame, headless_app, let_pass, press_key, said, show,
};
use super::toast::Toasts;

fn toasts(app: &App) -> Vec<String> {
    let shown = app.world().resource::<Toasts>().texts();
    shown.into_iter().map(str::to_owned).collect()
}

#[test]
fn a_refusal_is_journaled_and_toasted_over_the_page() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char('a'));
    assert_eq!(said(&app), ["nothing to add or delete here"]);
    assert_eq!(toasts(&app), said(&app));
    let frame = composed_frame(&app);
    let row = frame.lines().find(|row| row.contains("nothing to add"));
    assert!(
        row.is_some_and(|row| row.trim_end().ends_with('│')),
        "boxed at the right edge: {frame}"
    );
    assert!(frame.lines().last().unwrap().contains("? help"), "{frame}");
}

#[test]
fn a_toast_outlives_a_glance_and_not_a_long_look() {
    let mut app = headless_app(SIZE);
    journal::say("plan reloaded");
    journal::warn("something refused");
    app.update();
    app.update();
    assert_eq!(toasts(&app).len(), 2);
    let_pass(&mut app, Duration::from_secs(5));
    assert_eq!(
        toasts(&app),
        ["something refused"],
        "a warning stays longer"
    );
    let_pass(&mut app, Duration::from_secs(5));
    assert!(toasts(&app).is_empty());
    assert_eq!(
        said(&app).len(),
        2,
        "the journal keeps what the toasts let go"
    );
}

#[test]
fn a_fourth_toast_pushes_the_oldest_out() {
    let mut app = headless_app(SIZE);
    for text in ["one", "two", "three", "four"] {
        journal::say(text);
    }
    app.update();
    app.update();
    assert_eq!(toasts(&app), ["two", "three", "four"]);
}

#[test]
fn a_press_takes_a_toast_down() {
    let mut app = headless_app(SIZE);
    journal::say("plan reloaded");
    app.update();
    app.update();
    let frame = composed_frame(&app);
    let (row, text) = frame
        .lines()
        .enumerate()
        .find(|(_, text)| text.contains("plan reloaded"))
        .unwrap();
    let column = text.chars().position(|symbol| symbol == 'p').unwrap();
    click(&mut app, column as u16, row as u16);
    assert!(toasts(&app).is_empty());
}

#[test]
fn the_drawer_lists_what_was_said_and_gives_the_keyboard_back() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    let table = app.world().resource::<InputFocus>().get();
    press_key(&mut app, KeyCode::Char('m'));
    assert!(composed_frame(&app).contains("nothing has been said yet"));
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(app.world().resource::<InputFocus>().get(), table);

    let held = app
        .world()
        .resource::<super::edit::Draft>()
        .plan
        .accounts
        .len();
    for _ in 0..held {
        press_key(&mut app, KeyCode::Char('d'));
        press_key(&mut app, KeyCode::Enter);
    }
    press_key(&mut app, KeyCode::Char('d'));
    let_pass(&mut app, Duration::from_secs(10));
    press_key(&mut app, KeyCode::Char('m'));
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Messages"), "{frame}");
    assert!(frame.contains("nothing highlighted to delete"), "{frame}");
    assert!(
        frame.lines().last().unwrap().contains("esc close"),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Char('m'));
    assert!(
        composed_frame(&app).contains("╭ Messages"),
        "the drawer owns m"
    );
    press_key(&mut app, KeyCode::Esc);
    assert!(!composed_frame(&app).contains("╭ Messages"));
}

/// The colour the badge beside the file name is drawn in.
fn badge_colour(app: &App) -> Option<Color> {
    let buffer = composed_buffer(app);
    (0..buffer.area.width)
        .filter_map(|column| buffer.cell((column, layout::TAB_ROW_ROWS / 2)))
        .find(|cell| cell.symbol() == "●")
        .and_then(|cell| cell.style().fg)
}

#[test]
fn the_badge_is_lit_while_the_draft_holds_unsaved_changes() {
    let mut app = headless_app(SIZE);
    app.update();
    assert_eq!(badge_colour(&app), Some(Color::DarkGray));
    app.world_mut()
        .run_system_once(|mut editor: DraftEditor| {
            let plan: &mut Plan = &mut editor.draft.plan;
            plan.plan.name = Some("draft".to_owned());
            editor.commit();
        })
        .unwrap();
    app.update();
    app.update();
    assert_eq!(badge_colour(&app), Some(Color::Cyan));
}
