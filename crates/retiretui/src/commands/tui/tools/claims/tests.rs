use std::path::PathBuf;

use bevy_app::App;
use plurimus::term::KeyCode;
use retiretui_engine::optimize::optimize_claims;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Scenario;

use super::people_tests::without_record;
use super::*;
use crate::commands::tui::compare::Compared;
use crate::commands::tui::confirm::Confirm;
use crate::commands::tui::support::{
    Headless, SIZE, commit_edit, composed_frame, headless_app_at, press_ctrl, press_key, redrawn,
    said, scratch_workspace, show, type_text,
};
use crate::commands::tui::tools::hold;

const PARTNER: &str = r#"
[[household.people]]
id = "you"
birth = 1962-01-01
earnings = { 2000 = 50000, 2010 = 60000, 2020 = 70000 }

[[income]]
id = "ss-you"
kind = "social-security"
owner = "you"
start = { age = 70, owner = "you" }
"#;

fn fixture() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claims-plan.toml");
    std::fs::read_to_string(path).unwrap()
}

fn couple() -> String {
    format!(
        "{}{PARTNER}",
        fixture().replace("\"single\"", "\"married-joint\"")
    )
}

/// A workspace holding `plan` as plan.toml, opened on the SSA Benefits
/// page.
fn workspace_app(plan: &str) -> (PathBuf, Headless) {
    let dir = scratch_workspace(plan);
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::SsaBenefits);
    settle(&mut app);
    (dir, app)
}

/// Hands the keyboard to the strategies, from the people it opens on.
fn to_strategies(app: &mut App) {
    press_key(app, KeyCode::Tab);
}

/// Answers the question on show with the answer the keyboard opens on.
fn answer_yes(app: &mut App) {
    assert!(app.world().resource::<Confirm>().is_open(), "nothing asked");
    press_key(app, KeyCode::Enter);
    app.update();
}

fn run_write(app: &mut App) -> Outcome {
    app.world_mut().run_system_cached(write_picker).unwrap()
}

fn run_adopt(app: &mut App) -> Outcome {
    app.world_mut().run_system_cached(adopt).unwrap()
}

fn settle(app: &mut App) {
    super::super::settle::<ClaimSearch>(app);
}

fn highlighted_ages(app: &App) -> Vec<u8> {
    let claims = app.world().resource::<Claims>();
    let (_, candidate) = claims.chosen().unwrap();
    candidate.claims.iter().map(|claim| claim.age).collect()
}

