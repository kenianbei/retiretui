//! Headless tests for a residency's places, picked through the search a
//! set too long for a menu opens.

use plurimus::term::KeyCode;

use super::fields::tab_to_button;
use super::{draft_plan, fixture_app_sized, open};
use crate::commands::tui::edit::build::FormButton;
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{SIZE, composed_frame, press_key, type_text};

#[test]
fn a_place_is_searched_for_and_applied() {
    let mut app = fixture_app_sized(SIZE);
    open(&mut app, Page::Residency);
    press_key(&mut app, KeyCode::Char(' '));
    type_text(&mut app, "portu");
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("Portugal"), "{frame}");
    assert!(!frame.contains("Poland"), "what matches: {frame}");
    press_key(&mut app, KeyCode::Enter);

    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Char(' '));
    type_text(&mut app, "none");
    press_key(&mut app, KeyCode::Enter);

    tab_to_button(&mut app, FormButton::Apply);
    press_key(&mut app, KeyCode::Enter);
    let residency = draft_plan(&app).residency.remove(0);
    assert_eq!(residency.country, "pt");
    assert_eq!(residency.state, None, "a field that may be empty");
    assert!(draft_plan(&app).validate().is_empty());
}
