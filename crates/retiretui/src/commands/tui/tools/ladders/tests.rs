use bevy_app::App;
use plurimus::term::KeyCode;
use toml::Value;

use super::*;
use crate::commands::tui::compare::Compared;
use crate::commands::tui::confirm::Confirm;
use crate::commands::tui::edit::ToolAnswers;
use crate::commands::tui::edit::tests::{open, tab_to_field};
use crate::commands::tui::support::{
    Headless, ROOMY, SIZE, commit_edit, composed_frame, headless_app, headless_app_at, is_asking,
    press_ctrl, press_key, press_shift, redrawn, said, scratch_full_plan, scratch_scenario,
    scratch_workspace, show, type_text,
};
use crate::commands::tui::tools::write::SAVE_FIRST;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Scenario;

fn tool(app: &App) -> toml::Table {
    app.world().resource::<Draft>().answers::<Constraints>()
}

fn constrain(app: &mut App, key: &str, value: Value) {
    let mut held = tool(app);
    held.insert(key.to_owned(), value);
    app.world_mut()
        .resource_mut::<Draft>()
        .tools
        .insert(Constraints::SLOT.to_owned(), Value::Table(held));
}

fn settle(app: &mut App) {
    super::super::settle::<Swept>(app);
}

/// Hands the keyboard to the options, from the constraints the page
/// opens on: back past the sidebar and the conversions.
fn to_options(app: &mut App) {
    for _ in 0..3 {
        press_shift(app, KeyCode::Tab);
    }
}

/// Answers the question on show with the answer the keyboard opens on.
fn answer_yes(app: &mut App) {
    assert!(app.world().resource::<Confirm>().is_open(), "nothing asked");
    press_key(app, KeyCode::Enter);
    app.update();
}

fn highlighted_rate(app: &App) -> String {
    let ladders = app.world().resource::<Ladders>();
    rate_label(ladders.highlighted_bracket().unwrap().rate)
}

/// The options pane's rows as drawn, each with whether the cursor marks
/// it: the right-hand column's lines from under its header to its foot.
pub(crate) fn table_rows(frame: &str) -> Vec<(bool, Vec<String>)> {
    const CHROME: [char; 8] = ['│', '▌', '▏', '▲', '▼', '█', '░', '║'];
    frame
        .lines()
        .skip_while(|line| !line.contains(" unfunded "))
        .skip(1)
        .take_while(|line| !line.contains('╰'))
        .map_while(|line| line.rsplit_once("││").map(|(_, pane)| pane))
        .map(|pane| {
            let is_marked = pane.starts_with(['▌', '▏']);
            let cells = pane.replace(CHROME, " ");
            let cells = cells.split_whitespace().map(str::to_owned).collect();
            (is_marked, cells)
        })
        .collect()
}

fn type_start_year(app: &mut App) {
    open(app, Page::RothConversions);
    tab_to_field(app, 3);
    type_text(app, "2030");
    press_key(app, KeyCode::Enter);
}

#[test]
fn applying_the_form_stores_the_constraints_and_leaves_the_draft_clean() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::RothConversions);
    let frame = composed_frame(&app);
    for text in [
        "Constraints",
        "Convert from",
        "Fill bracket",
        "MAGI cap",
        "Ladder Options",
        "Ranked here once",
        "Conversions",
    ] {
        assert!(frame.contains(text), "{text}: {frame}");
    }
    type_start_year(&mut app);
    assert_eq!(tool(&app).get("start_year"), Some(&Value::Integer(2030)));
    assert!(!app.world().resource::<Draft>().is_dirty());
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("nothing to undo")
    );
    press_key(&mut app, KeyCode::Char('r'));
    assert_eq!(
        tool(&app).get("start_year"),
        Some(&Value::Integer(2030)),
        "kept across a reload"
    );
}

#[test]
fn a_scenario_session_takes_constraints_too() {
    let scenario = scratch_scenario();
    let mut app = headless_app_at(scenario, SIZE);
    type_start_year(&mut app);
    assert_eq!(tool(&app).get("start_year"), Some(&Value::Integer(2030)));
}

