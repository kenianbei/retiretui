use bevy_app::App;
use bevy_ecs::prelude::Entity;
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;
use plurimus::widgets::ActiveDescendant;

use super::verdict::TileAt;
use crate::commands::tui::edit::Row;
use crate::commands::tui::edit::tests::cursor as table_row;
use crate::commands::tui::nav::Page;
use crate::commands::tui::pane::Framed;
use crate::commands::tui::session::YearCursor;
use crate::commands::tui::success::Successes;
use crate::commands::tui::support::{
    Headless, SIZE, TEST_PLAN, TODAY, active_page, commit_edit, composed_frame, headless_app,
    headless_app_at, ledger_year, overview_year, press_key, redrawn, said, scratch_full_plan,
    scratch_plan, searched_app, show,
};
use crate::commands::tui::tools::settle_all;

fn cursor(app: &App) -> Option<i16> {
    app.world().resource::<YearCursor>().0
}

fn holder(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

/// The row under the cursor of the list the keyboard is in.
fn highlighted(app: &App) -> Option<Entity> {
    let list = holder(app).expect("a list holds the keyboard");
    app.world().get::<ActiveDescendant>(list).unwrap().0
}

/// The title of the pane the keyboard is in; the Success tile has none.
pub(super) fn held_title(app: &mut App) -> String {
    let held = holder(app).expect("a pane holds the keyboard");
    if app.world().get::<TileAt>(held).is_some() {
        return "Success".to_owned();
    }
    let pane = app
        .world()
        .get::<bevy_ecs::hierarchy::ChildOf>(held)
        .unwrap()
        .parent();
    let title = &app.world().get::<Framed>(pane).unwrap().title;
    title.split(" · ").next().unwrap_or_default().to_owned()
}

/// Success runs again for a changed plan while the Overview is shown,
/// and answers.
#[test]
fn success_runs_while_the_overview_is_shown_and_answers() {
    let mut app = searched_app(scratch_plan(), TEST_PLAN, SIZE);
    let frame = redrawn(&mut app);
    assert!(frame.contains("% of 20"), "{frame}");
    commit_edit(&mut app, |plan| plan.expenses[0].amount = 50_000);
    app.update();
    assert!(app.world().resource::<Successes>().is_running());
    settle_all(&mut app);
    let frame = redrawn(&mut app);
    assert!(frame.contains("% of 20"), "{frame}");
    assert!(!frame.contains("running"), "{frame}");
}

#[test]
fn a_headless_overview_runs_nothing_by_itself() {
    let mut app = headless_app(SIZE);
    app.update();
    assert!(!app.world().resource::<Successes>().is_running());
    assert!(!app.world().resource::<super::Better>().is_running());
    let frame = composed_frame(&app);
    assert!(frame.contains(" waiting"), "{frame}");
}

#[test]
fn enter_on_success_opens_monte_carlo() {
    let mut app = headless_app(SIZE);
    assert_eq!(held_title(&mut app), "Success", "the page is entered on it");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert_eq!(active_page(&app), Page::MonteCarlo);
}

#[test]
fn v_turns_the_chart_through_its_views_and_back() {
    let mut app = headless_app(SIZE);
    let titles = [
        "Net worth · ",
        "Income against taxes · ",
        "Balances by tax treatment · ",
    ];
    for title in titles {
        press_key(&mut app, KeyCode::Char('v'));
        let frame = redrawn(&mut app);
        assert!(frame.contains(&format!("╭ {title}")), "{title}: {frame}");
    }
    assert!(frame_has_key(&mut app), "the balances are keyed");
    press_key(&mut app, KeyCode::Char('v'));
    assert!(!frame_has_key(&mut app), "the key goes with them");
}

fn frame_has_key(app: &mut App) -> bool {
    redrawn(app).contains("██ HSA  ░░ pre-tax  ▒▒ Roth  ▓▓ taxable")
}

#[test]
fn arrows_walk_the_year_within_the_plan_and_every_view_follows() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Left);
    assert_eq!(cursor(&app), None, "no year before the first");
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Right);
    assert_eq!(overview_year(&mut app), TODAY.0 + 2);
    press_key(&mut app, KeyCode::Left);
    assert_eq!(overview_year(&mut app), TODAY.0 + 1);
    show(&mut app, Page::Ledger);
    assert_eq!(ledger_year(&mut app), TODAY.0 + 1);
}

