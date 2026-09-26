use bevy_app::App;
use plurimus::term::KeyCode;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{FilingStatus, Plan};
use retiretui_engine::project::validate_plan;
use toml::Table;

use super::{CANCEL, CREATE, LifeStage, SetupAnswers, TITLE};
use crate::commands::tui::edit::{Draft, ToolAnswers};
use crate::commands::tui::nav::Page;
use crate::commands::tui::session::Session;
use crate::commands::tui::support::{
    self, SETTLING_TICKS, SIZE, active_page, cell_of, click, commit_edit, composed_frame,
    headless_app_at, is_asking, lit_tab, press_ctrl, press_key, press_shift, run_command, said,
    scratch_workspace, type_text,
};

/// The answers a form holding `body` under this filing and stage would
/// have been applied with.
pub(super) fn as_table(filing: FilingStatus, stage: LifeStage, body: &str) -> Table {
    format!(
        "filing = \"{}\"\nstage = \"{}\"\n{body}",
        filing.as_str(),
        stage.as_str()
    )
    .parse()
    .expect("the answers are a table")
}

/// Holds `body` in the form as typing it in would, and lets the form
/// take it up.
fn answer(app: &mut App, filing: FilingStatus, body: &str) {
    let table = as_table(filing, LifeStage::Working, body);
    let mut draft = app.world_mut().resource_mut::<Draft>();
    draft
        .tools
        .insert(SetupAnswers::SLOT.to_owned(), toml::Value::Table(table));
    settle(app);
}

fn settle(app: &mut App) {
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
}

fn press_button(app: &mut App, label: &str) {
    let (x, y) = cell_of(app, label);
    click(app, x, y);
    settle(app);
}

/// Presses `Create` and names the plan `name` in the picker it opens.
fn create_as(app: &mut App, name: &str) {
    press_button(app, CREATE);
    type_text(app, name);
    press_key(app, KeyCode::Enter);
    settle(app);
}

fn document(app: &App) -> Option<std::path::PathBuf> {
    app.world().resource::<Session>().plan_path.clone()
}

const SAM: &str = "name = \"Sam\"\nsalary = 100000\n";

/// A shell launched on a directory holding no plan.
fn empty_shell() -> (std::path::PathBuf, support::Headless) {
    let dir = scratch_workspace("");
    for held in std::fs::read_dir(&dir)
        .expect("the scratch workspace")
        .flatten()
    {
        std::fs::remove_file(held.path()).expect("a file of the fixture's");
    }
    let mut app = headless_app_at(dir.clone(), SIZE);
    settle(&mut app);
    (dir, app)
}

#[test]
fn without_a_document_the_form_is_all_there_is_and_cannot_be_left() {
    let (_, mut app) = empty_shell();
    let frame = composed_frame(&app);
    assert!(frame.contains(TITLE) && frame.contains(CREATE), "{frame}");
    assert!(!frame.contains(CANCEL), "nothing to go back to: {frame}");
    assert!(!frame.contains("╭ Plan "), "no page of a plan: {frame}");
    assert_eq!(
        lit_tab(&mut app),
        Some(crate::commands::tui::nav::Group::Plan.tab())
    );
    press_key(&mut app, KeyCode::Esc);
    settle(&mut app);
    assert!(
        composed_frame(&app).contains(TITLE),
        "esc leaves it standing"
    );
}

#[test]
fn creating_names_the_plan_writes_it_and_lands_in_its_accounts() {
    let (dir, mut app) = empty_shell();
    answer(&mut app, FilingStatus::Single, SAM);
    create_as(&mut app, "ours");

    let written = dir.join("ours.toml");
    let text = std::fs::read_to_string(&written).expect("the plan was written");
    let plan = Plan::from_toml_str(&text).expect("and re-reads");
    assert!(
        validate_plan(&plan, &TaxTables::embedded()).is_empty(),
        "as a plan that validates"
    );
    assert_eq!(document(&app), Some(written), "which the shell now holds");
    assert_eq!(active_page(&app), Page::Accounts);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("╭ Plan ") && frame.contains("│▌ Accounts"),
        "{frame}"
    );

    press_key(&mut app, KeyCode::Right);
    press_key(&mut app, KeyCode::Enter);
    settle(&mut app);
    let frame = composed_frame(&app);
    assert!(
        frame.contains("[ Sam ▾ ]"),
        "an account's owner is one of the people the answers named, by name: {frame}"
    );
}

#[test]
fn an_example_hides_the_questions_and_is_named_after_itself() {
    let (dir, mut app) = empty_shell();
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Right);
    settle(&mut app);
    let frame = composed_frame(&app);
    assert!(frame.contains("Start from"), "{frame}");
    assert!(!frame.contains("Birth year"), "nothing to answer: {frame}");
    press_button(&mut app, CREATE);
    assert!(
        composed_frame(&app).contains("starter.toml"),
        "the name offered is the example's: {}",
        composed_frame(&app)
    );
    press_key(&mut app, KeyCode::Enter);
    settle(&mut app);

    let written = dir.join("starter.toml");
    assert_eq!(document(&app), Some(written.clone()));
    let plan = Plan::from_toml_str(&std::fs::read_to_string(written).unwrap()).unwrap();
    let (_, _, example) = super::examples::named("starter.toml").unwrap();
    let example = Plan::from_toml_str(example).unwrap();
    assert_eq!(plan, example, "the example as it stands");
}

#[test]
fn a_form_no_one_touched_still_makes_a_plan() {
    let (dir, mut app) = empty_shell();
    create_as(&mut app, "blank");
    assert_eq!(document(&app), Some(dir.join("blank.toml")));
    let people = &app.world().resource::<Draft>().plan.household.people;
    assert_eq!(people.len(), 1, "a single filer, by default");
}