#[test]
fn applying_a_destination_searches_by_itself_and_the_help_follows_the_keyboard() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    show(&mut app, Page::RothConversions);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("Convert to"),
        "the constraints to read: {frame}"
    );
    assert!(app.world().resource::<Ladders>().running.is_none());
    press_key(&mut app, KeyCode::Enter);
    tab_to_field(&mut app, 1);
    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    settle(&mut app);
    assert!(
        app.world().resource::<Ladders>().found().is_some(),
        "{:?} {:?} {}",
        said(&app),
        tool(&app),
        redrawn(&mut app)
    );
    assert!(redrawn(&mut app).contains("⏎ edits a constraint"));
    to_options(&mut app);
    let frame = redrawn(&mut app);
    assert!(frame.contains("⏎ takes the highlighted ladder"), "{frame}");
    assert_eq!(table_rows(&frame)[0].1[0], "Current", "{frame}");
}

#[test]
fn without_a_destination_applying_searches_nothing() {
    let mut app = headless_app(SIZE);
    open(&mut app, Page::RothConversions);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert!(app.world().resource::<Ladders>().running.is_none());
    assert!(app.world().resource::<Ladders>().found().is_none());
}

#[test]
fn a_search_needs_a_destination_and_a_valid_draft() {
    let mut app = headless_app(SIZE);
    show(&mut app, Page::RothConversions);
    assert!(!app.world().resource::<Ladders>().is_running());
    constrain(&mut app, "to", Value::String("k".to_owned()));
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Ladders>().refused.as_deref(),
        Some("must be a Roth account"),
        "a destination is searched, and the engine answers"
    );
    commit_edit(&mut app, |plan| plan.plan.start_year = 1000);
    app.update();
    assert!(
        !app.world().resource::<Ladders>().is_running(),
        "an invalid draft is not searched"
    );
}

#[test]
fn constraints_that_hold_nothing_leave_the_last_search_standing() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    show(&mut app, Page::RothConversions);
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    settle(&mut app);
    assert!(app.world().resource::<Ladders>().found().is_some());
    let mut held = tool(&app);
    held.remove("to");
    app.world_mut()
        .resource_mut::<Draft>()
        .tools
        .insert(Constraints::SLOT.to_owned(), Value::Table(held));
    app.update();
    assert!(!app.world().resource::<Ladders>().is_running());
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    app.update();
    let ladders = app.world().resource::<Ladders>();
    assert!(!ladders.is_running(), "the constraints searched last");
    assert!(ladders.found().is_some());
}

#[test]
fn a_search_arrives_ranked_best_first_and_arrows_step_over_the_plan_s_own_row() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    show(&mut app, Page::RothConversions);
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    app.update();
    app.update();
    if app.world().resource::<Ladders>().is_running() {
        let frame = composed_frame(&app);
        assert!(frame.contains("Ladder Options · searching… "), "{frame}");
    }
    settle(&mut app);
    let frame = redrawn(&mut app);
    assert!(frame.contains("s ─"), "the title says how long: {frame}");
    let expected: Vec<String> = {
        let plan = &app.world().resource::<Draft>().plan;
        let (options, rate) = held(app.world().resource::<Draft>()).unwrap();
        search(plan, &TaxTables::embedded(), &options, rate)
            .unwrap()
            .brackets
            .iter()
            .map(|bracket| rate_label(bracket.rate))
            .collect()
    };
    assert!(expected.len() > 1, "a sweep");
    let rows = table_rows(&frame);
    let labels: Vec<&str> = rows[1..]
        .iter()
        .map(|(_, cells)| cells[0].as_str())
        .collect();
    assert_eq!(labels, expected, "best first: {frame}");

    assert_eq!(highlighted_rate(&app), expected[0]);
    to_options(&mut app);
    press_key(&mut app, KeyCode::Down);
    assert_eq!(highlighted_rate(&app), expected[1]);
    let rows = table_rows(&redrawn(&mut app));
    assert!(
        rows[2].0 && !rows[1].0 && !rows[0].0,
        "one row is marked: {rows:?}"
    );
    press_key(&mut app, KeyCode::Up);
    press_key(&mut app, KeyCode::Up);
    app.update();
    assert_eq!(
        highlighted_rate(&app),
        expected[0],
        "the plan's own row is passed over"
    );
}

