//! Headless tests for opening a document over the one the shell holds.

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy_app::App;
use bevy_ecs::prelude::With;
use plurimus::term::KeyCode;
use plurimus::ui::UiLabel;
use plurimus::widgets::{ListItem, ListItemTrailing};

use crate::commands::tui::documents;
use crate::commands::tui::edit::Draft;
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::session::{Projected, Session, YearCursor};
use crate::commands::tui::support::{
    SIZE, TEST_PLAN, answer_back, commit_edit as commit, composed_frame, headless_app,
    headless_app_at, is_asking, is_browsing, let_pass, press_ctrl, press_key, said, scenario_over,
    scratch_full_plan, scratch_workspace, show, type_text,
};

fn open(app: &mut App, path: &Path) {
    app.world_mut()
        .run_system_cached_with(documents::open, path.to_path_buf().into())
        .unwrap();
    app.update();
    app.update();
}

fn document(app: &App) -> PathBuf {
    app.world().resource::<Session>().plan_path.clone().unwrap()
}

fn draft(app: &App) -> &Draft {
    app.world().resource::<Draft>()
}

fn answer(app: &mut App, back: usize) {
    answer_back(app, back);
    app.update();
}

/// Longer than the coarse clock file times are stamped from, so a write
/// straight after a save gets a later mtime.
const MTIME_TICK: Duration = Duration::from_millis(50);

const SAVE: usize = 0;
const DISCARD: usize = 1;
const CANCEL: usize = 2;

#[test]
fn a_clean_switch_reseeds_every_page() {
    let mut app = headless_app(SIZE);
    let before = document(&app);
    commit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    press_ctrl(&mut app, KeyCode::Char('s'));
    *app.world_mut().resource_mut::<YearCursor>() = YearCursor(Some(2030));
    show(&mut app, Page::Accounts);
    let full = scratch_full_plan();
    open(&mut app, &full);
    assert!(!is_asking(&app), "a clean draft asks nothing");
    assert_eq!(document(&app), full);
    assert_eq!(app.world().resource::<ActivePage>().page(), Page::Overview);
    assert_eq!(*app.world().resource::<YearCursor>(), YearCursor::default());
    assert!(!draft(&app).is_dirty());
    let projected = app.world().resource::<Projected>();
    assert_eq!(
        projected.plan.plan.name.as_deref(),
        Some("base"),
        "the new plan projected"
    );
    show(&mut app, Page::Accounts);
    assert!(composed_frame(&app).contains("fid-401k"), "rows rebuilt");
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(said(&app).last().unwrap(), "nothing to undo");
    assert!(std::fs::read_to_string(before).unwrap().contains("renamed"));
}

#[test]
fn a_dirty_draft_asks_and_cancel_keeps_it() {
    let mut app = headless_app(SIZE);
    let before = document(&app);
    commit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    let full = scratch_full_plan();
    open(&mut app, &full);
    assert!(is_asking(&app));
    let frame = composed_frame(&app);
    assert!(frame.contains("Save the changes to"), "{frame}");
    answer(&mut app, CANCEL);
    assert!(!is_asking(&app));
    assert_eq!(document(&app), before);
    assert!(draft(&app).is_dirty(), "and the edit is still there");
}

#[test]
fn discard_switches_without_writing() {
    let mut app = headless_app(SIZE);
    let before = document(&app);
    commit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    let full = scratch_full_plan();
    open(&mut app, &full);
    answer(&mut app, DISCARD);
    assert_eq!(document(&app), full);
    assert!(!std::fs::read_to_string(before).unwrap().contains("renamed"));
}

#[test]
fn save_writes_then_switches_and_a_refused_save_stops_the_switch() {
    let mut app = headless_app(SIZE);
    let before = document(&app);
    commit(&mut app, |plan| plan.plan.inflation = 9.0);
    let full = scratch_full_plan();
    open(&mut app, &full);
    answer(&mut app, SAVE);
    assert!(said(&app).last().unwrap().starts_with("not saved"));
    assert_eq!(document(&app), before, "the refusal stops the switch");
    assert!(draft(&app).is_dirty());

    commit(&mut app, |plan| {
        plan.plan.inflation = 0.03;
        plan.plan.name = Some("renamed".to_owned());
    });
    open(&mut app, &full);
    answer(&mut app, SAVE);
    assert_eq!(document(&app), full);
    assert!(std::fs::read_to_string(before).unwrap().contains("renamed"));
}

