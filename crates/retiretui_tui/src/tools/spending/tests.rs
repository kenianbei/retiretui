use std::path::PathBuf;

use bevy_app::App;
use plurimus::term::KeyCode;

use super::*;
use crate::compare::Compared;
use crate::support::{
    Headless, SIZE, answer_back, click, composed_frame, headless_app_at, headless_app_bound,
    is_asking, press_ctrl, press_key, redrawn, said, scratch_workspace, show, type_text,
};

/// A retiree who spends on what they could cut, a mortgage they could
/// not, and a roof once, run through a hundred markets.
const RETIREE: &str = r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 95
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1961-01-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 1200000
allocation = { stocks = 0.6, bonds = 0.4 }

[[expenses]]
id = "living"
name = "Living expenses"
amount = 30000

[[expenses]]
id = "mortgage"
amount = 12000
cola = false
essential = true

[[expenses]]
id = "roof"
amount = 25000
on = { date = 2030-01-01 }

[[expenses]]
id = "travel"
amount = 10000

[market.monte_carlo]
trials = 100
"#;

/// A workspace holding `plan` as plan.toml, opened on the Spending Ceiling
/// page with its search answered.
fn workspace_app(plan: &str) -> (PathBuf, Headless) {
    let dir = scratch_workspace(plan);
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::SpendingCeiling);
    settle(&mut app);
    (dir, app)
}

fn settle(app: &mut App) {
    super::super::settle::<Ceilings>(app);
}

/// Answers the question on show with the answer the keyboard opens on.
fn answer_yes(app: &mut App) {
    assert!(is_asking(app), "nothing asked");
    answer_back(app, 0);
    app.update();
}

fn amounts(app: &App) -> Vec<(String, i64)> {
    let stated = app.world().resource::<Draft>().plan.expenses.iter();
    stated.map(|it| (it.id.clone(), it.amount)).collect()
}

fn ceilings(app: &App) -> &Ceilings {
    app.world().resource::<Spending>().found().unwrap()
}

/// The amount each ceiling makes of `living`, the plan's own market first.
fn living_at(app: &App) -> [i64; 2] {
    ceilings(app)
        .listed()
        .map(|listed| listed.ceiling.expenses[0].amount)
}

/// The Ceilings table's rows under its header, each with whether the bar
/// marks it and what it leads with: the plan's own row first.
fn table_rows(frame: &str) -> Vec<(bool, String)> {
    frame
        .lines()
        .skip_while(|line| !line.contains(" Unfunded "))
        .skip(1)
        .take_while(|line| !line.contains('╰'))
        .filter_map(|line| {
            let (_, pane) = line.split_once("││")?;
            let first = pane.trim_matches(['▌', '▏', ' ', '│']);
            let first = first.split("  ").next().unwrap_or_default();
            Some((pane.starts_with(['▌', '▏']), first.to_owned()))
        })
        .collect()
}

#[test]
fn the_page_lists_both_ceilings_and_starts_on_the_one_at_the_target() {
    let (_, app) = workspace_app(RETIREE);
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ Ceilings "), "{frame}");
    assert_eq!(
        table_rows(&frame),
        [
            (false, "Current".to_owned()),
            (false, "In its own market".to_owned()),
            (true, "In 90% of markets".to_owned())
        ],
        "{frame}"
    );
    let [_, at_target] = living_at(&app);
    let money = MoneyForm::Full.money(at_target);
    assert!(
        frame.contains(&format!("Living expenses   $30,000        {money}")),
        "{frame}"
    );
    assert!(frame.contains("travel            $10,000"), "{frame}");
    assert!(
        !frame.contains("mortgage") && !frame.contains("roof"),
        "{frame}"
    );
    assert!(
        frame.contains("Target success  90%, the default"),
        "{frame}"
    );
    assert!(
        frame.contains("w write") && frame.contains("t take"),
        "{frame}"
    );
    assert!(frame.contains(ON_OPTIONS), "{frame}");
    assert!(said(&app).is_empty(), "{:?}", said(&app));
    assert!(!app.world().resource::<Draft>().is_dirty());
}

#[test]
fn the_ceiling_in_the_plans_own_market_says_it_spends_everything_unless_some_is_held_back() {
    let (_, mut app) = workspace_app(RETIREE);
    press_key(&mut app, KeyCode::Up);
    let frame = redrawn(&mut app);
    assert!(
        table_rows(&frame)[1].0,
        "the bar is on the plan's own market: {frame}"
    );
    assert!(frame.contains(spending::SPENDS_IT_ALL), "{frame}");
    let [planned, _] = living_at(&app);
    let money = MoneyForm::Full.money(planned);
    assert!(
        frame.contains(&format!("$30,000        {money}")),
        "its expenses follow the bar: {frame}"
    );
    press_key(&mut app, KeyCode::Down);
    assert!(redrawn(&mut app).contains(ON_OPTIONS));

    let reserved = format!("{RETIREE}\n[market]\nleave_at_least = 100000\n");
    let (_, mut app) = workspace_app(&reserved);
    press_key(&mut app, KeyCode::Up);
    let frame = redrawn(&mut app);
    assert!(
        frame.contains(ON_OPTIONS) && !frame.contains(spending::SPENDS_IT_ALL),
        "{frame}"
    );
}