#[test]
fn one_bracket_gives_one_row_and_an_engine_refusal_is_shown() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    show(&mut app, Page::RothConversions);
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    constrain(&mut app, "bracket", Value::Integer(22));
    settle(&mut app);
    let sweep = app
        .world()
        .resource::<Ladders>()
        .found()
        .map(|swept| &swept.sweep)
        .unwrap();
    assert_eq!(sweep.brackets.len(), 1);

    constrain(&mut app, "bracket", Value::Integer(99));
    settle(&mut app);
    let ladders = app.world().resource::<Ladders>();
    assert!(ladders.found().is_none());
    assert_eq!(ladders.said(), "no bracket with rate 0.99");
    assert!(redrawn(&mut app).contains("no bracket with rate 0.99"));
}

#[test]
fn an_edit_during_a_search_drops_the_result() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    let draft = app.world().resource::<Draft>();
    let (options, _) = held(draft).unwrap();
    let plan = draft.plan.clone();
    app.world_mut()
        .resource_mut::<Ladders>()
        .start(plan, move |plan| {
            let sweep = sweep_brackets(plan, &TaxTables::embedded(), &options)?;
            Ok(Swept { sweep, options })
        });
    commit_edit(&mut app, |plan| plan.plan.name = Some("changed".to_owned()));
    settle(&mut app);
    let ladders = app.world().resource::<Ladders>();
    assert!(ladders.found().is_none() && ladders.refused.is_none());
}

/// A workspace holding the full fixture as plan.toml, opened at `size`.
fn workspace_app(size: plurimus::core::TerminalSize) -> (std::path::PathBuf, Headless) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../retiretui_engine/tests/fixtures/full.toml");
    let dir = scratch_workspace(&std::fs::read_to_string(fixture).unwrap());
    let app = headless_app_at(dir.join("plan.toml"), size);
    (dir, app)
}

fn run_write(app: &mut App) -> Outcome {
    app.world_mut().run_system_cached(write_picker).unwrap()
}

#[test]
fn the_highlighted_option_lists_its_ladder_as_the_engine_searched_it() {
    let (_, mut app) = workspace_app(ROOMY);
    show(&mut app, Page::RothConversions);
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    settle(&mut app);
    let frame = redrawn(&mut app);
    assert!(frame.contains("Taxable"), "{frame}");
    let (rate, steps) = {
        let bracket = app
            .world()
            .resource::<Ladders>()
            .highlighted_bracket()
            .unwrap();
        (bracket.rate, bracket.steps.clone())
    };
    let plan = app.world().resource::<Draft>().plan.clone();
    let (options, _) = held(app.world().resource::<Draft>()).unwrap();
    let expected = optimize_conversions(&plan, &TaxTables::embedded(), &options, rate).unwrap();
    assert_eq!(steps, expected.ladder.steps, "what the CLI would search");
    assert!(!steps.is_empty(), "the best ladder converts");
    for step in &expected.ladder.steps {
        assert!(
            frame.contains(&format!(" {} ", step.year)),
            "{}: {frame}",
            step.year
        );
    }
    to_options(&mut app);
    press_key(&mut app, KeyCode::Down);
    let next = app
        .world()
        .resource::<Ladders>()
        .highlighted_bracket()
        .unwrap()
        .steps
        .clone();
    let frame = redrawn(&mut app);
    let listed = |year: i16| frame.contains(&format!(" {year} "));
    assert!(next.iter().all(|step| listed(step.year)), "{frame}");
}

#[test]
fn w_on_the_frame_writes_and_the_key_row_says_so() {
    let (_, mut app) = workspace_app(SIZE);
    show(&mut app, Page::RothConversions);
    assert!(composed_frame(&app).contains("w write"));
    press_key(&mut app, KeyCode::Char('w'));
    app.update();
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some(NOTHING_SEARCHED_YET)
    );
}

