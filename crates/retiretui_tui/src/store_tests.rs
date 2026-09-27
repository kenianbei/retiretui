//! The shell over a workspace kept as keys, as a browser keeps it: listed,
//! opened, saved, and followed as it is on disk.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy_ecs::prelude::With;
use plurimus::term::KeyCode;
use plurimus::ui::UiLabel;
use plurimus::widgets::{ListItem, ListItemTrailing};

use crate::session::{Projected, Session};
use crate::store::memory::Memory;
use crate::store::{KeyStore, Store};
use crate::support::{
    Headless, SIZE, TEST_PLAN, commit_edit, headless_app_over, is_browsing, let_pass, press_ctrl,
    press_key, scenario_over, type_text,
};
use crate::watch::POLL_SECONDS;

const WORKSPACE: &str = "/workspace";

/// A workspace holding a plan and a scenario over it, in a backend another
/// tab could share.
fn workspace(backend: &Memory) -> Arc<dyn Store> {
    let store = KeyStore::new(backend.clone());
    store.create_dir_all(Path::new(WORKSPACE)).unwrap();
    store.write(&file("plan.toml"), TEST_PLAN).unwrap();
    store
        .write(&file("variant.toml"), &scenario_over("plan.toml"))
        .unwrap();
    Arc::new(store)
}

fn file(name: &str) -> PathBuf {
    Path::new(WORKSPACE).join(name)
}

fn listed(app: &mut Headless) -> Vec<(String, String)> {
    let mut rows = app
        .world_mut()
        .query_filtered::<(&UiLabel, Option<&ListItemTrailing>), With<ListItem>>();
    rows.iter(app.world())
        .map(|(label, trailing)| {
            let badge = trailing.map(|trailing| trailing.0.to_string());
            (label.0.to_string(), badge.unwrap_or_default())
        })
        .collect()
}

fn plan_name(app: &Headless) -> Option<String> {
    app.world().resource::<Projected>().plan.plan.name.clone()
}

#[test]
fn the_workspace_lists_with_its_badges_and_a_scenario_opens_through_it() {
    let backend = Memory::default();
    let mut app = headless_app_over(workspace(&backend), PathBuf::from(WORKSPACE), SIZE);
    assert!(is_browsing(&app));
    app.update();
    let rows = listed(&mut app);
    for row in [("plan.toml", "plan"), ("variant.toml", "scenario")] {
        let row = (row.0.to_owned(), row.1.to_owned());
        assert!(rows.contains(&row), "{rows:?}");
    }
    type_text(&mut app, "variant");
    press_key(&mut app, KeyCode::Enter);
    app.update();
    let session = app.world().resource::<Session>();
    assert_eq!(
        session.plan_path.as_deref(),
        Some(file("variant.toml").as_path())
    );
    assert_eq!(plan_name(&app).as_deref(), Some("variant"));
}

#[test]
fn a_save_is_written_into_the_store() {
    let backend = Memory::default();
    let store = workspace(&backend);
    let mut app = headless_app_over(Arc::clone(&store), file("plan.toml"), SIZE);
    commit_edit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    press_ctrl(&mut app, KeyCode::Char('s'));
    let saved = store.read(&file("plan.toml")).unwrap();
    assert!(saved.contains("name = \"renamed\""), "{saved}");
}

#[test]
fn a_write_from_another_tab_is_followed() {
    let backend = Memory::default();
    let mut app = headless_app_over(workspace(&backend), file("plan.toml"), SIZE);
    let other_tab = KeyStore::new(backend);
    let renamed = TEST_PLAN.replace("name = \"test-plan\"", "name = \"elsewhere\"");
    other_tab.write(&file("plan.toml"), &renamed).unwrap();
    let_pass(&mut app, std::time::Duration::from_secs_f32(POLL_SECONDS));
    assert_eq!(plan_name(&app).as_deref(), Some("elsewhere"));
}