#[test]
fn the_plan_is_built_from_the_answers_as_they_stand_at_create() {
    let (_, mut app) = empty_shell();
    answer(&mut app, FilingStatus::Single, SAM);
    answer(&mut app, FilingStatus::Single, "name = \"Pat\"\n");
    create_as(&mut app, "pat");
    let plan = &app.world().resource::<Draft>().plan;
    assert_eq!(plan.household.people[0].id, "pat", "the corrected name");
    assert!(plan.accounts.iter().all(|account| account.owner == "pat"));
}

#[test]
fn a_new_plan_over_a_document_can_be_cancelled_without_touching_it() {
    let mut app = headless_app_at(support::scratch_plan(), SIZE);
    commit_edit(&mut app, |plan| plan.accounts[0].balance = 1);
    let held = document(&app);
    run_command(&mut app, "new");
    settle(&mut app);
    let frame = composed_frame(&app);
    assert!(frame.contains(TITLE) && frame.contains(CANCEL), "{frame}");
    assert_eq!(document(&app), held, "the document stays open beneath");

    press_button(&mut app, CANCEL);
    assert!(!composed_frame(&app).contains(TITLE));
    let draft = app.world().resource::<Draft>();
    assert_eq!(draft.plan.accounts[0].balance, 1, "its draft as it was");
    assert!(draft.is_dirty(), "and still to be saved");
}

#[test]
fn an_unsaved_document_is_asked_about_before_the_new_plan_is_named() {
    let path = scratch_workspace(support::TEST_PLAN).join("plan.toml");
    let mut app = headless_app_at(path.clone(), SIZE);
    commit_edit(&mut app, |plan| plan.accounts[0].balance = 1);
    run_command(&mut app, "new");
    answer(&mut app, FilingStatus::Single, SAM);
    press_button(&mut app, CREATE);
    assert!(is_asking(&app), "what becomes of the unsaved draft");
    let untouched = std::fs::read_to_string(&path).expect("the document");
    assert_eq!(untouched, support::TEST_PLAN, "and nothing is written yet");

    support::answer_back(&mut app, 0);
    settle(&mut app);
    let saved = std::fs::read_to_string(&path).expect("the document");
    assert_ne!(saved, support::TEST_PLAN, "saving wrote the old draft");
    type_text(&mut app, "second");
    press_key(&mut app, KeyCode::Enter);
    settle(&mut app);
    let second = path.with_file_name("second.toml");
    assert_eq!(document(&app), Some(second), "and then the new plan opens");
    let people = &app.world().resource::<Draft>().plan.household.people;
    assert_eq!(people[0].id, "sam", "as the plan the answers made");
    assert_eq!(
        std::fs::read_to_string(&path).expect("the document"),
        saved,
        "the old document as it was saved"
    );
}

#[test]
fn a_name_not_given_returns_to_the_form_with_its_answers() {
    let (_, mut app) = empty_shell();
    answer(&mut app, FilingStatus::Single, SAM);
    press_button(&mut app, CREATE);
    press_key(&mut app, KeyCode::Esc);
    settle(&mut app);
    assert_eq!(document(&app), None, "nothing was written");
    let frame = composed_frame(&app);
    assert!(frame.contains(TITLE) && frame.contains("Sam"), "{frame}");
}

#[test]
fn the_form_keeps_every_key_from_the_document_beneath_it() {
    let mut app = headless_app_at(support::scratch_plan(), SIZE);
    commit_edit(&mut app, |plan| plan.accounts[0].balance = 1);
    run_command(&mut app, "new");
    settle(&mut app);
    press_ctrl(&mut app, KeyCode::Char('z'));
    press_key(&mut app, KeyCode::Char('r'));
    press_key(&mut app, KeyCode::Char('1'));
    settle(&mut app);
    assert!(composed_frame(&app).contains(TITLE), "the form stands");
    let draft = app.world().resource::<Draft>();
    assert_eq!(
        draft.plan.accounts[0].balance, 1,
        "and the draft is as it was"
    );
}

#[test]
fn esc_with_nothing_to_cancel_says_nothing() {
    let (_, mut app) = empty_shell();
    press_key(&mut app, KeyCode::Esc);
    settle(&mut app);
    assert_eq!(support::said(&app), Vec::<String>::new());
}

#[test]
fn the_answers_are_kept_apart_from_the_optimizer_s() {
    let mut app = headless_app_at(support::scratch_plan(), SIZE);
    let constraints: Table = "bracket = 22".parse().expect("a table");
    let slot = "optimizer";
    app.world_mut()
        .resource_mut::<Draft>()
        .tools
        .insert(slot.to_owned(), toml::Value::Table(constraints.clone()));
    run_command(&mut app, "new");
    answer(&mut app, FilingStatus::Single, SAM);
    press_button(&mut app, CREATE);
    press_key(&mut app, KeyCode::Esc);
    settle(&mut app);
    let tools = &app.world().resource::<Draft>().tools;
    assert_eq!(
        tools.get(slot).and_then(toml::Value::as_table),
        Some(&constraints)
    );
}

#[test]
fn the_pane_keys_are_the_forms_own_where_no_page_is_shown() {
    let (_, mut app) = empty_shell();
    let heard = said(&app);
    press_shift(&mut app, KeyCode::Tab);
    assert_eq!(
        said(&app),
        heard,
        "shift-tab has nowhere to go and says so to no one"
    );
    let pane = app.world().resource::<bevy_input_focus::InputFocus>().get();
    press_key(&mut app, KeyCode::Tab);
    let field = app.world().resource::<bevy_input_focus::InputFocus>().get();
    assert_ne!(field, pane, "tab goes into the fields");
    assert_eq!(said(&app), heard);
}
