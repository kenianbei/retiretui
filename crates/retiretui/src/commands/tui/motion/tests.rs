use std::time::Duration;

use plurimus::core::ratatui_core::style::Color;
use plurimus::term::KeyCode;

use super::*;
use crate::commands::tui::layout;
use crate::commands::tui::nav::Page;
use crate::commands::tui::sidebar::SIDEBAR_COLS;
use crate::commands::tui::support::{
    Headless, SIZE, cell_fg, composed_frame, headless_app, headless_app_set, let_pass, press_key,
    said, scratch_plan, show, type_text,
};

const DIM: Play = Play::Dim(Color::Gray);

/// Where the accounts table's first item is drawn, outside any dialog:
/// the end of its balance, which no box centred on the body reaches.
/// A cell of the accounts table's first row, in from the sidebar.
const FIRST_ROW: (u16, u16) = (SIDEBAR_COLS + 6, layout::BODY_TOP + 2);
const WELL_PAST_ANY_EFFECT: Duration = Duration::from_secs(1);

fn moving_app(motion: Motion) -> Headless {
    let mut settings = Settings::default();
    settings.motion = motion;
    let mut app = headless_app_set(scratch_plan(), SIZE, settings);
    show(&mut app, Page::Accounts);
    app
}

#[test]
fn full_motion_gives_every_effect_its_own_length() {
    assert_eq!(Motion::Full.length(DIM), Duration::from_millis(120));
    assert_eq!(
        Motion::Full.length(Play::Coalesce),
        Duration::from_millis(150)
    );
    assert_eq!(
        Motion::Full.length(Play::Receipt(Color::Gray)),
        Duration::from_millis(400)
    );
}

#[test]
fn reduced_motion_keeps_what_informs_and_cuts_what_transitions() {
    assert_eq!(Motion::Reduced.length(DIM), Motion::Full.length(DIM));
    assert!(!Motion::Reduced.length(Play::Receipt(Color::Gray)).is_zero());
    assert!(Motion::Reduced.length(Play::Coalesce).is_zero());
}

#[test]
fn no_motion_gives_every_effect_no_time_at_all() {
    for play in [DIM, Play::Receipt(Color::Gray), Play::Coalesce] {
        assert!(Motion::Off.length(play).is_zero(), "{play:?}");
    }
}

#[test]
fn the_backdrop_dims_under_a_dialog_and_comes_back_when_it_closes() {
    let mut app = moving_app(Motion::Full);
    let resting = cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1);
    assert_ne!(resting, Some(Color::DarkGray));
    press_key(&mut app, KeyCode::Down);
    let resting_unfocused = cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1);

    press_key(&mut app, KeyCode::Char('d'));
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    let frame = composed_frame(&app);
    assert_eq!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::DarkGray),
        "the page behind is dimmed: {frame}"
    );
    let (row, text) = frame
        .lines()
        .enumerate()
        .find(|(_, text)| text.contains("Delete"))
        .unwrap();
    let column = text.chars().position(|symbol| symbol == 'D').unwrap();
    assert_ne!(
        cell_fg(&app, column as u16, row as u16),
        Some(Color::DarkGray),
        "the dialog is not"
    );
    assert_eq!(
        cell_fg(&app, 1, SIZE.rows - 1),
        Some(Color::Cyan),
        "nor the hint row, which names its keys"
    );

    press_key(&mut app, KeyCode::Esc);
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    assert_eq!(cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1), resting_unfocused);
}

#[test]
fn with_motion_off_nothing_dims() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('d'));
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    assert!(composed_frame(&app).contains("Delete"));
    assert_ne!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::DarkGray)
    );
}

#[test]
fn an_applied_row_fades_in_from_dim() {
    let mut app = moving_app(Motion::Full);
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "x");
    press_key(&mut app, KeyCode::Enter);
    assert!(said(&app).is_empty(), "applied: {:?}", said(&app));
    assert_eq!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::DarkGray),
        "the row starts from dim: {}",
        composed_frame(&app)
    );
    let_pass(&mut app, WELL_PAST_ANY_EFFECT);
    assert_eq!(
        cell_fg(&app, FIRST_ROW.0, FIRST_ROW.1),
        Some(Color::Cyan),
        "and settles into the cursor row's own accent"
    );
}

#[test]
fn the_motion_command_steps_through_the_three_and_says_which() {
    let mut app = headless_app(SIZE);
    assert_eq!(app.world().resource::<Settings>().motion, Motion::Off);
    for wanted in [Motion::Full, Motion::Reduced, Motion::Off] {
        press_key(&mut app, KeyCode::Char(':'));
        type_text(&mut app, "motion");
        press_key(&mut app, KeyCode::Enter);
        assert_eq!(app.world().resource::<Settings>().motion, wanted);
    }
    assert_eq!(said(&app), ["motion full", "motion reduced", "motion off"]);
}