/// The options table's rows under its header, each with whether the bar
/// marks it: the plan's own row first. The options are the rightmost pane,
/// so each line is read from its last pane border.
fn table_rows(frame: &str) -> Vec<(bool, Vec<String>)> {
    const CHROME: [char; 8] = ['│', '▌', '▏', '▲', '▼', '█', '░', '║'];
    frame
        .lines()
        .skip_while(|line| !line.contains(" unfunded "))
        .skip(1)
        .map(|line| {
            let pane = line.rsplit_once("││").map_or(line, |(_, pane)| pane);
            let is_marked = pane.starts_with(['▌', '▏']);
            let cells = pane.replace(CHROME, " ");
            (
                is_marked,
                cells
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
        })
        .take_while(|(_, cells)| !cells.is_empty())
        .collect()
}

fn claim_age(app: &App, id: &str) -> Option<u8> {
    let plan = &app.world().resource::<Draft>().plan;
    let income = plan.income.iter().find(|income| income.id == id)?;
    income.start.as_ref()?.age
}

#[test]
fn the_page_searches_by_itself_beside_the_people() {
    let (_, app) = workspace_app(&fixture());
    let frame = composed_frame(&app);
    assert!(frame.contains("╭ People "), "{frame}");
    assert!(frame.contains("╭ Claim Options "), "{frame}");
    assert!(
        !frame.contains("Every computed benefit"),
        "no form: {frame}"
    );
    let rows = table_rows(&frame);
    assert_eq!(rows[0].1[..2], ["Current", "67"], "{frame}");
    assert!(!rows[0].0, "the plan's own row is never marked: {frame}");
    assert!(
        rows[1].0,
        "the bar is on the best option, not the plan's own row: {frame}"
    );
    assert_eq!(
        rows.len(),
        10,
        "the plan's own row and nine options: {frame}"
    );
    assert!(
        said(&app).is_empty(),
        "a search of its own says nothing: {:?}",
        said(&app)
    );
    assert!(frame.contains("↑↓ person  ⏎ actions"), "{frame}");
    let people_left_of_options = frame.lines().any(|line| {
        line.find("╭ People ")
            .zip(line.find("╭ Claim Options "))
            .is_some_and(|(people, options)| people < options)
    });
    assert!(people_left_of_options, "{frame}");
}

#[test]
fn a_search_arrives_ranked_best_first_and_arrows_step_over_the_plan_s_own_row() {
    let (_, mut app) = workspace_app(&fixture());
    commit_edit(&mut app, |plan| plan.plan.name = Some("changed".to_owned()));
    hold::<ClaimSearch>(&mut app, true);
    app.update();
    assert!(composed_frame(&app).contains("Claim Options · searching… "));
    hold::<ClaimSearch>(&mut app, false);
    settle(&mut app);
    let frame = composed_frame(&app);
    assert!(frame.contains("s ─"), "the title says how long: {frame}");
    let expected = {
        let plan = &app.world().resource::<Draft>().plan;
        optimize_claims(plan, &TaxTables::embedded(), &[], &[]).unwrap()
    };
    let found = app.world().resource::<Claims>().found().unwrap();
    assert_eq!(found.candidates.len(), 9);
    for (ours, theirs) in found.candidates.iter().zip(&expected.candidates) {
        assert_eq!(ours.claims, theirs.claims, "what the CLI would rank");
    }
    let ages_of = |at: usize| vec![expected.candidates[at].claims[0].age];
    assert_eq!(
        table_rows(&frame)[1].1[0],
        ages_of(0)[0].to_string(),
        "best first"
    );

    to_strategies(&mut app);
    assert_eq!(highlighted_ages(&app), ages_of(0));
    press_key(&mut app, KeyCode::Down);
    assert_eq!(highlighted_ages(&app), ages_of(1));
    let rows = table_rows(&redrawn(&mut app));
    assert!(
        rows[2].0 && !rows[1].0 && !rows[0].0,
        "one row is marked: {rows:?}"
    );
    press_key(&mut app, KeyCode::Up);
    press_key(&mut app, KeyCode::Up);
    app.update();
    assert_eq!(
        highlighted_ages(&app),
        ages_of(0),
        "the plan's own row is passed over"
    );
    assert!(!table_rows(&redrawn(&mut app))[0].0);
}

#[test]
fn a_couple_s_options_scroll_to_keep_the_cursor_in_view() {
    let (_, mut app) = workspace_app(&couple());
    let frame = composed_frame(&app);
    let header = frame
        .lines()
        .find(|line| line.contains(" unfunded "))
        .unwrap_or_else(|| panic!("no header: {frame}"));
    assert!(
        header.contains(" me ") && header.contains(" you "),
        "{frame}"
    );
    assert_eq!(table_rows(&frame)[0].1[..3], ["Current", "67", "70"]);
    let found = app.world().resource::<Claims>().found().unwrap();
    assert_eq!(found.candidates.len(), 63);
    to_strategies(&mut app);
    press_key(&mut app, KeyCode::End);
    app.update();
    let frame = redrawn(&mut app);
    let panes: Vec<&str> = frame
        .lines()
        .filter_map(|line| line.rsplit_once("││").map(|(_, pane)| pane))
        .collect();
    let is_row = |pane: &&str| pane.chars().any(|cell| cell.is_ascii_digit());
    let last_row = panes.iter().rposition(is_row);
    let marked = panes.iter().rposition(|pane| pane.starts_with('▌'));
    assert_eq!(
        marked, last_row,
        "the last option is in view and marked: {frame}"
    );
    assert!(
        !frame.contains("Current"),
        "the top rows scrolled away: {frame}"
    );
}

#[test]
fn the_pane_says_why_nothing_could_be_searched_and_an_edit_drops_a_search() {
    let unsearchable = without_record(&fixture())
        .replace("start = { age = 67", "amount = 24000\nstart = { age = 67");
    let (_, app) = workspace_app(&unsearchable);
    assert!(app.world().resource::<Claims>().found().is_none());
    assert!(
        composed_frame(&app).contains("no social-security income computes its benefit"),
        "{}",
        composed_frame(&app)
    );
    assert!(said(&app).is_empty(), "the page's own search says nothing");

    let (_, mut app) = workspace_app(&fixture());
    commit_edit(&mut app, |plan| plan.plan.name = Some("changed".to_owned()));
    hold::<ClaimSearch>(&mut app, true);
    app.update();
    assert!(app.world().resource::<Claims>().is_running());
    commit_edit(&mut app, |plan| {
        plan.plan.name = Some("changed again".to_owned());
    });
    hold::<ClaimSearch>(&mut app, false);
    settle(&mut app);
    assert!(said(&app).is_empty(), "a dropped search says nothing");
    assert!(
        app.world().resource::<Claims>().found().is_some(),
        "the page searched the changed plan by itself"
    );
}

#[test]
fn w_writes_the_highlighted_claims_and_compares_them() {
    let (dir, mut app) = workspace_app(&couple());
    to_strategies(&mut app);
    press_key(&mut app, KeyCode::Down);
    let ages = highlighted_ages(&app);
    assert_eq!(run_write(&mut app), Outcome::Done);
    assert!(redrawn(&mut app).contains("Write overlay"));
    type_text(&mut app, "claims");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let written = dir.join("claims.toml");
    let text = std::fs::read_to_string(&written).unwrap();
    assert!(text.contains("base = \"plan.toml\""), "{text}");
    assert!(Scenario::from_toml_str(&text).unwrap().is_some(), "{text}");
    assert!(text.contains("[[income]]\nid = \"ss\"\n"), "{text}");
    assert!(
        text.contains(&format!(
            "[income.start]\nage = {}\nowner = \"you\"\n",
            ages[1]
        )),
        "{text}"
    );
    assert!(app.world().resource::<Compared>().has(&written));
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("wrote claims.toml, compared")
    );
}

