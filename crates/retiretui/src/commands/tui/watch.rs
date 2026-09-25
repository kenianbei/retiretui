use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy_app::{App, Update};
use bevy_ecs::prelude::{Res, ResMut, Resource};
use bevy_time::{Real, Time, Timer, TimerMode};
use retiretui_engine::params::TaxTables;
use retiretui_engine::project::project;

use crate::commands::Invalid;

use super::edit::{DraftEditor, EditSession};
use super::journal;
use super::session::{Projected, Session, Today};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, poll_watch);
}

pub const POLL_SECONDS: f32 = 1.0;

/// The resolved chain's files and their mtimes as last loaded.
#[derive(Resource)]
pub struct Watch {
    files: Vec<(PathBuf, Option<SystemTime>)>,
    timer: Timer,
}

impl Watch {
    pub fn new(files: Vec<(PathBuf, Option<SystemTime>)>) -> Self {
        Self {
            files,
            timer: Timer::from_seconds(POLL_SECONDS, TimerMode::Repeating),
        }
    }

    /// A plan file alone, as after a save under a new name.
    pub fn at(path: PathBuf) -> Self {
        Self::new(stamp(vec![path]))
    }

    /// Whether the session opened a scenario: the chain has a base beneath
    /// the given file.
    pub fn is_scenario(&self) -> bool {
        self.files.len() > 1
    }

    /// Records the files' current mtimes as the known state.
    pub fn restamp(&mut self) {
        restamp(&mut self.files);
    }
}

/// Records `files`' current mtimes as the known state.
pub fn restamp(files: &mut Stamped) {
    for (path, recorded) in files {
        *recorded = mtime(path);
    }
}

/// The resolved chain's files, each with its mtime as last seen.
pub type Stamped = Vec<(PathBuf, Option<SystemTime>)>;

/// What the session shows at launch: its document projected, or the blank
/// plan behind an empty shell that watches nothing.
pub fn load_session(session: &Session, today: Today) -> Result<(Projected, Stamped), String> {
    match &session.plan_path {
        Some(path) => {
            load_projected(path, &session.tables).map_err(|invalid| invalid.headline().to_owned())
        }
        None => Ok((Projected::blank(&session.tables, today), Vec::new())),
    }
}

/// Loads, validates, and projects the plan at `path`. Also returns the
/// stamped chain files to watch.
///
/// # Errors
///
/// Why the plan did not pass the gate: a reader says its headline, or its
/// reason where it names the file itself.
pub(crate) fn load_projected(
    path: &Path,
    tables: &TaxTables,
) -> Result<(Projected, Stamped), Invalid> {
    let (plan, files) = crate::commands::validated_plan_with_files(path, tables)?;
    let projection = project(&plan, tables);
    Ok((Projected { plan, projection }, stamp(files)))
}

fn stamp(files: Vec<PathBuf>) -> Stamped {
    files
        .into_iter()
        .map(|path| {
            let modified = mtime(&path);
            (path, modified)
        })
        .collect()
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

pub fn is_stale(files: &[(PathBuf, Option<SystemTime>)]) -> bool {
    files
        .iter()
        .any(|(path, recorded)| mtime(path) != *recorded)
}

/// Follows the document's chain on the watch's beat.
pub(crate) fn poll_watch(
    time: Res<Time<Real>>,
    mut watch: ResMut<Watch>,
    mut editor: DraftEditor,
    session: Res<EditSession>,
) {
    if watch.timer.tick(time.delta()).just_finished() && is_stale(&watch.files) {
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

/// A failed reload keeps the last good projection; the recorded mtimes are
/// refreshed either way so a broken save does not refire every poll.
pub fn apply_reload(
    session: &Session,
    projected: &mut Projected,
    watch: &mut Watch,
) -> Result<(), String> {
    match load_projected(session.document()?, &session.tables) {
        Ok((fresh, files)) => {
            *projected = fresh;
            watch.files = files;
            Ok(())
        }
        Err(invalid) => {
            watch.restamp();
            Err(format!("reload failed: {}", invalid.headline()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str, text: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("retiretui-watch-{name}.toml"));
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn staleness_follows_mtimes() {
        let path = scratch("stale", "schema = 1\n");
        let files = stamp(vec![path.clone()]);
        assert!(!is_stale(&files));
        let missing = vec![(PathBuf::from("no-such-file.toml"), mtime(&path))];
        assert!(is_stale(&missing), "a vanished file counts as stale");
    }

    #[test]
    fn scenario_chains_are_watched_whole() {
        let base = scratch("chain-base", super::super::support::TEST_PLAN);
        let overlay =
            super::super::support::scenario_over(&base.file_name().unwrap().to_string_lossy());
        let scenario = scratch("chain-overlay", &overlay);
        let (plan, files) = crate::commands::load_plan_with_files(&scenario).unwrap();
        assert_eq!(plan.plan.name.as_deref(), Some("variant"));
        assert_eq!(files.len(), 2, "{files:?}");
        assert_eq!(files[1], base.canonicalize().unwrap());
    }
}
