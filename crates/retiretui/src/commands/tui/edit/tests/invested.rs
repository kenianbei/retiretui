//! Headless tests for how an account is invested: the pick no file holds,
//! and what a mix applied writes.

use plurimus::term::KeyCode;
use retiretui_engine::plan::Allocation;

use super::{
    INVESTED, draft_plan, fixture_app, fixture_app_sized, shows_row, tab_to_field, tab_to_key,
};
use crate::commands::tui::nav::Page;
use crate::commands::tui::support::{
    Headless, ROOMY, commit_edit, composed_frame, press_key, show, type_text,
};

#[test]
fn one_mix_shows_its_shares_in_place_of_a_return_and_applies_as_an_allocation() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Fixed return ▾ ]"), "{frame}");
    assert!(
        shows_row(&frame, "Expected return") && !shows_row(&frame, "Stocks"),
        "{frame}"
    );
    tab_to_field(&mut app, INVESTED);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ One mix ▾ ]"), "{frame}");
    assert!(
        !shows_row(&frame, "Expected return") && shows_row(&frame, "Stocks"),
        "{frame}"
    );
    // Each share is a slider, then its text.
    tab_to_field(&mut app, 2);
    type_text(&mut app, "70");
    tab_to_field(&mut app, 2);
    type_text(&mut app, "20");
    press_key(&mut app, KeyCode::Enter);
    let account = &draft_plan(&app).accounts[1];
    assert_eq!(account.expected_return, None, "{account:?}");
    let Some(Allocation::Mix(mix)) = account.allocation else {
        panic!("one mix: {account:?}");
    };
    assert!((mix.stocks - 0.7).abs() < 1e-9 && (mix.bonds - 0.2).abs() < 1e-9);
    assert!(
        (mix.cash - 0.1).abs() < 1e-9,
        "cash is what is left: {mix:?}"
    );
}

#[test]
fn a_glide_path_offers_its_first_step() {
    let mut app = fixture_app();
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, INVESTED);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Right);
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("[ Glide path ▾ ]"), "{frame}");
    assert!(shows_row(&frame, "Mix 1 from") && !shows_row(&frame, "Mix 2 from"));
    assert!(!shows_row(&frame, "Expected return"));
}

/// The brokerage account's glide path: three steps, the first leaving a
/// tenth in cash.
const GLIDE: &str = r#"steps = [
    { from = { age = 50, owner = "jordan" }, stocks = 0.7, bonds = 0.2, cash = 0.1 },
    { from = { age = 60, owner = "jordan" }, stocks = 0.6, bonds = 0.4 },
    { from = { age = 70, owner = "jordan" }, bonds = 1.0 },
]"#;

fn open_glide_path() -> Headless {
    let mut app = fixture_app_sized(ROOMY);
    commit_edit(&mut app, |plan| {
        let steps: toml::Table = GLIDE.parse().unwrap();
        let account = &mut plan.accounts[1];
        account.expected_return = None;
        account.allocation = Some(steps["steps"].clone().try_into().unwrap());
    });
    show(&mut app, Page::Accounts);
    app.update();
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    app
}

#[test]
fn a_glide_path_offers_every_filled_step_and_one_blank_one() {
    let app = open_glide_path();
    let frame = composed_frame(&app);
    assert!(
        shows_row(&frame, "Mix 3 from") && shows_row(&frame, "Mix 4 from"),
        "{frame}"
    );
    assert!(!shows_row(&frame, "Mix 5 from"), "{frame}");
}

#[test]
fn a_mix_shows_in_cash_what_its_stocks_and_bonds_leave_as_they_move() {
    let mut app = open_glide_path();
    let cash_of = |frame: &str, left: &str| {
        let row = |line: &&str| line.contains("  Cash") && line.contains(left);
        frame.lines().any(|line| row(&line))
    };
    assert!(cash_of(&composed_frame(&app), "10%"));
    tab_to_key(&mut app, "allocation.0.stocks");
    press_key(&mut app, KeyCode::Left);
    app.update();
    let frame = composed_frame(&app);
    assert!(cash_of(&frame, "15%"), "{frame}");
}
