//! Compared files as the disk has them: each read, re-read when it
//! changes, and one opened in the document's place.

use std::path::{Path, PathBuf};

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::{Commands, Res, ResMut};
use retiretui_engine::params::TaxTables;

use super::{Compared, plans};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::documents::{self, Opening};
use crate::commands::tui::journal;
use crate::commands::tui::session::{self, Projected, Session};
use crate::commands::tui::watch::{self, Stamped};

pub(super) struct ComparedDoc {
    pub(super) path: PathBuf,
    pub(super) projected: Projected,
    files: Stamped,
    /// Why the file last failed to re-read, its figures kept from the read
    /// before.
    pub(super) failure: Option<String>,
}

impl ComparedDoc {
    pub(super) fn read(path: PathBuf, tables: &TaxTables) -> Result<Self, String> {
        let (projected, files) = watch::load_projected(&path, tables)
            .map_err(|invalid| invalid.headline().to_owned())?;
        Ok(Self {
            path,
            projected,
            files,
            failure: None,
        })
    }

    /// Reads the file again; one that fails keeps its figures, says so,
    /// and marks its row until it reads again.
    fn reread(&mut self, tables: &TaxTables) {
        match watch::load_projected(&self.path, tables) {
            Ok((projected, files)) => {
                self.projected = projected;
                self.files = files;
                self.failure = None;
            }
            Err(invalid) => {
                watch::restamp(&mut self.files);
                let (name, reason) = (session::file_name(&self.path), invalid.reason());
                journal::warn(format!("{name} not re-read: {reason}"));
                self.failure = Some(reason.to_owned());
            }
        }
    }
}

impl Compared {
    /// Re-reads every compared file.
    pub fn reload(&mut self, tables: &TaxTables) {
        for doc in &mut self.docs {
            doc.reread(tables);
        }
    }

    /// Re-reads each compared file that changed on disk, answering
    /// whether any had.
    fn reread_stale(&mut self, tables: &TaxTables) -> bool {
        let mut is_reread = false;
        let stale = self.docs.iter_mut();
        for doc in stale.filter(|doc| watch::is_stale(&doc.files)) {
            doc.reread(tables);
            is_reread = true;
        }
        is_reread
    }

    /// The set once `opened`, one of its plans, is the document: `left`,
    /// the document it replaces, takes its place as saved on disk - where
    /// it is saved at all - and the baseline follows its row.
    #[must_use]
    pub fn swapped(mut self, opened: &Path, left: Option<PathBuf>, tables: &TaxTables) -> Self {
        let Some(at) = self.docs.iter().position(|doc| doc.path == opened) else {
            return self;
        };
        self.docs.remove(at);
        let is_opened_baseline = self.baseline.as_deref() == Some(opened);
        let joined = left.and_then(|path| {
            let name = session::file_name(&path).into_owned();
            ComparedDoc::read(path, tables)
                .inspect_err(|reason| journal::warn(format!("{name} not compared: {reason}")))
                .ok()
        });
        self.baseline = match (&joined, self.baseline.take()) {
            (_, Some(_)) if is_opened_baseline => None,
            (Some(joined), None) => Some(joined.path.clone()),
            (_, kept) => kept,
        };
        if let Some(joined) = joined {
            self.docs.insert(at, joined);
        }
        self
    }

    /// The first file that failed to re-read, and why.
    pub(super) fn failure(&self) -> Option<String> {
        self.docs.iter().find_map(|doc| {
            let reason = doc.failure.as_ref()?;
            Some(format!(
                "{} not re-read: {reason}",
                session::file_name(&doc.path)
            ))
        })
    }
}

/// Re-reads the compared files that changed on disk, on the watch's beat;
/// the set changes only where one had.
pub(super) fn follow_disk(session: Res<Session>, mut compared: ResMut<Compared>) {
    if compared
        .bypass_change_detection()
        .reread_stale(&session.tables)
    {
        compared.set_changed();
    }
}

/// Opens the highlighted plan as the document, the one it replaces
/// joining the compared plans in its place.
pub(crate) fn open_highlighted(
    cursor: plans::Cursor,
    session: Res<Session>,
    compared: Res<Compared>,
    mut commands: Commands,
) -> Outcome {
    let at = cursor.place().checked_sub(1);
    let Some(doc) = at.and_then(|at| compared.docs.get(at)) else {
        return Outcome::Refused(format!("{} is open already", session.file_name()));
    };
    let opening = Opening::swapping(doc.path.clone());
    commands.run_system_cached_with(documents::open, opening);
    Outcome::Done
}