#[test]
fn t_and_enter_ask_then_take_the_chosen_claims_as_one_step() {
    let (_, mut app) = workspace_app(&couple());
    let best: Vec<u8> = app
        .world()
        .resource::<Claims>()
        .found()
        .unwrap()
        .best()
        .claims
        .iter()
        .map(|claim| claim.age)
        .collect();
    assert_eq!(run_adopt(&mut app), Outcome::Done);
    app.update();
    assert!(
        composed_frame(&app).contains(&format!("Take these claims? ss at {}", best[0])),
        "the question names the claims: {}",
        composed_frame(&app)
    );
    answer_yes(&mut app);
    assert_eq!(
        claim_age(&app, "ss"),
        Some(best[0]),
        "the cursor opens on the best"
    );
    press_ctrl(&mut app, KeyCode::Char('z'));
    settle(&mut app);
    to_strategies(&mut app);
    press_key(&mut app, KeyCode::Down);
    let ages = highlighted_ages(&app);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    answer_yes(&mut app);
    assert_eq!(claim_age(&app, "ss"), Some(ages[0]));
    assert_eq!(claim_age(&app, "ss-you"), Some(ages[1]));
    assert!(app.world().resource::<Draft>().is_dirty());
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some(format!("claimed ss at {}, ss-you at {}", ages[0], ages[1]).as_str())
    );
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(
        claim_age(&app, "ss"),
        Some(67),
        "one step back is the plan's own"
    );
    assert_eq!(claim_age(&app, "ss-you"), Some(70));
}

#[test]
fn the_options_come_back_after_nothing_could_be_searched() {
    let (_, mut app) = workspace_app(&couple());
    commit_edit(&mut app, |plan| {
        plan.income.clear();
        for person in &mut plan.household.people {
            person.earnings.clear();
        }
    });
    settle(&mut app);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("no social-security income computes"),
        "{frame}"
    );
    assert!(!frame.contains("Current"), "no options: {frame}");
    press_ctrl(&mut app, KeyCode::Char('z'));
    settle(&mut app);
    let frame = composed_frame(&app);
    assert_eq!(
        table_rows(&frame)[0].1[..3],
        ["Current", "67", "70"],
        "{frame}"
    );
    assert!(
        table_rows(&frame).len() > 2,
        "the options are drawn again: {frame}"
    );
}

#[test]
fn a_held_claim_stays_out_of_the_search() {
    let (_, mut app) = workspace_app(&couple());
    press_key(&mut app, KeyCode::Down);
    assert_eq!(
        app.world_mut().run_system_cached(hold_claim).unwrap(),
        Outcome::Done
    );
    settle(&mut app);
    let found = app.world().resource::<Claims>().found().unwrap();
    assert_eq!(found.incomes, ["ss"], "only me is searched");
    assert_eq!(found.candidates.len(), 9);
    assert!(
        composed_frame(&app).contains(" held "),
        "{}",
        composed_frame(&app)
    );
    assert_eq!(
        app.world_mut().run_system_cached(hold_claim).unwrap(),
        Outcome::Done
    );
    settle(&mut app);
    assert_eq!(
        app.world()
            .resource::<Claims>()
            .found()
            .unwrap()
            .candidates
            .len(),
        63
    );
}
