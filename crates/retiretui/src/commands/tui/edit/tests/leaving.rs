//! Headless tests for leaving an item whose edits no one applied.

use crate::commands::tui::layout;
use bevy_ecs::system::RunSystemOnce;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::term::KeyCode;

use super::fields::{open_and_retype_balance, tab_to_button};
use super::{
    clear_field, draft_plan, fixture_app, fixture_app_sized, focused, is_editing, open,
    tab_to_field,
};
use crate::commands::tui::edit::build::FormButton;
use crate::commands::tui::edit::{Draft, DraftEditor};
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::session::Session;
use crate::commands::tui::support::{
    self, SIZE, answer_back, cell_of, cell_style, click, composed_frame, headless_app, is_asking,
    press_ctrl, press_key, press_shift, said, show, type_text,
};
use crate::commands::tui::theme::Theme;

/// A cell inside the first tab's box, which is how a page is chosen with
/// the pointer.
const OVERVIEW_TAB: (u16, u16) = (2, layout::TAB_ROW_ROWS / 2);

#[derive(Clone, Copy)]
pub(super) enum Answers {
    Cancel,
    Discard,
    Apply,
}

/// The dialog opens on Apply, the last of its three answers.
pub(super) fn answer(app: &mut bevy_app::App, which: Answers) {
    let back = match which {
        Answers::Apply => 0,
        Answers::Discard => 1,
        Answers::Cancel => 2,
    };
    answer_back(app, back);
}

fn active_page(app: &bevy_app::App) -> Page {
    app.world().resource::<ActivePage>().0
}

#[test]
fn buttons_say_which_answer_is_meant_and_which_loses_the_edit() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    let over = app.world().resource::<Theme>().over;
    let style_of = |app: &bevy_app::App, button: &str| {
        let (column, row) = cell_of(app, button);
        cell_style(app, column, row)
    };
    let is_bold = |style: Style| style.add_modifier.contains(Modifier::BOLD);
    assert!(is_bold(style_of(&app, "[ Apply ]")), "the form's own");
    assert!(!is_bold(style_of(&app, "[ Discard ]")));

    press_key(&mut app, KeyCode::Esc);
    app.update();
    let discard = style_of(&app, "[ Discard ]");
    assert_eq!(discard.fg, Some(over), "the dialog's loses the edit");
    assert!(!is_bold(discard));
    press_shift(&mut app, KeyCode::Tab);
    app.update();
    let held = style_of(&app, "[ Discard ]");
    assert_eq!(held.fg, Some(over), "and says so under the keyboard");
    assert!(is_bold(held), "which it shows by its weight");
}

#[test]
fn esc_on_an_edited_item_asks_what_to_do_with_the_edit() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    tab_to_button(&mut app, FormButton::Apply);
    press_key(&mut app, KeyCode::Esc);
    assert!(
        is_editing(&app) && is_asking(&app),
        "nothing is dropped yet"
    );
    let frame = composed_frame(&app);
    assert!(frame.contains("Edit brokerage has changes"), "{frame}");
    answer(&mut app, Answers::Discard);
    assert!(!is_editing(&app) && !is_asking(&app));
    assert_eq!(draft_plan(&app).accounts[1].balance, 300_000);
}

#[test]
fn answering_apply_stores_the_edit_and_cancel_goes_back_to_it() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    let field = focused(&app);
    press_key(&mut app, KeyCode::Esc);
    answer(&mut app, Answers::Cancel);
    assert!(is_editing(&app) && !is_asking(&app));
    assert_eq!(focused(&app), field, "the keyboard is back where it was");
    assert!(
        composed_frame(&app).contains("123456"),
        "and so is the edit"
    );
    press_key(&mut app, KeyCode::Esc);
    answer(&mut app, Answers::Apply);
    assert!(!is_editing(&app));
    assert_eq!(draft_plan(&app).accounts[1].balance, 123_456);
}

#[test]
fn a_press_outside_an_edited_item_asks_too() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    click(&mut app, 1, 2);
    assert!(is_editing(&app) && is_asking(&app));
}

#[test]
fn an_open_item_keeps_the_chord_that_would_quit() {
    let mut app = headless_app(SIZE);
    open(&mut app, Page::Settings);
    tab_to_field(&mut app, 2);
    clear_field(&mut app);
    type_text(&mut app, "80");
    press_ctrl(&mut app, KeyCode::Char('c'));
    assert!(app.should_exit().is_none(), "the form has the keys");
    assert!(!is_asking(&app));
}

#[test]
fn a_press_on_the_tab_bar_over_an_edit_asks_and_turns_no_page() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Settings);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, 2);
    clear_field(&mut app);
    type_text(&mut app, "80");
    click(&mut app, OVERVIEW_TAB.0, OVERVIEW_TAB.1);
    assert!(is_asking(&app));
    assert_eq!(active_page(&app), Page::Settings);
    answer(&mut app, Answers::Apply);
    assert_eq!(draft_plan(&app).plan.horizon_age, 80);
    assert_eq!(
        active_page(&app),
        Page::Settings,
        "the press was the question's"
    );
}

#[test]
fn applying_an_item_no_one_edited_leaves_the_draft_clean() {
    let mut app = fixture_app();
    open(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_editing(&app));
    assert!(!app.world().resource::<Draft>().is_dirty());
}

#[test]
fn a_disk_change_waits_behind_an_item_being_edited() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Settings);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, 2);
    clear_field(&mut app);
    type_text(&mut app, "80");
    let plan_path = app.world().resource::<Session>().plan_path.clone().unwrap();
    let changed = support::TEST_PLAN.replace("horizon_age = 70", "horizon_age = 75");
    support::change_on_disk(&mut app, &plan_path, &changed);
    assert!(is_editing(&app), "the item stays open");
    assert!(composed_frame(&app).contains("80"), "with its edit");
    assert!(
        said(&app)
            .iter()
            .any(|text| text.contains("plan changed on disk"))
    );
}

#[test]
fn reloading_over_an_edit_asks_about_the_edit_first() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Settings);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, 2);
    clear_field(&mut app);
    type_text(&mut app, "80");
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Char('r'));
    assert!(is_asking(&app));
    answer(&mut app, Answers::Cancel);
    assert!(
        composed_frame(&app).contains("80"),
        "the edit is still held"
    );
}

#[test]
fn an_item_that_moved_under_its_edit_is_not_written_over_another() {
    let mut app = fixture_app();
    open_and_retype_balance(&mut app);
    app.world_mut()
        .run_system_once(|mut editor: DraftEditor| {
            editor.draft.plan.accounts.remove(0);
            editor.commit();
        })
        .unwrap();
    app.update();
    press_key(&mut app, KeyCode::Enter);
    assert!(is_editing(&app), "refused rather than applied");
    let accounts = draft_plan(&app).accounts;
    assert_eq!(accounts[0].id, "brokerage");
    assert_eq!(
        accounts[1].balance, 450_000,
        "the item now second is untouched"
    );
    assert_eq!(accounts[0].balance, 300_000);
}

#[test]
fn tab_walks_a_form_round_wherever_it_is_drawn() {
    for (page, size, drawn) in [
        (Page::Household, SIZE, "alone"),
        (Page::Accounts, SIZE, "over its table"),
    ] {
        let mut app = fixture_app_sized(size);
        show(&mut app, page);
        press_key(&mut app, KeyCode::Enter);
        let first = focused(&app);
        tab_to_button(&mut app, FormButton::Apply);
        press_key(&mut app, KeyCode::Tab);
        assert_eq!(focused(&app), first, "{drawn}: apply is the last stop");
    }
}