#[test]
fn tab_reaches_every_pane_in_drawn_order() {
    let mut app = headless_app(SIZE);
    let mut walked = vec![held_title(&mut app)];
    for _ in 0..5 {
        press_key(&mut app, KeyCode::Tab);
        walked.push(held_title(&mut app));
    }
    let expected = [
        "Success",
        "Milestones",
        "Needs attention",
        "Balances by tax treatment",
        "2026",
        "Could do better",
    ];
    assert_eq!(walked, expected);
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(held_title(&mut app), "Success", "and round again");
}

/// The year's arrows are the page's: they do not also walk the keyboard
/// to the pane beside.
#[test]
fn an_arrow_moves_the_year_and_leaves_the_keyboard_where_it_is() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    press_key(&mut app, KeyCode::Tab);
    let milestones = holder(&app);
    press_key(&mut app, KeyCode::Down);
    let row = highlighted(&app);
    press_key(&mut app, KeyCode::Right);
    assert_eq!(cursor(&app), Some(TODAY.0 + 1));
    assert_eq!(holder(&app), milestones, "→ stayed in Milestones");
    assert_eq!(highlighted(&app), row, "on the row it was on");
    for _ in 0..3 {
        press_key(&mut app, KeyCode::Tab);
    }
    let todo = holder(&app);
    assert_eq!(held_title(&mut app), "2027");
    press_key(&mut app, KeyCode::Left);
    assert_eq!(cursor(&app), Some(TODAY.0));
    assert_eq!(holder(&app), todo, "← stayed in To do");
}

/// Walks the keyboard to the pane titled `title`.
pub(super) fn hold(app: &mut App, title: &str) {
    for _ in 0..6 {
        if held_title(app) == title {
            return;
        }
        press_key(app, KeyCode::Tab);
    }
    panic!("no pane is titled {title}");
}

/// The Overview over the test plan spending past its means.
fn short_app() -> Headless {
    let path = scratch_plan();
    let plan = TEST_PLAN.replace("amount = 60000", "amount = 400000");
    std::fs::write(&path, plan).unwrap();
    headless_app_at(path, SIZE)
}

#[test]
fn enter_on_a_milestone_opens_the_ledger_on_its_year() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    hold(&mut app, "Milestones");
    let frame = redrawn(&mut app);
    assert!(frame.contains("2037 retire "), "{frame}");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Ledger);
    assert_eq!(cursor(&app), Some(2037));
    show(&mut app, Page::Ledger);
    assert_eq!(
        ledger_year(&mut app),
        2037,
        "the Ledger's row is the year's"
    );
}

#[test]
fn e_opens_the_item_behind_a_milestone_on_its_page() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    hold(&mut app, "Milestones");
    let frame = redrawn(&mut app);
    assert!(frame.contains("2032 inheritance arrives"), "{frame}");
    press_key(&mut app, KeyCode::Char('e'));
    assert_eq!(active_page(&app), Page::Income);
    app.update();
    assert_eq!(table_row(&mut app), Row(3), "the inheritance is fourth");
}

/// Medicare is no item of the plan's: its person is.
#[test]
fn e_on_a_person_s_milestone_opens_the_person() {
    let mut app = headless_app(SIZE);
    hold(&mut app, "Milestones");
    let frame = redrawn(&mut app);
    assert!(frame.contains("2045 Medicare · me"), "{frame}");
    press_key(&mut app, KeyCode::Char('e'));
    assert_eq!(active_page(&app), Page::People);
    app.update();
    assert_eq!(table_row(&mut app), Row(0));
}

#[test]
fn e_with_no_item_behind_the_row_is_refused() {
    let mut app = short_app();
    press_key(&mut app, KeyCode::Char('e'));
    assert_eq!(
        active_page(&app),
        Page::Overview,
        "the Success tile has none"
    );
    hold(&mut app, "Needs attention");
    let frame = redrawn(&mut app);
    assert!(frame.contains("2026 on: unfunded, "), "{frame}");
    press_key(&mut app, KeyCode::Char('e'));
    assert_eq!(active_page(&app), Page::Overview);
    let refusals = said(&app);
    let refusals = refusals
        .iter()
        .filter(|line| line.contains(super::rows::NOTHING_TO_EDIT));
    assert_eq!(refusals.count(), 2, "{:?}", said(&app));
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Ledger, "⏎ still opens its year");
}

#[test]
fn an_issue_row_opens_its_item_on_enter() {
    let mut app = headless_app(SIZE);
    let frame = redrawn(&mut app);
    assert!(frame.contains("No plan issues"), "{frame}");
    commit_edit(&mut app, |plan| plan.accounts[1].balance = -1);
    hold(&mut app, "Needs attention");
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("1 issue · Accounts › k › Balance:"),
        "{frame}"
    );
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Accounts);
    app.update();
    assert_eq!(table_row(&mut app), Row(1));
}
