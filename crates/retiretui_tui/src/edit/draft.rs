//! The commands and systems over the draft: committing an edit, undo, redo
//! and save.

use bevy_ecs::prelude::{Res, ResMut};
use bevy_ecs::system::SystemParam;
pub use retiretui_client::draft::{Draft, write_draft};
use retiretui_client::issues::issue_words;
use retiretui_engine::project::project;

use crate::command::Outcome;
use crate::journal;
use crate::session::{Projected, Session};
use crate::watch::{self, Watch};

/// Everything a committed edit touches.
#[derive(SystemParam)]
pub struct DraftEditor<'w> {
    pub draft: ResMut<'w, Draft>,
    pub session: Res<'w, Session>,
    pub projected: ResMut<'w, Projected>,
}

impl DraftEditor<'_> {
    /// Re-reads the plan from disk, discarding the draft; a failure keeps
    /// both the draft and the last good view.
    pub fn reload(&mut self, watch: &mut Watch) -> Result<(), String> {
        watch::apply_reload(&self.session, &mut self.projected, watch)?;
        self.draft.reset(self.projected.plan.clone());
        journal::say("plan reloaded");
        Ok(())
    }

    /// Records an edit as one step of history, then validates the draft: a
    /// valid draft re-projects at once, an invalid one holds the last good
    /// view and reports its first issue.
    pub fn commit(&mut self) {
        self.draft.record();
        self.revalidate();
    }

    fn revalidate(&mut self) {
        let tables = &self.session.tables;
        let Some(issue) = self.draft.revalidate(tables).cloned() else {
            self.projected.projection = project(&self.draft.plan, tables);
            self.projected.plan = self.draft.plan.clone();
            return;
        };
        journal::warn(issue_words(&issue, &self.draft));
    }
}

pub fn undo(mut editor: DraftEditor) -> Outcome {
    if !editor.draft.undo() {
        return Outcome::Refused("nothing to undo".to_owned());
    }
    editor.revalidate();
    Outcome::Done
}

pub fn redo(mut editor: DraftEditor) -> Outcome {
    if !editor.draft.redo() {
        return Outcome::Refused("nothing to redo".to_owned());
    }
    editor.revalidate();
    Outcome::Done
}

pub fn save(mut draft: ResMut<Draft>, session: Res<Session>, mut watch: ResMut<Watch>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let path = match session.document() {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refused(refusal),
    };
    if let Err(refusal) = write_draft(session.store.as_ref(), &draft, path) {
        return Outcome::Refused(refusal);
    }
    watch.restamp();
    draft.saved();
    journal::say(format!("saved {}", path.display()));
    Outcome::Done
}
