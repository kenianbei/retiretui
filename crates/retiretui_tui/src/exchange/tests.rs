//! Files handed across an edge a test holds.

use std::sync::{Arc, Mutex};

use super::commands::{NO_EDGE, SAVE_FIRST, download, upload};
use super::{Edge, Exchange};
use crate::command::Outcome;
use crate::store::{DiskStore, Store as _};
use crate::support::{
    SIZE, TEST_PLAN, answer_back, commit_edit, headless_app_at, is_asking, said, scratch_workspace,
};

/// An edge that keeps what it was handed and hands in what a test puts
/// on it.
#[derive(Debug, Default)]
struct Kept {
    handed: Mutex<Vec<(String, String)>>,
    arriving: Mutex<Vec<(String, String)>>,
}

impl Exchange for Kept {
    fn ask(&self) {}

    fn hand_out(&self, name: &str, text: &str) {
        self.handed
            .lock()
            .unwrap()
            .push((name.to_owned(), text.to_owned()));
    }

    fn arrived(&self) -> Vec<(String, String)> {
        std::mem::take(&mut *self.arriving.lock().unwrap())
    }
}

fn app_with_edge() -> (crate::support::Headless, Arc<Kept>, std::path::PathBuf) {
    let dir = scratch_workspace(TEST_PLAN);
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    let kept = Arc::new(Kept::default());
    app.insert_resource(Edge(Arc::clone(&kept) as Arc<dyn Exchange>));
    (app, kept, dir)
}

fn arrive(kept: &Kept, name: &str, text: &str) {
    kept.arriving
        .lock()
        .unwrap()
        .push((name.to_owned(), text.to_owned()));
}

#[test]
fn an_arrival_lands_in_the_workspace_and_a_taken_name_asks_first() {
    let (mut app, kept, dir) = app_with_edge();
    arrive(&kept, "new.toml", "one");
    app.update();
    app.update();
    assert_eq!(DiskStore.read(&dir.join("new.toml")).unwrap(), "one");
    assert_eq!(said(&app).last().unwrap(), "uploaded new.toml");
    arrive(&kept, "new.toml", "two");
    app.update();
    assert!(is_asking(&app));
    assert_eq!(DiskStore.read(&dir.join("new.toml")).unwrap(), "one");
    answer_back(&mut app, 0);
    app.update();
    app.update();
    assert_eq!(DiskStore.read(&dir.join("new.toml")).unwrap(), "two");
}

#[test]
fn a_download_hands_out_the_file_as_saved_and_never_a_draft() {
    let (mut app, kept, dir) = app_with_edge();
    let done = app.world_mut().run_system_cached(download).unwrap();
    assert!(matches!(done, Outcome::Done));
    let handed = kept.handed.lock().unwrap().clone();
    let saved = DiskStore.read(&dir.join("plan.toml")).unwrap();
    assert_eq!(handed, [("plan.toml".to_owned(), saved)]);
    commit_edit(&mut app, |plan| plan.plan.name = Some("renamed".to_owned()));
    let refused = app.world_mut().run_system_cached(download).unwrap();
    assert!(matches!(refused, Outcome::Refused(reason) if reason == SAVE_FIRST));
}

#[test]
fn without_an_edge_nothing_is_handed_across() {
    let mut app = headless_app_at(scratch_workspace(TEST_PLAN).join("plan.toml"), SIZE);
    let refused = app.world_mut().run_system_cached(upload).unwrap();
    assert!(matches!(refused, Outcome::Refused(reason) if reason == NO_EDGE));
}
