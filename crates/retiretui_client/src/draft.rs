//! The working copy every edit lands in, its whole-plan history, and the
//! gate every write of it goes through.

use std::collections::VecDeque;
use std::mem;
use std::path::Path;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Issue, Plan, Scenario, to_table};
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
    origin: Origin,
    issues: Vec<Issue>,
}

/// What the draft was resolved from, and so how it is written back.
#[derive(Debug)]
enum Origin {
    /// A plan file, or a plan not yet named.
    Plan,
    /// A scenario whose overlay the draft cannot be written back into.
    ReadOnly,
    /// A scenario, saved into its own overlay over what is beneath it.
    Over(Beneath),
}

/// What a scenario's draft is stated over when it is saved: the plan the
/// scenario's own overlay applies to, and that overlay as last read or
/// written.
#[derive(Debug, Clone)]
pub struct Beneath {
    /// The plan the overlay applies to, as its canonical table.
    pub table: Table,
    /// The overlay as last read or written.
    pub kept: Scenario,
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

    /// `plan` as a draft of its own, read-only where it was resolved from a
    /// scenario whose overlay it cannot be written back into.
    #[must_use]
    pub fn new(plan: Plan, is_read_only: bool) -> Self {
        Self {
            committed: plan.clone(),
            plan,
            tools: Table::new(),
            undone: VecDeque::new(),
            redone: Vec::new(),
            is_dirty: false,
            origin: if is_read_only {
                Origin::ReadOnly
            } else {
                Origin::Plan
            },
            issues: Vec::new(),
        }
    }

