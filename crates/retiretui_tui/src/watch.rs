use std::path::PathBuf;
use std::sync::Arc;

use bevy_app::{App, Update};
use bevy_ecs::prelude::{Res, ResMut, Resource};
use bevy_time::{Real, Time, Timer, TimerMode};

pub use retiretui_client::session::{Stamped, is_stale, load_projected, load_session, stamp};
use retiretui_client::store::Store;

use super::edit::{DraftEditor, EditSession};
use super::journal;
use super::session::{Projected, Session};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, poll_watch);
}

pub const POLL_SECONDS: f32 = 1.0;

/// The resolved chain's files and their stamps as last loaded.
#[derive(Resource)]
pub struct Watch {
    store: Arc<dyn Store>,
    files: Stamped,
    timer: Timer,
}

impl Watch {
    pub fn new(store: Arc<dyn Store>, files: Stamped) -> Self {
        Self {
            store,
            files,
            timer: Timer::from_seconds(POLL_SECONDS, TimerMode::Repeating),
        }
    }

    /// A plan file alone, as after a save under a new name.
    pub fn at(store: Arc<dyn Store>, path: PathBuf) -> Self {
        let files = stamp(store.as_ref(), vec![path]);
        Self::new(store, files)
    }

    /// Whether the session opened a scenario: the chain has a base beneath
    /// the given file.
    pub fn is_scenario(&self) -> bool {
        self.files.len() > 1
    }

    /// Records the files' current stamps as the known state.
    pub fn restamp(&mut self) {
        for (path, recorded) in &mut self.files {
            *recorded = self.store.stamp(path);
        }
    }
}

/// Follows the document's chain on the watch's beat.
pub(crate) fn poll_watch(
    time: Res<Time<Real>>,
    mut watch: ResMut<Watch>,
    mut editor: DraftEditor,
    session: Res<EditSession>,
) {
    if watch.timer.tick(time.delta()).just_finished()
        && is_stale(watch.store.as_ref(), &watch.files)
    {
        follow_document(&mut watch, &mut editor, &session);
    }
}

/// Whether the watch beat this frame, for what else follows the disk on it.
pub(crate) fn on_beat(watch: Res<Watch>) -> bool {
    watch.timer.just_finished()
}

/// Neither a dirty draft nor an item being edited is overwritten by a
/// disk change; the user decides with an explicit reload.
fn follow_document(watch: &mut Watch, editor: &mut DraftEditor, session: &EditSession) {
    if editor.draft.is_dirty() || session.is_dirty() {
        watch.restamp();
        journal::warn("plan changed on disk; r reloads and discards the draft");
        return;
    }
    if let Err(message) = editor.reload(watch) {
        journal::warn(message);
    }
}

/// A failed reload keeps the last good projection; the watch follows the
/// chain as read either way, so a broken save does not refire every poll.
pub fn apply_reload(
    session: &Session,
    projected: &mut Projected,
    watch: &mut Watch,
) -> Result<(), String> {
    let (loaded, files) =
        load_projected(session.store.as_ref(), session.document()?, &session.tables);
    watch.files = files;
    *projected = loaded.map_err(|invalid| format!("reload failed: {}", invalid.headline()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::DiskStore;

    fn scratch(name: &str, text: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("retiretui-watch-{name}.toml"));
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn staleness_follows_stamps() {
        let path = scratch("stale", "schema = 1\n");
        let files = stamp(&DiskStore, vec![path.clone()]);
        assert!(!is_stale(&DiskStore, &files));
        let missing = vec![(PathBuf::from("no-such-file.toml"), DiskStore.stamp(&path))];
        assert!(
            is_stale(&DiskStore, &missing),
            "a vanished file counts as stale"
        );
    }

    #[test]
    fn scenario_chains_are_watched_whole() {
        let base = scratch("chain-base", super::super::support::TEST_PLAN);
        let overlay =
            super::super::support::scenario_over(&base.file_name().unwrap().to_string_lossy());
        let scenario = scratch("chain-overlay", &overlay);
        let mut files = Vec::new();
        let plan = crate::files::load_plan_with_files(&DiskStore, &scenario, &mut files).unwrap();
        assert_eq!(plan.plan.name.as_deref(), Some("variant"));
        assert_eq!(files.len(), 2, "{files:?}");
        assert_eq!(files[1], base.canonicalize().unwrap());
    }
}