#[test]
fn a_scenario_opens_read_only_and_a_failed_load_changes_nothing() {
    let mut app = headless_app(SIZE);
    let base = document(&app);
    let scenario = base.with_extension("scenario.toml");
    let overlay = format!(
        "schema = 1\nbase = \"{}\"\n\n[plan]\nname = \"variant\"\n",
        base.file_name().unwrap().to_string_lossy()
    );
    std::fs::write(&scenario, overlay).unwrap();
    open(&mut app, &scenario);
    assert_eq!(document(&app), scenario);
    assert!(draft(&app).refuse_if_read_only().is_some());
    let projected = app.world().resource::<Projected>();
    assert_eq!(projected.plan.plan.name.as_deref(), Some("variant"));

    let missing = base.with_extension("missing.toml");
    open(&mut app, &missing);
    assert_eq!(document(&app), scenario);
    assert!(
        said(&app).last().unwrap().starts_with("not opened"),
        "{:?}",
        said(&app)
    );
}

fn workspace() -> PathBuf {
    scratch_workspace(TEST_PLAN)
}

fn scenario() -> String {
    scenario_over("plan.toml")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

/// Runs `save-as` from the palette, which has no key.
fn save_as(app: &mut App) {
    press_key(app, KeyCode::Char(':'));
    type_text(app, "save-as");
    press_key(app, KeyCode::Enter);
    app.update();
}

#[test]
fn a_directory_launch_offers_the_workspace_and_choosing_opens_a_file() {
    let dir = workspace();
    let mut app = headless_app_at(dir.clone(), SIZE);
    assert!(is_browsing(&app));
    app.update();
    for (row, badge) in [
        ("plan.toml", "plan"),
        ("variant.toml", "scenario"),
        ("broken.toml", "invalid"),
    ] {
        assert_eq!(badge_of(&mut app, row), badge);
    }
    let frame = composed_frame(&app);
    assert!(!frame.contains("notes.txt"), "{frame}");
    type_text(&mut app, "brok");
    press_key(&mut app, KeyCode::Enter);
    assert!(!is_browsing(&app));
    assert!(
        said(&app).last().unwrap().starts_with("not opened"),
        "{:?}",
        said(&app)
    );
    assert!(app.world().resource::<Session>().plan_path.is_none());
    press_ctrl(&mut app, KeyCode::Char('o'));
    assert!(is_browsing(&app), "the open command asks again");
    type_text(&mut app, "plan");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert_eq!(document(&app), dir.join("plan.toml"));
    assert!(
        composed_frame(&app).contains("Balances by tax treatment"),
        "pages drawn"
    );
}

#[test]
fn the_picker_climbs_out_of_the_workspace_and_the_next_opens_beside_the_document() {
    let dir = workspace();
    let beside = dir.join("beside");
    std::fs::create_dir(&beside).unwrap();
    std::fs::write(beside.join("other.toml"), TEST_PLAN).unwrap();
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    press_ctrl(&mut app, KeyCode::Char('o'));
    type_text(&mut app, "beside/oth");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert_eq!(document(&app), beside.join("other.toml"));
    press_ctrl(&mut app, KeyCode::Char('o'));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("other.toml"), "listed beside it: {frame}");
    assert!(!frame.contains("variant.toml"), "{frame}");
    type_text(&mut app, "../pl");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert_eq!(document(&app), dir.join("plan.toml"));
}

#[test]
fn compare_with_refuses_the_document_and_takes_another() {
    let dir = workspace();
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::Compare);
    press_key(&mut app, KeyCode::Char('c'));
    assert!(is_browsing(&app));
    type_text(&mut app, "plan");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(said(&app).last().unwrap(), "plan.toml is the document");
    press_key(&mut app, KeyCode::Char('c'));
    type_text(&mut app, "variant");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert_eq!(said(&app).last().unwrap(), "compared variant.toml");
    press_key(&mut app, KeyCode::Char('c'));
    app.update();
    assert_eq!(badge_of(&mut app, "variant.toml"), "compared");
    assert_eq!(badge_of(&mut app, "plan.toml"), "plan");
}