#[test]
fn applying_another_target_searches_again_and_leaves_the_draft_clean() {
    let (_, mut app) = workspace_app(RETIREE);
    let [_, at_ninety] = living_at(&app);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Enter);
    assert!(
        redrawn(&mut app).contains(spending::ABOUT),
        "the line fits beside the sidebar"
    );
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "70%");
    press_key(&mut app, KeyCode::Enter);
    let held = app.world().resource::<Draft>().answers::<Answers>();
    assert_eq!(held.get("success"), Some(&toml::Value::Float(0.7)));
    assert!(!app.world().resource::<Draft>().is_dirty());
    settle(&mut app);
    let frame = redrawn(&mut app);
    assert_eq!(table_rows(&frame)[2].1, "In 70% of markets", "{frame}");
    assert!(frame.contains("Target success  70%"), "{frame}");
    let [_, at_seventy] = living_at(&app);
    assert!(
        at_seventy > at_ninety,
        "fewer markets to last in leaves more to spend"
    );
}

#[test]
fn a_target_applied_from_the_question_a_press_outside_asks_is_searched_too() {
    let (_, mut app) = workspace_app(RETIREE);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Tab);
    type_text(&mut app, "70%");
    click(&mut app, 2, 20);
    assert!(is_asking(&app), "{}", redrawn(&mut app));
    answer_back(&mut app, 0);
    settle(&mut app);
    let frame = redrawn(&mut app);
    assert_eq!(table_rows(&frame)[2].1, "In 70% of markets", "{frame}");
}

#[test]
fn t_and_enter_ask_then_take_the_chosen_ceiling_as_one_step() {
    let (_, mut app) = workspace_app(RETIREE);
    let [planned, at_target] = living_at(&app);
    let travel = ceilings(&app).at_target.expenses[1].amount;
    let adopt = app.world_mut().run_system_cached(adopt).unwrap();
    assert_eq!(adopt, Outcome::Done);
    app.update();
    let frame = composed_frame(&app);
    let flexible = MoneyForm::Full.money(at_target + travel);
    assert!(
        frame.contains(&format!("Set flexible spending to {flexible} a year?")),
        "the question names the spending: {frame}"
    );
    answer_yes(&mut app);
    let named = |amounts: [i64; 4]| {
        ["living", "mortgage", "roof", "travel"]
            .map(str::to_owned)
            .into_iter()
            .zip(amounts)
            .collect::<Vec<_>>()
    };
    assert_eq!(amounts(&app), named([at_target, 12_000, 25_000, travel]));
    assert!(app.world().resource::<Draft>().is_dirty());
    assert_eq!(
        said(&app).last().cloned(),
        Some(format!("flexible spending is now {flexible} a year"))
    );
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(
        amounts(&app),
        named([30_000, 12_000, 25_000, 10_000]),
        "one step back is the plan's own"
    );

    settle(&mut app);
    press_key(&mut app, KeyCode::Up);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    answer_yes(&mut app);
    assert_eq!(amounts(&app)[0], ("living".to_owned(), planned));
}

#[test]
fn w_writes_the_highlighted_ceiling_and_compares_it() {
    let (dir, mut app) = workspace_app(RETIREE);
    let expected = spending_overlay("plan.toml", &ceilings(&app).at_target.expenses).unwrap();
    let write = app.world_mut().run_system_cached(write_picker).unwrap();
    assert_eq!(write, Outcome::Done);
    assert!(redrawn(&mut app).contains("Write overlay"));
    type_text(&mut app, "ceiling");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let written = dir.join("ceiling.toml");
    assert_eq!(std::fs::read_to_string(&written).unwrap(), expected);
    assert!(
        expected.contains("id = \"travel\"") && !expected.contains("mortgage"),
        "{expected}"
    );
    assert!(app.world().resource::<Compared>().has(&written));
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("wrote ceiling.toml, compared")
    );
}

#[test]
fn the_pane_says_why_there_is_nothing_to_scale() {
    let fixed = RETIREE.replace("amount = 30000", "amount = 30000\nessential = true");
    let fixed = fixed.replace("amount = 10000", "amount = 10000\nessential = true");
    let (_, mut app) = workspace_app(&fixed);
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("There is no flexible spending to scale"),
        "{frame}"
    );
    assert!(frame.contains(NOT_SEARCHED), "{frame}");
    let adopt = app.world_mut().run_system_cached(adopt).unwrap();
    assert_eq!(adopt, Outcome::Refused(NOTHING_SEARCHED_YET.to_owned()));
    let write = app.world_mut().run_system_cached(write_picker).unwrap();
    assert_eq!(write, Outcome::Refused(NOTHING_SEARCHED_YET.to_owned()));
}

#[test]
fn the_help_names_the_key_the_user_gave_writing() {
    let dir = scratch_workspace(RETIREE);
    let help = |keys: &str| {
        let mut app = headless_app_bound(dir.join("plan.toml"), SIZE, keys);
        show(&mut app, Page::SpendingCeiling);
        settle(&mut app);
        redrawn(&mut app)
    };
    let rebound = help("write-spending = \"W\"");
    assert!(
        rebound.contains("after asking; W writes it as a scenario."),
        "{rebound}"
    );
    let unbound = help("write-spending = []");
    assert!(unbound.contains("after asking. "), "{unbound}");
}
