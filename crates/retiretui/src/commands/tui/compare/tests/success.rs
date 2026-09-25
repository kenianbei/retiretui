use retiretui_engine::plan::{Market, Plan};

use super::*;
use crate::commands::tui::success::{Success, Successes};
use crate::commands::tui::tools::settle_all;

fn row_plans(app: &App) -> Vec<Plan> {
    let document = app.world().resource::<Projected>().plan.clone();
    let compared = app.world().resource::<Compared>().docs.iter();
    std::iter::once(document)
        .chain(compared.map(|doc| doc.projected.plan.clone()))
        .collect()
}

fn successes(app: &App) -> Vec<Success> {
    let held = app.world().resource::<Successes>();
    row_plans(app).iter().map(|plan| held.of(plan)).collect()
}

fn is_running(app: &App) -> bool {
    app.world().resource::<Successes>().is_running()
}

fn is_answered(success: &Success) -> bool {
    matches!(success, Success::Rate(_))
}

fn set_trials(plan: &mut Plan, trials: u32) {
    let market = plan.market.get_or_insert_with(Market::default);
    market.monte_carlo.get_or_insert_default().trials = Some(trials);
}

fn cut_trials(plan: &mut Plan) {
    set_trials(plan, 10);
}

#[test]
fn every_row_answers_with_its_share_of_markets_survived() {
    let mut app = comparing_variant(SIZE);
    settle_all(&mut app);
    let answered = successes(&app);
    assert!(answered.iter().all(is_answered), "{answered:?}");
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Success") && !frame.contains("running"),
        "{frame}"
    );
}

#[test]
fn an_answered_plan_is_not_run_again() {
    let mut app = comparing_variant(SIZE);
    settle_all(&mut app);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('b'));
    app.update();
    assert!(!is_running(&app), "the rows moved, their plans did not");
}

#[test]
fn a_changed_plan_runs_again() {
    let mut app = comparing_variant(SIZE);
    settle_all(&mut app);
    commit_edit(&mut app, cut_trials);
    app.update();
    let document = successes(&app)[0];
    assert_eq!(document, Success::Running { done: 0, total: 10 });
    settle_all(&mut app);
    assert!(successes(&app).iter().all(is_answered));
}

#[test]
fn nothing_runs_behind_another_page() {
    let mut app = comparing_variant(SIZE);
    settle_all(&mut app);
    commit_edit(&mut app, |plan| set_trials(plan, 1000));
    app.update();
    assert!(is_running(&app), "the Compare page runs its rows");
    show(&mut app, Page::Overview);
    assert!(!is_running(&app), "leaving the page stops the run");
    commit_edit(&mut app, cut_trials);
    redrawn(&mut app);
    assert!(!is_running(&app));
    assert_eq!(successes(&app)[0], Success::Waiting);
}

#[test]
fn a_difference_reads_in_points() {
    let (own, base) = (Success::Rate(0.957), Success::Rate(0.945));
    assert_eq!(own.against(base), "+1.2 pts");
    assert_eq!(base.against(own), "-1.2 pts");
    assert_eq!(own.against(own), "same");
    let running = Success::Running {
        done: 340,
        total: 1000,
    };
    assert_eq!(running.against(base), "running 340 of 1,000");
    assert_eq!(own.against(running), "95.7%");
}

#[test]
fn difference_turns_the_answered_success_to_points() {
    let mut app = comparing_variant(SIZE);
    settle_all(&mut app);
    press_key(&mut app, KeyCode::Char('d'));
    let frame = redrawn(&mut app);
    assert!(frame.contains("same") || frame.contains(" pts"), "{frame}");
}
