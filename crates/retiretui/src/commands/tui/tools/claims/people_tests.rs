use std::path::PathBuf;

use bevy_app::App;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::system::IntoSystem;
use bevy_input_focus::InputFocus;
use plurimus::term::KeyCode;
use retiretui_engine::optimize::benefit_estimates;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use super::*;
use crate::commands::tui::edit::Importing;
use crate::commands::tui::pane::Framed;
use crate::commands::tui::support::{
    Headless, SIZE, composed_frame, headless_app_at, press_ctrl, press_key, press_shift, redrawn,
    said, scratch_workspace, show,
};

pub(super) const BENEFIT: &str = "[[income]]\nid = \"ss\"\nkind = \"social-security\"\nowner = \"me\"\nstart = { age = 67, owner = \"me\" }\n";
pub(super) const SALARY: &str = "[[income]]\nid = \"pay\"\nkind = \"salary\"\nowner = \"me\"\namount = 90000\nend = { age = 62, owner = \"me\" }\n";

/// `plan` with its earnings record taken out.
pub(super) fn without_record(plan: &str) -> String {
    plan.lines()
        .filter(|line| !line.starts_with("earnings"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn fixture() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claims-plan.toml");
    std::fs::read_to_string(path).unwrap()
}

fn unclaimed() -> String {
    let plan = fixture().replace(BENEFIT, "");
    assert!(
        !plan.contains("social-security"),
        "the fixture moved: {plan}"
    );
    plan
}

pub(super) fn app_on(plan: &str) -> Headless {
    let dir = scratch_workspace(plan);
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::SsaBenefits);
    super::super::settle::<ClaimSearch>(&mut app);
    app
}

pub(super) fn plan(app: &App) -> &Plan {
    &app.world().resource::<Draft>().plan
}

/// Runs `command` and waits out the search the change starts.
pub(super) fn run<M>(app: &mut App, command: impl IntoSystem<(), Outcome, M> + 'static) -> Outcome {
    let outcome = app.world_mut().run_system_cached(command).unwrap();
    super::super::settle::<ClaimSearch>(app);
    outcome
}

/// The title of the pane holding the keyboard, or of the pane it is in.
fn holder_title(app: &App) -> String {
    let world = app.world();
    let mut at = world.resource::<InputFocus>().get();
    while let Some(entity) = at {
        if let Some(framed) = world.get::<Framed>(entity) {
            return framed.title.clone();
        }
        at = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    panic!("nothing holds the keyboard");
}

/// The cells of a People row, from its cursor mark on.
fn person_cells(line: &str) -> Vec<String> {
    let pane = line.split("││").nth(1).unwrap_or(line);
    let cells: Vec<String> = pane
        .replace('│', " ")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let from = cells
        .iter()
        .position(|cell| cell == "▌" || cell == "▏")
        .unwrap_or(0);
    cells[from..].to_vec()
}

#[test]
fn the_pane_lists_each_person_s_record_income_and_estimates() {
    let app = app_on(&fixture());
    let tables = TaxTables::embedded();
    let estimates = benefit_estimates(plan(&app), &tables, "me")
        .map(|figure| crate::commands::tui::present::compact_money(figure.unwrap()));
    let frame = composed_frame(&app);
    let row = frame
        .lines()
        .find(|line| line.contains(" 6y "))
        .unwrap_or_else(|| panic!("no row for me: {frame}"));
    let cells = person_cells(row);
    assert_eq!(cells[..4], ["▌", "me", "6y", "computed"], "{row}");
    assert_eq!(cells[4..7], estimates, "{row}");
}

#[test]
fn the_page_opens_on_the_people_and_tab_walks_to_the_options() {
    let mut app = app_on(&fixture());
    assert_eq!(holder_title(&app), "People");
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(holder_title(&app), "Claim Options");
    press_shift(&mut app, KeyCode::Tab);
    assert_eq!(holder_title(&app), "People");
}

#[test]
fn arrows_on_the_pane_move_the_person_the_keys_act_on() {
    let couple = format!(
        "{}\n[[household.people]]\nid = \"you\"\nbirth = 1962-01-01\n",
        fixture().replace("\"single\"", "\"married-joint\"")
    );
    let mut app = app_on(&couple);
    press_key(&mut app, KeyCode::Down);
    let frame = redrawn(&mut app);
    let is_marked =
        |line: &str| person_cells(line).starts_with(&["▌".to_owned(), "you".to_owned()]);
    assert!(frame.lines().any(is_marked), "{frame}");
    assert_eq!(run(&mut app, import_statement), Outcome::Done);
    assert_eq!(app.world().resource::<Importing>().asked_for(), Some("you"));
    assert!(app.world().resource::<Browsing>().is_open());
}

#[test]
fn k_computes_a_typed_benefit_as_one_step() {
    let typed = fixture().replace(
        "owner = \"me\"\nstart",
        "owner = \"me\"\namount = 24000\nstart",
    );
    let mut app = app_on(&typed);
    let frame = composed_frame(&app);
    assert!(frame.contains(" typed "), "{frame}");
    assert!(
        frame.contains("typed at $2000 a month"),
        "the help line says the figure: {frame}"
    );
    assert_eq!(run(&mut app, compute_benefit), Outcome::Done);
    assert!(plan(&app).income[0].is_derived());
    assert_eq!(
        run(&mut app, compute_benefit),
        Outcome::Refused("me has no typed benefit to compute".to_owned())
    );
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(plan(&app).income[0].amount, Some(24000));
}

#[test]
fn c_fills_a_career_only_where_there_is_no_record() {
    assert_eq!(
        run(&mut app_on(&fixture()), fill_career),
        Outcome::Refused("me has an earnings record; a statement replaces it".to_owned())
    );
    let no_record = without_record(&fixture());
    let mut unpaid = app_on(&no_record);
    let Outcome::Refused(reason) = run(&mut unpaid, fill_career) else {
        panic!("a career needs a salary");
    };
    assert!(reason.contains("import a statement"), "{reason}");

    let mut app = app_on(&no_record.replace("[[expenses]]", &format!("{SALARY}\n[[expenses]]")));
    assert_eq!(run(&mut app, fill_career), Outcome::Done);
    let years = plan(&app).person("me").unwrap().earnings.len();
    assert_eq!(years, 40, "from 22 in 1986 through 2025");
    assert!(
        said(&app)
            .last()
            .unwrap()
            .contains("from a career at their salary")
    );
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert!(plan(&app).person("me").unwrap().earnings.is_empty());
}

#[test]
fn t_adds_the_benefit_the_search_made_up_with_its_claim() {
    let mut app = app_on(&unclaimed());
    assert!(composed_frame(&app).contains("none"));
    assert_eq!(run(&mut app, adopt), Outcome::Done);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let added = &plan(&app).income[0];
    assert_eq!(added.id, "ss-me");
    assert!(
        added.is_derived()
            && added
                .start
                .as_ref()
                .is_some_and(|start| start.age.is_some())
    );
    assert!(app.world().resource::<Draft>().issues().is_empty());
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert!(
        plan(&app).income.is_empty(),
        "one step back is the plan's own"
    );
}

#[test]
fn a_named_person_is_shown_by_name_and_kept_by_id() {
    let named = fixture().replace("id = \"me\"\n", "id = \"me\"\nname = \"Pat Lee\"\n");
    let mut app = app_on(&named);
    let frame = composed_frame(&app);
    let row = frame
        .lines()
        .find(|line| line.contains(" 6y "))
        .unwrap_or_else(|| panic!("no row for Pat: {frame}"));
    assert_eq!(person_cells(row)[..3], ["▌", "Pat", "Lee"], "{row}");
    let header = frame
        .lines()
        .find(|line| line.contains(" unfunded "))
        .unwrap();
    assert!(
        header.contains("Pat Lee"),
        "the options name the person: {header}"
    );
    assert!(frame.contains("act on Pat Lee."), "{frame}");
    assert_eq!(run(&mut app, remove_benefit), Outcome::Done);
    assert!(redrawn(&mut app).contains("Remove Pat Lee's Social Security income?"));
}
