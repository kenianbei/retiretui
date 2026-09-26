//! Headless tests for the market tools: searching by themselves, what the
//! Assumptions pane turns to, and a run opened in the Ledger.

use std::time::Duration;

use bevy_app::App;
use bevy_ecs::prelude::{Entity, With};
use plurimus::term::KeyCode;
use retiretui_engine::market::{MonteCarlo, Runs};
use retiretui_engine::plan::Market;

use super::MarketTool;
use super::views::{View, ViewPart};
use crate::commands::tui::nav::Page;
use crate::commands::tui::pane::Framed;
use crate::commands::tui::session::{LedgerRun, Projected};
use crate::commands::tui::support::{
    Headless, SIZE, active_page, assert_at_rest, click_year, commit_edit, headless_app,
    ledger_year, press_key, redrawn, show,
};
use crate::commands::tui::theme::Theme;
use crate::commands::tui::tools::{ResultPane, Tool, settle_all};

/// Hands the keyboard to the runs, from the assumptions the page opens on.
fn to_runs(app: &mut Headless) {
    press_key(app, KeyCode::Tab);
}

/// The shell, shown `page` once its search has answered.
fn app_on(page: Page) -> Headless {
    let mut app = headless_app(SIZE);
    show(&mut app, page);
    settle_all(&mut app);
    app
}

fn ledger_run(app: &App) -> Option<String> {
    let run = app.world().resource::<LedgerRun>();
    run.0.as_ref().map(|(label, _)| label.clone())
}

#[test]
fn monte_carlo_searches_by_itself_and_says_how_the_plan_fares() {
    let mut app = app_on(Page::MonteCarlo);
    let frame = redrawn(&mut app);
    assert!(frame.contains("money lasts in"), "{frame}");
    assert!(
        frame.contains("of 1,000") && frame.contains("Worst"),
        "{frame}"
    );
    assert!(
        frame.contains("Money lasts"),
        "the verdict heads the assumptions: {frame}"
    );
}

#[test]
fn an_edit_searches_again_under_it() {
    let mut app = app_on(Page::MonteCarlo);
    commit_edit(&mut app, |plan| {
        let market = plan.market.get_or_insert_with(Market::default);
        market.monte_carlo.get_or_insert_default().trials = Some(40);
    });
    settle_all(&mut app);
    let found = app
        .world()
        .resource::<Tool<MonteCarlo>>()
        .found()
        .map(|found| found.runs.runs.len());
    assert_eq!(found, Some(40));
    assert!(redrawn(&mut app).contains("of 40"));
}

#[test]
fn enter_on_an_assumption_turns_to_where_it_is_edited() {
    let mut app = app_on(Page::MonteCarlo);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Market);
    show(&mut app, Page::MonteCarlo);
    for _ in 0..8 {
        press_key(&mut app, KeyCode::Down);
    }
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        active_page(&app),
        Page::Accounts,
        "the accounts no mix is held in"
    );
}

#[test]
fn enter_on_a_run_opens_it_in_the_ledger_until_esc_or_an_edit() {
    let mut app = app_on(Page::MonteCarlo);
    let plan_before = app.world().resource::<Projected>().projection.clone();
    to_runs(&mut app);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Ledger);
    assert_eq!(
        ledger_run(&app).as_deref(),
        Some("the 90th percentile market")
    );
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Ledger · the 90th percentile market"),
        "{frame}"
    );
    let projected = &app.world().resource::<Projected>().projection;
    assert_eq!(*projected, plan_before, "the overview still shows the plan");
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(ledger_run(&app), None);
    assert!(!redrawn(&mut app).contains("esc returns"));
    show(&mut app, Page::MonteCarlo);
    to_runs(&mut app);
    press_key(&mut app, KeyCode::Enter);
    assert!(ledger_run(&app).is_some());
    commit_edit(&mut app, |plan| plan.plan.inflation = 0.03);
    app.update();
    assert_eq!(
        ledger_run(&app),
        None,
        "a changed plan leaves the run behind"
    );
}

