use std::path::PathBuf;

use retiretui_engine::plan::Dollars;

use super::*;
use crate::commands::tui::documents::{self, Opening};
use crate::commands::tui::settings::Settings;
use crate::commands::tui::support::{active_page, answer_back, headless_app_set, is_asking};

const SAVE: usize = 0;
const DISCARD: usize = 1;
const CANCEL: usize = 2;
const RICH: Dollars = 5_000_000;

fn document(app: &App) -> String {
    app.world().resource::<Session>().file_name().into_owned()
}

fn baseline(app: &App) -> Option<String> {
    let path = app.world().resource::<Compared>().baseline.clone();
    path.map(|path| session::file_name(&path).into_owned())
}

/// The first account's balance in the compared plan named `name`.
fn balance_of(app: &App, name: &str) -> Dollars {
    let compared = app.world().resource::<Compared>();
    let doc = compared.docs.iter().find(|doc| doc.path.ends_with(name));
    doc.unwrap().projected.plan.accounts[0].balance
}

/// Opens the second row, the variant, with ⏎.
fn open_variant(app: &mut App) {
    press_key(app, KeyCode::Down);
    press_key(app, KeyCode::Enter);
    app.update();
}

#[test]
fn enter_opens_a_compared_plan_and_the_document_takes_its_place() {
    let mut app = comparing_variant(SIZE);
    let workspace = app.world().resource::<Session>().workspace().to_path_buf();
    std::fs::write(workspace.join("other.toml"), test_plan_briefly_run()).unwrap();
    compare_with(&mut app, "other");
    open_variant(&mut app);
    assert_eq!(document(&app), "variant.toml", "{:?}", said(&app));
    assert_eq!(compared(&app), ["plan.toml", "other.toml"]);
    assert_eq!(
        active_page(&app),
        Page::Compare,
        "the swap stays on Compare"
    );
}

#[test]
fn enter_on_the_document_says_it_is_open() {
    let mut app = comparing_variant(SIZE);
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(document(&app), "plan.toml");
    assert_eq!(compared(&app), ["variant.toml"]);
    assert!(said(&app).contains(&"plan.toml is open already".to_owned()));
}

/// The variant is the baseline, under difference, and the draft holds
/// an edit no one saved; ⏎ on the variant asks about it.
fn asking_over_a_dirty_draft() -> crate::commands::tui::support::Headless {
    let mut app = comparing_variant(SIZE);
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('b'));
    press_key(&mut app, KeyCode::Char('d'));
    commit_edit(&mut app, |plan| plan.accounts[0].balance = RICH);
    press_key(&mut app, KeyCode::Enter);
    assert!(is_asking(&app));
    app
}

#[test]
fn cancel_leaves_the_compared_set_as_it_was() {
    let mut app = asking_over_a_dirty_draft();
    answer_back(&mut app, CANCEL);
    app.update();
    assert_eq!(document(&app), "plan.toml");
    assert_eq!(compared(&app), ["variant.toml"]);
    assert_eq!(baseline(&app).as_deref(), Some("variant.toml"));
    assert!(app.world().resource::<Compared>().is_difference);
}

#[test]
fn discard_compares_the_old_document_as_it_is_on_disk() {
    let mut app = asking_over_a_dirty_draft();
    answer_back(&mut app, DISCARD);
    app.update();
    assert_eq!(document(&app), "variant.toml");
    assert_eq!(compared(&app), ["plan.toml"]);
    assert_ne!(balance_of(&app, "plan.toml"), RICH, "the draft is gone");
    assert_eq!(baseline(&app), None, "the opened baseline is the document");
    assert!(app.world().resource::<Compared>().is_difference);
}

#[test]
fn save_compares_the_old_document_as_saved() {
    let mut app = asking_over_a_dirty_draft();
    answer_back(&mut app, SAVE);
    app.update();
    assert_eq!(document(&app), "variant.toml");
    assert_eq!(compared(&app), ["plan.toml"]);
    assert_eq!(balance_of(&app, "plan.toml"), RICH);
}

#[test]
fn the_documents_baseline_stays_with_its_row() {
    let mut app = comparing_variant(SIZE);
    open_variant(&mut app);
    assert_eq!(baseline(&app).as_deref(), Some("plan.toml"));
}

#[test]
fn a_document_without_a_file_drops_out() {
    let dir = scratch_workspace(&test_plan_briefly_run());
    let mut app = headless_app_set(dir.clone(), SIZE, Settings::still());
    let tables = app.world().resource::<Session>().tables.clone();
    std::fs::write(dir.join("other.toml"), test_plan_briefly_run()).unwrap();
    let variant: PathBuf = dir.join("variant.toml");
    let mut held = app.world_mut().resource_mut::<Compared>();
    held.take_in(variant.clone(), &tables).unwrap();
    held.take_in(dir.join("other.toml"), &tables).unwrap();
    app.world_mut()
        .run_system_cached_with(documents::open, Opening::swapping(variant))
        .unwrap();
    app.update();
    assert_eq!(document(&app), "variant.toml", "{:?}", said(&app));
    assert_eq!(compared(&app), ["other.toml"]);
}
