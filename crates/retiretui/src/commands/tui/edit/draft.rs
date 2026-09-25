use std::collections::VecDeque;
use std::mem;
use std::path::Path;

use bevy_ecs::prelude::{Res, ResMut, Resource};
use bevy_ecs::system::SystemParam;
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::{project, validate_plan};
use toml::{Table, Value};

use super::domain::ToolAnswers;

use crate::commands::tui::command::Outcome;
use crate::commands::tui::journal;
use crate::commands::tui::session::{Projected, Session};
use crate::commands::tui::watch::{self, Watch};

/// The working copy every edit lands in. A valid draft is what the views
/// project; an invalid one keeps the last good projection and its issues.
#[derive(Resource)]
pub struct Draft {
    pub plan: Plan,
    /// What each tool's form holds beside the plan, under the tool's own
    /// name; dropped with the document and kept across a reload.
    pub tools: Table,
    /// The plan as of the last commit, undo, redo, or reset: what the next
    /// commit's undo goes back to.
    committed: Plan,
    undone: VecDeque<Plan>,
    redone: Vec<Plan>,
    is_dirty: bool,
    is_read_only: bool,
    issues: Vec<Issue>,
}

pub const READ_ONLY_REASON: &str = "scenario sessions are read-only";

/// How many applied items undo reaches back over.
const HISTORY_DEPTH: usize = 100;

impl Draft {
    /// The answers tool `T`'s form holds, blank until it has been applied.
    pub fn answers<T: ToolAnswers>(&self) -> Table {
        let held = self.tools.get(T::SLOT).and_then(Value::as_table);
        held.cloned().unwrap_or_default()
    }

    /// A scenario session is read-only: the resolved plan cannot be written
    /// back into an overlay.
    pub fn new(plan: Plan, is_read_only: bool) -> Self {
        Self {
            committed: plan.clone(),
            plan,
            tools: Table::new(),
            undone: VecDeque::new(),
            redone: Vec::new(),
            is_dirty: false,
            is_read_only,
            issues: Vec::new(),
        }
    }

    /// Starts over from `plan`, as after a reload.
    pub fn reset(&mut self, plan: Plan) {
        self.committed = plan.clone();
        self.plan = plan;
        self.undone.clear();
        self.redone.clear();
        self.issues.clear();
        self.is_dirty = false;
    }

    pub const fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Whether the session can be written to at all.
    pub const fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// The refusal a mutating command gives in a read-only session.
    pub fn refuse_if_read_only(&self) -> Option<Outcome> {
        self.is_read_only
            .then(|| Outcome::Refused(READ_ONLY_REASON.to_owned()))
    }

    /// The refusal a write gives while the draft has issues.
    pub fn refuse_if_invalid(&self) -> Option<Outcome> {
        let issue = super::issue_words(self.issues.first()?, self);
        Some(Outcome::Refused(format!(
            "not saved, {} issue(s): {issue}",
            self.issues.len()
        )))
    }

    /// Written under a name of its own, the draft is a plan whatever it
    /// was resolved from.
    pub fn saved_as(&mut self) {
        self.is_read_only = false;
        self.is_dirty = false;
    }
}

/// Writes the draft to `path` as canonical TOML, through the same
/// validation gate as every other write.
///
/// # Errors
///
/// The refusal to say: the draft's first issue, or what the write failed
/// on.
pub fn write_draft(draft: &Draft, path: &Path) -> Result<(), String> {
    if let Some(Outcome::Refused(reason)) = draft.refuse_if_invalid() {
        return Err(reason);
    }
    crate::commands::write_plan(path, &draft.plan)
}

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
        let draft = &mut *self.draft;
        let before = mem::replace(&mut draft.committed, draft.plan.clone());
        draft.undone.push_back(before);
        if draft.undone.len() > HISTORY_DEPTH {
            draft.undone.pop_front();
        }
        draft.redone.clear();
        self.revalidate();
    }

    /// Makes `plan` the draft, giving back the one it replaces.
    fn swap_in(&mut self, plan: Plan) -> Plan {
        self.draft.committed = plan.clone();
        let replaced = mem::replace(&mut self.draft.plan, plan);
        self.revalidate();
        replaced
    }

    fn revalidate(&mut self) {
        self.draft.is_dirty = true;
        self.draft.issues = validate_plan(&self.draft.plan, &self.session.tables);
        match self.draft.issues.first() {
            None => {
                self.projected.projection = project(&self.draft.plan, &self.session.tables);
                self.projected.plan = self.draft.plan.clone();
            }
            Some(issue) => journal::warn(super::issue_words(issue, &self.draft)),
        }
    }
}

pub fn undo(mut editor: DraftEditor) -> Outcome {
    let Some(plan) = editor.draft.undone.pop_back() else {
        return Outcome::Refused("nothing to undo".to_owned());
    };
    let replaced = editor.swap_in(plan);
    editor.draft.redone.push(replaced);
    Outcome::Done
}

pub fn redo(mut editor: DraftEditor) -> Outcome {
    let Some(plan) = editor.draft.redone.pop() else {
        return Outcome::Refused("nothing to redo".to_owned());
    };
    let replaced = editor.swap_in(plan);
    editor.draft.undone.push_back(replaced);
    Outcome::Done
}

pub fn save(mut draft: ResMut<Draft>, session: Res<Session>, mut watch: ResMut<Watch>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return refusal;
    }
    let path = match session.document() {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refused(refusal),
    };
    if let Err(refusal) = write_draft(&draft, path) {
        return Outcome::Refused(refusal);
    }
    watch.restamp();
    draft.is_dirty = false;
    journal::say(format!("saved {}", path.display()));
    Outcome::Done
}
