use std::path::PathBuf;

use bevy_app::App;
use plurimus::term::KeyCode;
use retiretui_engine::plan::Scenario;
use retiretui_engine::plan::TreatmentClass::{Deferred, Roth, Taxable};

use super::*;
use crate::compare::Compared;
use crate::support::{
    Headless, SIZE, answer_back, composed_frame, headless_app_at, headless_app_bound, is_asking,
    press_ctrl, press_key, redrawn, said, scratch_workspace, show, type_text,
};

/// A retiree drained Roth first, who ends with more drained taxable, Roth,
/// deferred.
pub(crate) const RETIREE: &str =
    include_str!("../../../../retiretui_engine/tests/fixtures/order-plan.toml");

/// A workspace holding `plan` as plan.toml, opened on the Withdrawal Order
/// page with its search answered.
fn workspace_app(plan: &str) -> (PathBuf, Headless) {
    let dir = scratch_workspace(plan);
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::WithdrawalOrder);
    settle(&mut app);
    (dir, app)
}

fn settle(app: &mut App) {
    super::super::settle::<OrderSearch>(app);
}

/// Answers the question on show with the answer the keyboard opens on.
fn answer_yes(app: &mut App) {
    assert!(is_asking(app), "nothing asked");
    answer_back(app, 0);
    app.update();
}

fn stated_order(app: &App) -> Vec<TreatmentClass> {
    app.world()
        .resource::<Draft>()
        .plan
        .plan
        .withdrawal_order
        .clone()
}

/// The table's rows under its header, each with whether the bar marks it
/// and the order it leads with: the plan's own row first.
fn table_rows(frame: &str) -> Vec<(bool, String)> {
    frame
        .lines()
        .skip_while(|line| !line.contains(" Unfunded "))
        .skip(1)
        .filter_map(|line| {
            let (_, pane) = line.split_once("││")?;
            let is_marked = pane.starts_with('▌');
            let order = pane.trim_matches(['▌', ' ', '│']);
            let order = order.split("  ").next().unwrap_or_default();
            (!order.is_empty()).then(|| (is_marked, order.to_owned()))
        })
        .collect()
}

#[test]
fn the_page_ranks_every_order_under_the_plan_s_own_row() {
    let (_, app) = workspace_app(RETIREE);
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Orders "), "{frame}");
    let rows = table_rows(&frame);
    let orders: Vec<&str> = rows.iter().map(|(_, order)| order.as_str()).collect();
    assert_eq!(
        orders[..2],
        ["Current", "Taxable, Roth, deferred"],
        "{frame}"
    );
    assert_eq!(rows.len(), 7, "the plan's own row and six orders: {frame}");
    assert!(orders.contains(&"Roth, deferred, taxable"), "{frame}");
    assert!(!rows[0].0, "the plan's own row is never marked: {frame}");
    assert!(rows[1].0, "the bar is on the best order: {frame}");
    assert!(
        frame.contains("w write") && frame.contains("t take"),
        "{frame}"
    );
    assert!(frame.contains(ON_OPTIONS), "{frame}");
    assert!(said(&app).is_empty(), "{:?}", said(&app));
}

#[test]
fn the_pane_says_why_there_is_nothing_to_order() {
    let cash_only = RETIREE.replace(r#"["roth", "deferred", "taxable"]"#, r#"["taxable"]"#);
    let (_, mut app) = workspace_app(&cash_only);
    let frame = redrawn(&mut app);
    assert!(
        frame.contains(
            "Fewer than two of the listed classes hold an account without a drain priority"
        ),
        "{frame}"
    );
    let adopt = app.world_mut().run_system_cached(adopt).unwrap();
    assert_eq!(adopt, Outcome::Refused(NOTHING_SEARCHED_YET.to_owned()));
}

#[test]
fn w_writes_the_highlighted_order_and_compares_it() {
    let (dir, mut app) = workspace_app(RETIREE);
    press_key(&mut app, KeyCode::Down);
    let write = app.world_mut().run_system_cached(write_picker).unwrap();
    assert_eq!(write, Outcome::Done);
    assert!(redrawn(&mut app).contains("Write overlay"));
    type_text(&mut app, "order");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let written = dir.join("order.toml");
    let text = std::fs::read_to_string(&written).unwrap();
    assert!(text.contains("base = \"plan.toml\""), "{text}");
    assert!(Scenario::from_toml_str(&text).unwrap().is_some(), "{text}");
    assert!(
        text.contains(
            "[plan]\nwithdrawal_order = [\n    \"taxable\",\n    \"deferred\",\n    \"roth\",\n]"
        ),
        "the second order, under the cursor: {text}"
    );
    assert!(app.world().resource::<Compared>().has(&written));
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("wrote order.toml, compared")
    );
}

#[test]
fn t_and_enter_ask_then_take_the_chosen_order_as_one_step() {
    let (_, mut app) = workspace_app(RETIREE);
    let adopt = app.world_mut().run_system_cached(adopt).unwrap();
    assert_eq!(adopt, Outcome::Done);
    app.update();
    let frame = composed_frame(&app);
    assert!(
        frame.contains("Withdraw in this order? Taxable, Roth,"),
        "the question names the order: {frame}"
    );
    answer_yes(&mut app);
    assert_eq!(stated_order(&app), [Taxable, Roth, Deferred]);
    assert!(app.world().resource::<Draft>().is_dirty());
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("now withdrawing in the order taxable, Roth, deferred")
    );
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(
        stated_order(&app),
        [Roth, Deferred, Taxable],
        "one step back is the plan's own"
    );

    settle(&mut app);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    answer_yes(&mut app);
    assert_eq!(stated_order(&app), [Taxable, Deferred, Roth]);
    settle(&mut app);
    let found = app.world().resource::<Orders>().found().unwrap();
    assert_eq!(found.candidates.len(), 6, "the same six outcomes");
    assert_eq!(found.best().order, [Taxable, Roth, Deferred]);
}

#[test]
fn the_help_names_the_key_the_user_gave_writing() {
    let dir = scratch_workspace(RETIREE);
    let help = |keys: &str| {
        let mut app = headless_app_bound(dir.join("plan.toml"), SIZE, keys);
        show(&mut app, Page::WithdrawalOrder);
        settle(&mut app);
        redrawn(&mut app)
    };
    let rebound = help("write-order = \"W\"");
    assert!(
        rebound.contains("after asking; W writes it as a scenario."),
        "{rebound}"
    );
    let unbound = help("write-order = []");
    assert!(unbound.contains("after asking. "), "{unbound}");
}
