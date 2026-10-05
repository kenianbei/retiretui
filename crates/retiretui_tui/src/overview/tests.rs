use bevy_app::App;
use bevy_ecs::prelude::Entity;
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;

use super::verdict::TileAt;
use crate::edit::Row;
use crate::edit::tests::cursor as table_row;
use crate::nav::Page;
use crate::pane::Framed;
use crate::session::{Basis, YearCursor};
use crate::success::Successes;
use crate::support::{
    Headless, ROOMY, SIZE, TEST_PLAN, active_page, commit_edit, composed_frame, headless_app,
    headless_app_at, ledger_year, press_key, redrawn, said, scratch_full_plan, scratch_plan,
    searched_app, show, test_projected,
};
use crate::tools::settle_all;

fn cursor(app: &App) -> Option<i16> {
    app.world().resource::<YearCursor>().0
}

fn holder(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
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
    assert!(frame.contains("% of 20 markets"), "it answered: {frame}");
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

/// Walks the keyboard to the pane titled `title`.
pub(super) fn hold(app: &mut App, title: &str) {
    for _ in 0..8 {
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
fn an_issue_row_opens_its_item_on_enter() {
    let mut app = headless_app(SIZE);
    let frame = redrawn(&mut app);
    assert!(frame.contains("No plan issues"), "{frame}");
    commit_edit(&mut app, |plan| plan.accounts[1].balance = -1);
    hold(&mut app, "Needs attention");
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Accounts › k › Balance: must not be negative"),
        "{frame}"
    );
    assert!(
        frame.contains("The figures are the last the plan had"),
        "the figures predate the issue: {frame}"
    );
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Accounts);
    app.update();
    assert_eq!(table_row(&mut app), Row(1));
}

#[test]
fn v_turns_the_chart_through_its_views_and_back_each_keyed() {
    let mut app = headless_app(SIZE);
    let views = [
        ("Net worth · ", "── net worth"),
        ("Income against taxes · ", "── income  ── taxes"),
        (
            "Net worth through random markets · today's dollars",
            " waiting",
        ),
        (
            "Balances by tax treatment · ",
            "██ HSA  ░░ pre-tax  ▒▒ Roth  ▓▓ taxable",
        ),
    ];
    for (title, key) in views {
        press_key(&mut app, KeyCode::Char('v'));
        let frame = redrawn(&mut app);
        assert!(frame.contains(&format!("╭ {title}")), "{title}: {frame}");
        assert!(frame.contains(key), "{key}: {frame}");
    }
}

#[test]
fn the_markets_chart_is_drawn_once_the_runs_answer() {
    let mut app = searched_app(scratch_plan(), TEST_PLAN, SIZE);
    for _ in 0..3 {
        press_key(&mut app, KeyCode::Char('v'));
    }
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("░░ 10th to 90th  ▒▒ 25th to 75th  ── median"),
        "{frame}"
    );
    app.world_mut().resource_mut::<Basis>().nominal = true;
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Net worth through random markets · today's dollars"),
        "the runs are kept in today's dollars alone: {frame}"
    );
}

#[test]
fn the_overview_holds_no_year() {
    let mut app = headless_app(SIZE);
    let before = redrawn(&mut app);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Left);
    assert_eq!(cursor(&app), None, "no arrow walks a year here");
    app.world_mut().resource_mut::<YearCursor>().0 = Some(2040);
    assert_eq!(
        redrawn(&mut app),
        before,
        "a year set elsewhere moves nothing"
    );
}

#[test]
fn tab_reaches_every_pane_in_drawn_order() {
    let mut app = headless_app(SIZE);
    let mut walked = vec![held_title(&mut app)];
    for _ in 0..6 {
        press_key(&mut app, KeyCode::Tab);
        walked.push(held_title(&mut app));
    }
    let expected = [
        "Success",
        "Milestones",
        "Needs attention",
        "Could do better",
        "Balances by tax treatment",
        "Over the plan",
        "Rests on",
    ];
    assert_eq!(walked, expected);
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(held_title(&mut app), "Success", "and round again");
}

#[test]
fn enter_on_the_chart_opens_the_ledger() {
    let mut app = headless_app(SIZE);
    hold(&mut app, "Balances by tax treatment");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert_eq!(active_page(&app), Page::Ledger);
}