/// What the picker's row naming `file` says at its right.
fn badge_of(app: &mut App, file: &str) -> String {
    let mut rows = app
        .world_mut()
        .query_filtered::<(&UiLabel, &ListItemTrailing), With<ListItem>>();
    rows.iter(app.world())
        .find(|(label, _)| label.0.to_string() == file)
        .map(|(_, trailing)| trailing.0.to_string())
        .unwrap_or_default()
}

#[test]
fn new_composes_a_plan_over_the_document_without_closing_it() {
    let dir = workspace();
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    commit(&mut app, |plan| plan.plan.horizon_age = 80);
    press_ctrl(&mut app, KeyCode::Char('n'));
    app.update();
    let frame = composed_frame(&app);
    assert!(frame.contains("New plan"), "the form is shown: {frame}");
    assert!(
        !frame.contains("Save the changes"),
        "nothing is left yet, so nothing is asked: {frame}"
    );
    assert_eq!(document(&app), dir.join("plan.toml"), "still the document");
    assert_eq!(read(&dir.join("plan.toml")), TEST_PLAN, "nothing written");
}

#[test]
fn save_as_writes_the_draft_under_a_new_name_and_keeps_the_history() {
    let dir = workspace();
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    commit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    save_as(&mut app);
    assert!(composed_frame(&app).contains("Save as"));
    type_text(&mut app, "copy");
    press_key(&mut app, KeyCode::Enter);
    let copy = dir.join("copy.toml");
    assert!(read(&copy).contains("renamed"));
    assert!(!read(&dir.join("plan.toml")).contains("renamed"));
    assert_eq!(document(&app), copy);
    assert!(!draft(&app).is_dirty());
    press_ctrl(&mut app, KeyCode::Char('z'));
    assert_eq!(draft(&app).plan.plan.name.as_deref(), Some("test-plan"));
    assert!(draft(&app).is_dirty(), "undone past the save");

    save_as(&mut app);
    type_text(&mut app, "again.toml");
    press_key(&mut app, KeyCode::Enter);
    let again = dir.join("again.toml");
    assert!(again.exists(), "not appended twice");
    assert_eq!(document(&app), again);
    let on_disk = read(&again).replace("test-plan", "ondisk");
    std::thread::sleep(MTIME_TICK);
    std::fs::write(&again, on_disk).unwrap();
    let_pass(&mut app, Duration::from_secs(2));
    assert_eq!(
        draft(&app).plan.plan.name.as_deref(),
        Some("ondisk"),
        "the new file is the one watched"
    );
}

#[test]
fn save_as_over_a_file_asks_first() {
    let dir = workspace();
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    commit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    save_as(&mut app);
    type_text(&mut app, "variant");
    press_key(&mut app, KeyCode::Enter);
    let frame = composed_frame(&app);
    assert!(frame.contains("Overwrite variant.toml?"), "{frame}");
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(read(&dir.join("variant.toml")), scenario(), "esc leaves it");
    save_as(&mut app);
    type_text(&mut app, "variant");
    press_key(&mut app, KeyCode::Enter);
    press_key(&mut app, KeyCode::Enter);
    assert!(read(&dir.join("variant.toml")).contains("renamed"));
    assert_eq!(document(&app), dir.join("variant.toml"));
}

#[test]
fn save_as_of_a_scenario_writes_the_resolved_plan_and_opens_it_editable() {
    let dir = workspace();
    let mut app = headless_app_at(dir.join("variant.toml"), SIZE);
    assert!(draft(&app).refuse_if_read_only().is_some());
    save_as(&mut app);
    type_text(&mut app, "flat");
    press_key(&mut app, KeyCode::Enter);
    let flat = read(&dir.join("flat.toml"));
    assert!(flat.contains("name = \"variant\""), "{flat}");
    assert!(!flat.contains("base"), "resolved, not an overlay: {flat}");
    assert_eq!(document(&app), dir.join("flat.toml"));
    assert!(draft(&app).refuse_if_read_only().is_none(), "editable now");

    commit(&mut app, |plan| plan.plan.inflation = 9.0);
    save_as(&mut app);
    assert!(
        !is_browsing(&app),
        "an invalid draft is refused before asking"
    );
    assert!(said(&app).last().unwrap().starts_with("not saved"));
}