#[test]
fn the_cursor_rests_on_as_planned_and_enter_opens_the_plan_s_own_ledger() {
    let mut app = app_on(Page::MonteCarlo);
    to_runs(&mut app);
    press_key(&mut app, KeyCode::Enter);
    assert!(ledger_run(&app).is_some(), "a run to leave behind");
    show(&mut app, Page::MonteCarlo);
    to_runs(&mut app);
    press_key(&mut app, KeyCode::Up);
    let tool = app.world().resource::<Tool<MonteCarlo>>();
    assert_eq!(
        tool.highlighted, 0,
        "on the plan's own row, not bounced off"
    );
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(active_page(&app), Page::Ledger);
    assert_eq!(ledger_run(&app), None, "the plan's own projection");
}

#[test]
fn the_runs_title_is_drawn_in_the_colour_of_the_plan_s_zone() {
    let mut app = app_on(Page::MonteCarlo);
    let share = {
        let tool = app.world().resource::<Tool<MonteCarlo>>();
        tool.found().expect("searched").runs.success_rate()
    };
    let theme = app.world().resource::<Theme>().clone();
    let styles: Vec<_> = app
        .world_mut()
        .query_filtered::<&Framed, With<ResultPane<MonteCarlo>>>()
        .iter(app.world())
        .map(|pane| pane.title_style)
        .collect();
    assert_eq!(styles, [Some(super::zone_style(share, &theme))]);
}

#[test]
fn historical_lists_every_start_year_worst_first() {
    let mut app = app_on(Page::Historical);
    let tool = app.world().resource::<Tool<Runs>>();
    let runs = tool.found().expect("searched");
    let listed = runs.listed();
    assert_eq!(listed.len(), 155);
    let worst = &runs.runs[runs.worst_first()[0]];
    assert_eq!(listed[0].1, worst);
    assert!(redrawn(&mut app).contains("survived"));
}

#[test]
fn a_restart_counts_its_runs_and_keeps_the_last_answer_until_it_lands() {
    let mut app = app_on(Page::MonteCarlo);
    let mut tool = app.world_mut().resource_mut::<Tool<MonteCarlo>>();
    let plan = retiretui_engine::plan::Plan::from_toml_str(include_str!(
        "../../../../../../retiretui_engine/tests/fixtures/full.toml"
    ))
    .unwrap();
    tool.restart(plan, 5, |_, _| {
        std::thread::sleep(Duration::from_millis(200));
        Err(Vec::new())
    });
    assert_eq!(tool.note(), "running 0 of 5");
    assert!(tool.found().is_some(), "the last answer stays on show");
}

/// The titles `v` walks the chart pane through, from the bands on.
fn view_titles(app: &mut App) -> Vec<&'static str> {
    const TITLES: [&str; 4] = ["Net Worth ·", "By Year ·", "Still Funded ·", "Ends With ·"];
    let mut seen = Vec::new();
    for _ in 0..TITLES.len() {
        let frame = redrawn(app);
        seen.extend(TITLES.iter().find(|title| frame.contains(**title)));
        press_key(app, KeyCode::Char('v'));
    }
    seen
}

#[test]
fn v_cycles_the_views_historical_without_the_by_year_table() {
    let mut app = app_on(Page::MonteCarlo);
    assert_eq!(
        view_titles(&mut app),
        ["Net Worth ·", "By Year ·", "Still Funded ·", "Ends With ·"]
    );
    assert!(redrawn(&mut app).contains("Net Worth ·"), "and round again");
    let mut app = app_on(Page::Historical);
    assert_eq!(
        view_titles(&mut app),
        [
            "Net Worth ·",
            "Still Funded ·",
            "Ends With ·",
            "Net Worth ·"
        ]
    );
}

#[test]
fn a_click_on_the_runs_chart_moves_the_ledger() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::Ledger);
    show(&mut app, Page::MonteCarlo);
    settle_all(&mut app);
    let mut charts = app.world_mut().query::<(Entity, &ViewPart)>();
    let bands = charts
        .iter(app.world())
        .find(|(_, part)| part.0 == Page::MonteCarlo && part.1 == View::Bands);
    let (chart, _) = bands.expect("the Monte Carlo bands");
    click_year(&mut app, chart, 2040);
    show(&mut app, Page::Ledger);
    assert_eq!(ledger_year(&mut app), 2040);
    assert_at_rest(&mut app);
}