    /// `plan`, resolved from a scenario, as a draft saved back into the
    /// scenario's overlay over `beneath`.
    #[must_use]
    pub fn over(plan: Plan, beneath: Beneath) -> Self {
        Self {
            origin: Origin::Over(beneath),
            ..Self::new(plan, false)
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

    /// Starts over as `fresh`, as after a reload, keeping what the tools
    /// hold beside the plan.
    pub fn reset(&mut self, fresh: Self) {
        let tools = mem::take(&mut self.tools);
        *self = Self { tools, ..fresh };
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

    /// Whether the draft is saved into the overlay of the scenario it was
    /// resolved from.
    #[must_use]
    pub const fn is_over_scenario(&self) -> bool {
        matches!(self.origin, Origin::Over(_))
    }

    /// Whether the session can be written to at all.
    #[must_use]
    pub const fn is_read_only(&self) -> bool {
        matches!(self.origin, Origin::ReadOnly)
    }

    /// The refusal a mutating command gives in a read-only session.
    #[must_use]
    pub fn refuse_if_read_only(&self) -> Option<String> {
        self.is_read_only().then(|| READ_ONLY_REASON.to_owned())
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
        let is_scenario = !matches!(self.origin, Origin::Plan);
        (is_scenario && is_its_file).then(|| OVER_SCENARIO.to_owned())
    }

    /// The refusal an edit gives where the scenario's overlay cannot state
    /// it - what its base states, cleared - with the plan put back as last
    /// committed, which is where every edit starts from.
    pub fn refuse_if_unsaid(&mut self) -> Option<String> {
        let Origin::Over(beneath) = &self.origin else {
            return None;
        };
        let refusal = self.stated_over(beneath).err()?;
        self.plan = self.committed.clone();
        Some(refusal)
    }

    /// The draft as the overlay that makes `beneath`'s plan into it.
    fn stated_over(&self, beneath: &Beneath) -> Result<Scenario, String> {
        let draft = to_table(&self.plan).map_err(crate::files::not_saved)?;
        let base = beneath.kept.base();
        Scenario::over(base, &beneath.table, &draft, Some(&beneath.kept)).map_err(|issues| {
            let issue = crate::issues::issue_words(&issues[0], self);
            format!("{issue}; clear it in {base}")
        })
    }

    /// Written under a name of its own, the draft is a plan whatever it
    /// was resolved from.
    pub fn saved_as(&mut self) {
        self.origin = Origin::Plan;
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
    if let Some(reason) = draft.refuse_if_invalid() {
        return Err(reason);
    }
    crate::files::write_plan(store, path, &draft.plan)
}

/// Writes the draft back to `path`, where it was opened from, through the
/// same gate as [`write_draft`]: a scenario's as its overlay, stated over
/// what is beneath it as read now, and any other as a plan.
///
/// # Errors
///
/// The refusal to say: the draft's first issue, what the overlay cannot
/// state, or what the read or the write failed on.
pub fn save_draft(store: &dyn Store, draft: &mut Draft, path: &Path) -> Result<(), String> {
    if let Some(reason) = draft.refuse_if_invalid() {
        return Err(reason);
    }
    if draft.is_over_scenario() {
        let beneath = crate::files::load_beneath(store, path)?;
        let kept = draft.stated_over(&beneath)?;
        let text = kept.to_toml_string().map_err(crate::files::not_saved)?;
        crate::files::write_text(store, path, &text)?;
        draft.origin = Origin::Over(Beneath { kept, ..beneath });
    } else {
        crate::files::write_plan(store, path, &draft.plan)?;
    }
    draft.saved();
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::files::{load_beneath, load_plan_with_files};
    use crate::setup::EXAMPLES;
    use crate::store::KeyStore;
    use crate::store::memory::Memory;

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
        let (_, _, mut over) = scenario_draft();
        assert!(
            over.refuse_over_scenario(true).is_some(),
            "an editable one too"
        );
        over.saved_as();
        assert!(!over.is_over_scenario());
        assert_eq!(over.refuse_over_scenario(true), None);
    }

    const BASE: &str = "/w/mid-career-couple.toml";
    /// Pins the base's own inflation, and names the plan.
    const SCENARIO: &str = "schema = 1\nbase = \"mid-career-couple.toml\"\n\n\
        [plan]\ninflation = 0.025\nname = \"Retire early\"\n";

    fn scenario_draft() -> (KeyStore<Memory>, PathBuf, Draft) {
        let store = KeyStore::new(Memory::default());
        store.create_dir_all(Path::new("/w")).unwrap();
        store.write(Path::new(BASE), EXAMPLES[1].2).unwrap();
        let path = PathBuf::from("/w/early.toml");
        store.write(&path, SCENARIO).unwrap();
        let plan = load_plan_with_files(&store, &path, &mut Vec::new()).unwrap();
        let beneath = load_beneath(&store, &path).unwrap();
        (store, path, Draft::over(plan, beneath))
    }

    #[test]
    fn a_scenario_draft_is_saved_into_its_overlay() {
        let (store, path, mut draft) = scenario_draft();
        let base = store.read(Path::new(BASE)).unwrap();
        draft.plan.plan.horizon_age = 97;
        assert!(draft.revalidate(&TaxTables::embedded()));
        save_draft(&store, &mut draft, &path).expect("saved");
        assert!(!draft.is_dirty());
        let text = store.read(&path).unwrap();
        let saved = Scenario::from_toml_str(&text).unwrap().expect("a scenario");
        assert_eq!(saved.base(), "mid-career-couple.toml");
        assert!(text.contains("inflation = 0.025"), "still pinned: {text}");
        assert!(text.contains("horizon_age = 97"), "{text}");
        let resolved = load_plan_with_files(&store, &path, &mut Vec::new()).unwrap();
        assert_eq!(resolved, draft.plan);
        assert_eq!(
            store.read(Path::new(BASE)).unwrap(),
            base,
            "the base untouched"
        );
    }

    #[test]
    fn clearing_what_the_base_states_is_refused_in_the_forms_words() {
        let (store, path, mut draft) = scenario_draft();
        assert_eq!(draft.refuse_if_unsaid(), None);
        draft.plan.plan.name = None;
        let refusal = draft.refuse_if_unsaid().expect("refused");
        assert_eq!(
            refusal,
            "Settings \u{203a} Plan name: a scenario cannot clear what its base states; \
             clear it in mid-career-couple.toml"
        );
        let name = draft.plan.plan.name.as_deref();
        assert_eq!(name, Some("Retire early"), "put back as last committed");
        draft.plan.plan.name = None;
        assert!(draft.revalidate(&TaxTables::embedded()));
        assert_eq!(save_draft(&store, &mut draft, &path), Err(refusal));
        assert_eq!(store.read(&path).unwrap(), SCENARIO, "nothing written");
    }
}