fn run_adopt(app: &mut App) -> Outcome {
    app.world_mut().run_system_cached(adopt).unwrap()
}

fn conversions(app: &App) -> usize {
    app.world().resource::<Draft>().plan.conversions.len()
}

fn highlighted_steps(app: &App) -> usize {
    let optimize = app.world().resource::<Ladders>();
    optimize.highlighted_bracket().unwrap().steps.len()
}

#[test]
fn t_and_enter_ask_then_take_the_ladder_in_place_of_the_last_one() {
    let mut app = headless_app_at(scratch_full_plan(), SIZE);
    show(&mut app, Page::RothConversions);
    assert!(composed_frame(&app).contains("t take"));
    press_key(&mut app, KeyCode::Char('t'));
    app.update();
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some(NOTHING_SEARCHED_YET)
    );
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    settle(&mut app);
    let own = conversions(&app);
    let best = highlighted_steps(&app);
    assert!(best > 0, "the best bracket has a ladder");
    assert_eq!(run_adopt(&mut app), Outcome::Done);
    let frame = redrawn(&mut app);
    assert!(frame.contains("Take the "), "{frame}");
    assert!(!frame.contains("in place of the"), "no ladder yet: {frame}");
    answer_yes(&mut app);
    assert_eq!(conversions(&app), own + best);
    let draft = app.world().resource::<Draft>();
    assert!(draft.is_dirty());
    assert!(draft.plan.conversions[own..].iter().all(is_ladder));
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some(format!("took {best} conversion(s) into the plan").as_str())
    );
    settle(&mut app);
    assert_eq!(
        highlighted_steps(&app),
        best,
        "searched in place of the ladder taken, not on top of it"
    );
    to_options(&mut app);
    press_key(&mut app, KeyCode::Down);
    let next = highlighted_steps(&app);
    press_key(&mut app, KeyCode::Enter);
    let frame = redrawn(&mut app);
    assert!(frame.contains("in place of the"), "{frame}");
    answer_yes(&mut app);
    assert_eq!(conversions(&app), own + next, "replaced, not stacked");
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(conversions(&app), own + best);
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(conversions(&app), own);
}

#[test]
fn write_overlay_refuses_until_searched_on_a_saved_draft() {
    let (dir, mut app) = workspace_app(SIZE);
    show(&mut app, Page::RothConversions);
    assert_eq!(
        run_write(&mut app),
        Outcome::Refused(NOTHING_SEARCHED_YET.to_owned())
    );
    constrain(&mut app, "to", Value::String("roth-ira".to_owned()));
    settle(&mut app);
    commit_edit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    settle(&mut app);
    assert_eq!(run_write(&mut app), Outcome::Refused(SAVE_FIRST.to_owned()));
    press_ctrl(&mut app, KeyCode::Char('s'));
    assert_eq!(run_write(&mut app), Outcome::Done);
    assert!(redrawn(&mut app).contains("Write overlay"));
    type_text(&mut app, "ladder");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let written = dir.join("ladder.toml");
    let text = std::fs::read_to_string(&written).unwrap();
    assert!(text.contains("base = \"plan.toml\""), "{text}");
    let scenario = Scenario::from_toml_str(&text).unwrap();
    assert!(scenario.is_some(), "a scenario: {text}");
    assert!(text.contains("[[conversions]]"), "{text}");
    assert!(text.contains("\ndate = "), "a native date: {text}");
    assert!(app.world().resource::<Compared>().has(&written));
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("wrote ladder.toml, compared")
    );

    assert_eq!(run_write(&mut app), Outcome::Done);
    app.update();
    type_text(&mut app, "ladder");
    press_key(&mut app, KeyCode::Enter);
    assert!(is_asking(&app));
    assert!(composed_frame(&app).contains("Overwrite ladder.toml?"));
    press_key(&mut app, KeyCode::Esc);
    assert!(!is_asking(&app));
}