#[test]
fn a_tight_pane_reads_the_highlighted_total_s_make_up_beneath_the_list() {
    let mut app = headless_app(SIZE);
    let totals = retiretui_client::overview::View::new(&test_projected(), false).totals;
    hold(&mut app, "Over the plan");
    let frame = redrawn(&mut app);
    assert!(frame.contains(&totals[0].made_of), "{frame}");
    assert!(
        !frame.contains(&totals[1].made_of),
        "one at a time: {frame}"
    );
    press_key(&mut app, KeyCode::Down);
    let frame = redrawn(&mut app);
    assert!(frame.contains(&totals[1].made_of), "{frame}");
    assert!(!frame.contains(&totals[0].made_of), "{frame}");
}

#[test]
fn a_roomy_pane_says_every_total_s_make_up_at_once() {
    let mut app = headless_app(ROOMY);
    let totals = retiretui_client::overview::View::new(&test_projected(), false).totals;
    let frame = redrawn(&mut app);
    for total in totals.iter().filter(|total| !total.made_of.is_empty()) {
        assert!(frame.contains(&total.made_of), "{}: {frame}", total.label);
    }
}

#[test]
fn enter_on_a_total_opens_its_page_or_its_tool() {
    let mut app = headless_app(SIZE);
    hold(&mut app, "Over the plan");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Income);
    show(&mut app, Page::Overview);
    hold(&mut app, "Over the plan");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::WithdrawalOrder);
}

#[test]
fn enter_on_an_assumption_opens_the_page_it_is_edited_on() {
    let mut app = headless_app(SIZE);
    hold(&mut app, "Rests on");
    let frame = redrawn(&mut app);
    assert!(frame.contains("Runs through   2050 · to age 70"), "{frame}");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Settings);
}

#[test]
fn e_with_no_item_behind_the_row_is_refused() {
    let mut app = headless_app(SIZE);
    press_key(&mut app, KeyCode::Char('e'));
    assert_eq!(
        active_page(&app),
        Page::Overview,
        "the Success tile has none"
    );
    hold(&mut app, "Needs attention");
    let frame = redrawn(&mut app);
    assert!(frame.contains("No plan issues"), "{frame}");
    press_key(&mut app, KeyCode::Char('e'));
    assert_eq!(active_page(&app), Page::Overview);
    let refusals = said(&app);
    let refusals = refusals
        .iter()
        .filter(|line| line.contains(super::rows::NOTHING_TO_EDIT));
    assert_eq!(refusals.count(), 2, "{:?}", said(&app));
}

#[test]
fn a_plan_that_runs_short_says_so_first_and_leads_to_the_year_and_the_spending() {
    let mut app = short_app();
    let frame = redrawn(&mut app);
    assert!(frame.contains("Short from the start"), "{frame}");
    hold(&mut app, "Needs attention");
    let frame = redrawn(&mut app);
    assert!(frame.contains("▌ Runs short from 2026: "), "{frame}");
    press_key(&mut app, KeyCode::Char('e'));
    assert_eq!(active_page(&app), Page::Expenses);
    show(&mut app, Page::Overview);
    hold(&mut app, "Needs attention");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Ledger);
    assert_eq!(ledger_year(&mut app), 2026);
}

/// The ceiling is the Overview's to search, after the rest; its tool takes
/// the answer in place of searching again.
#[test]
fn the_ceiling_row_leads_to_its_tool_which_takes_what_was_found() {
    let mut app = searched_app(scratch_plan(), TEST_PLAN, ROOMY);
    let plan = app
        .world()
        .resource::<crate::session::Projected>()
        .plan
        .clone();
    let answers = toml::Table::new();
    let better = app.world().resource::<super::Better>();
    let found = better.spending(&plan, &answers).expect("searched").clone();
    hold(&mut app, "Could do better");
    let frame = redrawn(&mut app);
    assert!(frame.contains(" in 90% of markets"), "{frame}");
    press_key(&mut app, KeyCode::End);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::SpendingCeiling);
    app.update();
    let tool = app.world().resource::<crate::tools::spending::Spending>();
    assert!(!tool.is_running(), "it searches nothing again");
    assert_eq!(tool.found(), Some(&found));
}
