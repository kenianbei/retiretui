//! The working copy every edit lands in, its whole-plan history, and the
//! gate every write of it goes through.

use std::collections::VecDeque;
use std::mem;
use std::path::Path;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::validate_plan;
use toml::{Table, Value};

use crate::forms::ToolAnswers;
use crate::store::Store;

/// The working copy every edit lands in. A valid draft is what the views
/// project; an invalid one keeps the last good projection and its issues.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::prelude::Resource))]
#[derive(Debug)]
pub struct Draft {
    /// The plan every edit lands in.
    pub plan: Plan,
    /// What each tool's form holds beside the plan, under the tool's own
    /// name; dropped with the document and kept across a reload.
    tools: Table,
    /// The plan as of the last commit, undo, redo, or reset: what the next
    /// commit's undo goes back to.
    committed: Plan,
    undone: VecDeque<Plan>,
    redone: Vec<Plan>,
    is_dirty: bool,
    is_read_only: bool,
    issues: Vec<Issue>,
}

/// Why a scenario session refuses a write.
pub(crate) const READ_ONLY_REASON: &str = "scenario sessions are read-only";

/// A scenario holds only its changes to a base; the plan resolved from it
/// written in its place would lose which were its own.
const OVER_SCENARIO: &str = "a scenario cannot be saved over; save it under a name of its own";

/// How many applied items undo reaches back over.
const HISTORY_DEPTH: usize = 100;

impl Draft {
    /// The answers tool `T`'s form holds, blank until it has been applied.
    pub fn answers<T: ToolAnswers>(&self) -> Table {
        let held = self.tools.get(T::SLOT).and_then(Value::as_table);
        held.cloned().unwrap_or_default()
    }

    /// Holds `answers` as tool `T`'s form's.
    pub fn set_answers<T: ToolAnswers>(&mut self, answers: Table) {
        self.tools.insert(T::SLOT.to_owned(), Value::Table(answers));
    }

    /// A scenario session is read-only: the resolved plan cannot be written
    /// back into an overlay.
    #[must_use]
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

    /// `plan` as a draft of its own, its issues found against `tables`;
    /// read-only where it was resolved from a scenario.
    #[must_use]
    pub fn validated(plan: Plan, tables: &TaxTables, is_read_only: bool) -> Self {
        let issues = validate_plan(&plan, tables);
        Self {
            issues,
            ..Self::new(plan, is_read_only)
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

    /// Whether the draft holds edits not yet saved.
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// What validation last found wrong with the draft, in validation order.
    #[must_use]
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Whether the session can be written to at all.
    #[must_use]
    pub const fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// The refusal a mutating command gives in a read-only session.
    #[must_use]
    pub fn refuse_if_read_only(&self) -> Option<String> {
        self.is_read_only.then(|| READ_ONLY_REASON.to_owned())
    }

    /// The refusal a write gives while the draft has issues.
    #[must_use]
    pub fn refuse_if_invalid(&self) -> Option<String> {
        let issue = crate::issues::issue_words(self.issues.first()?, self);
        Some(format!(
            "not saved, {}: {issue}",
            crate::present::issue_count(self.issues.len())
        ))
    }

    /// The refusal a save under another name gives where that name is the
    /// scenario the draft was resolved from, as `is_its_file` says.
    #[must_use]
    pub fn refuse_over_scenario(&self, is_its_file: bool) -> Option<String> {
        (self.is_read_only && is_its_file).then(|| OVER_SCENARIO.to_owned())
    }

    /// Written under a name of its own, the draft is a plan whatever it
    /// was resolved from.
    pub fn saved_as(&mut self) {
        self.is_read_only = false;
        self.is_dirty = false;
    }

    /// Written where it came from, the draft holds nothing unsaved.
    pub fn saved(&mut self) {
        self.is_dirty = false;
    }

    /// Records the edit just made as one step of history.
    pub fn record(&mut self) {
        let before = mem::replace(&mut self.committed, self.plan.clone());
        self.undone.push_back(before);
        if self.undone.len() > HISTORY_DEPTH {
            self.undone.pop_front();
        }
        self.redone.clear();
    }

    /// Whether there is an edit to step back over.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Whether there is an undone edit to step forward over.
    #[must_use]
    pub const fn can_redo(&self) -> bool {
        !self.redone.is_empty()
    }

    /// Steps back over the last recorded edit, answering whether there was
    /// one.
    pub fn undo(&mut self) -> bool {
        let Some(plan) = self.undone.pop_back() else {
            return false;
        };
        let replaced = self.swap_in(plan);
        self.redone.push(replaced);
        true
    }

    /// Steps forward over the last undone edit, answering whether there was
    /// one.
    pub fn redo(&mut self) -> bool {
        let Some(plan) = self.redone.pop() else {
            return false;
        };
        let replaced = self.swap_in(plan);
        self.undone.push_back(replaced);
        true
    }

    /// Makes `plan` the draft, giving back the one it replaces.
    fn swap_in(&mut self, plan: Plan) -> Plan {
        self.committed = plan.clone();
        mem::replace(&mut self.plan, plan)
    }

    /// Validates the draft after a change against `tables`, answering
    /// whether it is valid, and so ready to project.
    pub fn revalidate(&mut self, tables: &TaxTables) -> bool {
        self.is_dirty = true;
        self.issues = validate_plan(&self.plan, tables);
        self.issues.is_empty()
    }
}

/// The draft as canonical TOML, through the same validation gate as every
/// other write.
///
/// # Errors
///
/// The draft's first issue, or what it failed to serialize on.
pub fn draft_text(draft: &Draft) -> Result<String, String> {
    if let Some(reason) = draft.refuse_if_invalid() {
        return Err(reason);
    }
    crate::files::plan_text(&draft.plan)
}

/// Writes the draft to `path` as canonical TOML, through the same
/// validation gate as every other write.
///
/// # Errors
///
/// The refusal to say: the draft's first issue, or what the write failed
/// on.
pub fn write_draft(store: &dyn Store, draft: &Draft, path: &Path) -> Result<(), String> {
    let text = draft_text(draft)?;
    store
        .write(path, &text)
        .map_err(|error| format!("not saved: {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::EXAMPLES;

    fn draft(is_read_only: bool) -> Draft {
        let plan = Plan::from_toml_str(EXAMPLES[0].2).expect("parses");
        Draft::new(plan, is_read_only)
    }

    #[test]
    fn a_draft_is_written_as_text_only_without_issues() {
        let mut draft = draft(false);
        let text = draft_text(&draft).expect("valid");
        assert_eq!(Plan::from_toml_str(&text).ok(), Some(draft.plan.clone()));
        draft.plan.plan.inflation = 9.0;
        assert!(!draft.revalidate(&TaxTables::embedded()));
        let refusal = draft_text(&draft).expect_err("issues");
        assert!(refusal.starts_with("not saved, 1 issue"), "{refusal}");
    }

    #[test]
    fn only_a_scenario_is_refused_over_its_own_file() {
        assert_eq!(draft(false).refuse_over_scenario(true), None);
        assert_eq!(draft(true).refuse_over_scenario(false), None);
        assert!(draft(true).refuse_over_scenario(true).is_some());
    }
}
