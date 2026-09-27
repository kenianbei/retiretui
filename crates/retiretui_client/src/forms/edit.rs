//! The item being edited: the snapshot its form's fields work on, what it
//! was when it was opened, and what applying it stores. Whatever draws the
//! form holds one of these while the item is open.

use std::collections::BTreeMap;
use std::iter;

use toml::{Table, Value};

use super::applies::{self, HAPPENS, ON_KEY, ONCE_UNDATED, happens_once};
use super::cells::field_of;
use super::lists::is_gate_open;
use super::offers::display_name;
use super::{FieldKind, FieldSpec, Form, Target};
use crate::codec::{get_path, is_within, set_path};
use crate::draft::Draft;

/// An item is followed by what it was when opened, so one changed or
/// renamed underneath may be another item altogether.
const CHANGED_UNDERNEATH: &str =
    "the plan changed under this edit; discard it and open the item again";

/// One item open in its form.
pub struct ItemEdit {
    form: Form,
    index: Option<usize>,
    /// What the form's fields write; applying copies it into the draft.
    snapshot: Table,
    pristine: Table,
    /// Fields the item had no use for as it was opened, yet held: they stay
    /// on show, to be seen and cleared by hand rather than by the form.
    stale: Vec<&'static str>,
    /// Stale fields cleared by hand and left: no longer kept on show for
    /// being stale, though still stale, so applying writes them cleared.
    cleared: Vec<&'static str>,
    /// What is wrong with each field whose parts do not yet make a value,
    /// by its key: what they hold is in no table the snapshot could.
    incomplete: BTreeMap<&'static str, &'static str>,
}

impl ItemEdit {
    /// Item `index` of `form`, as `draft` holds it.
    #[must_use]
    pub fn open(form: Form, draft: &Draft, index: usize) -> Self {
        let pristine = (form.item)(draft, index).unwrap_or_default();
        Self::of(form, Some(index), pristine)
    }

    /// A new item of `form`'s list, the owner already filled in.
    #[must_use]
    pub fn create(form: Form, draft: &Draft) -> Self {
        let blank = form.list.map(|list| (list.blank)(&draft.plan));
        Self::of(form, None, blank.unwrap_or_default())
    }

    fn of(form: Form, index: Option<usize>, pristine: Table) -> Self {
        let snapshot = applies::opened(&form, &pristine);
        Self {
            stale: applies::stale(&form, &pristine, &snapshot),
            form,
            index,
            snapshot,
            pristine,
            cleared: Vec::new(),
            incomplete: BTreeMap::new(),
        }
    }

    /// What the form's fields hold.
    #[must_use]
    pub const fn snapshot(&self) -> &Table {
        &self.snapshot
    }

    /// Where the item sat when opened, or last applied; `None` while new.
    #[must_use]
    pub const fn index(&self) -> Option<usize> {
        self.index
    }

    /// Whether the field `key`'s parts do not yet make a value.
    #[must_use]
    pub fn is_incomplete(&self, key: &str) -> bool {
        self.incomplete.contains_key(key)
    }

    /// Writes `value` into the field `key`; `complaint` is why the parts it
    /// was made from do not yet make one, where they do not.
    pub fn set(
        &mut self,
        key: &'static str,
        value: Option<Value>,
        complaint: Option<&'static str>,
    ) {
        match complaint {
            Some(complaint) => self.incomplete.insert(key, complaint),
            None => self.incomplete.remove(key),
        };
        set_path(&mut self.snapshot, key, value);
    }

    /// Ticks or clears the tick `key` stands for: its table as it is first
    /// made, or none, dropping whatever its rows had not yet made.
    pub fn tick(&mut self, key: &'static str, is_ticked: bool) {
        let ticked = field_of(self.form.fields, key).and_then(|spec| match spec.kind {
            FieldKind::Presence(ticked) => Some(ticked),
            _ => None,
        });
        self.incomplete.retain(|held, _| !is_within(held, key));
        let table = ticked.filter(|_| is_ticked);
        let table = table.map(|ticked| Value::Table(ticked.parse().unwrap_or_default()));
        self.set(key, table, None);
    }

    /// What the item's form is called: the item's own name, or the kind of
    /// item being made.
    #[must_use]
    pub fn title(&self) -> String {
        let Some(list) = self.form.list else {
            return self.form.title.to_owned();
        };
        let name = display_name(&self.snapshot, list.identity, self.form.fields);
        match (self.index, name) {
            (None, _) => format!("New {}", list.singular),
            (Some(_), Some(name)) => format!("Edit {name}"),
            (Some(_), None) => format!("Edit {}", list.singular),
        }
    }

    /// Whether applying would store anything the item does not hold.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.held_back().is_some() || self.written() != self.stripped(self.pristine.clone())
    }

    /// What is wrong with the first field that does not yet make a value,
    /// under the label the form shows for it.
    fn complaint(&self) -> Option<String> {
        let (key, complaint) = self.held_back()?;
        let spec = field_of(self.form.fields, key);
        let named = spec.map_or(key, |spec| spec.label);
        Some(format!("{named}: {complaint}"))
    }

    /// The first field on show that does not yet make a value, and what is
    /// wrong with it. Once is only a pick until it has a date, which no
    /// file could state, so it is held back as a half-made trigger is.
    fn held_back(&self) -> Option<(&'static str, &'static str)> {
        let mut unmade = self.incomplete.iter().filter(|(key, _)| self.uses(key));
        if let Some((key, complaint)) = unmade.next() {
            return Some((key, complaint));
        }
        let asks_timing = self.snapshot.contains_key(HAPPENS);
        let is_undated = happens_once(&self.snapshot) && !self.snapshot.contains_key(ON_KEY);
        (asks_timing && is_undated).then_some((ON_KEY, ONCE_UNDATED))
    }

    /// Hides the stale fields cleared by hand but `held`, the one being
    /// typed in, answering whether any went: a row cleared and left has
    /// nothing left to be seen for.
    pub fn hide_cleared(&mut self, held: Option<&str>) -> bool {
        let is_cleared = |key: &&&'static str| {
            held != Some(**key)
                && get_path(&self.snapshot, key).is_none()
                && !self.cleared.contains(key)
        };
        let newly: Vec<&'static str> = self.stale.iter().filter(is_cleared).copied().collect();
        self.cleared.extend(&newly);
        !newly.is_empty()
    }

    /// Whether the field `key` is on show: under a tick holding its table,
    /// where it sits under one, and used by its own rule, or stale and not
    /// yet cleared and left.
    #[must_use]
    pub fn is_on_show(&self, key: &str) -> bool {
        let spec = field_of(self.form.fields, key);
        is_gate_open(self.form.fields, key, &self.snapshot)
            && spec.is_none_or(|spec| {
                spec.is_shown_for(&self.snapshot)
                    || (self.stale.contains(&spec.key) && !self.cleared.contains(&spec.key))
            })
    }

    fn applies(&self, spec: &FieldSpec) -> bool {
        spec.is_shown_for(&self.snapshot) || self.stale.contains(&spec.key)
    }

    /// Whether the item has a use for the field `key`.
    fn uses(&self, key: &str) -> bool {
        let spec = field_of(self.form.fields, key);
        spec.is_none_or(|spec| self.applies(spec))
    }

    /// `item` less what the item being edited has no use for, and with
    /// each field no file holds turned back into what one does.
    fn stripped(&self, mut item: Table) -> Table {
        for spec in self.form.fields {
            if let Some(derived) = spec.derived {
                if let Some(value) = item.remove(spec.key) {
                    (derived.write)(&mut item, &value);
                }
            } else if !self.applies(spec) {
                set_path(&mut item, spec.key, None);
            }
        }
        item
    }

    /// What applying stores.
    fn written(&self) -> Table {
        self.stripped(self.snapshot.clone())
    }

    /// Where the item now sits, found by what it was when opened, so one
    /// moved underneath is followed; `None` once nothing is what was opened.
    fn stored_at(&self, draft: &Draft) -> Option<usize> {
        let Some(held) = self.index else {
            return Some(self.form.list.map_or(0, |list| (list.count)(&draft.plan)));
        };
        let count = self.form.list.map_or(1, |list| (list.count)(&draft.plan));
        let is_opened =
            |index: &usize| (self.form.item)(draft, *index).as_ref() == Some(&self.pristine);
        iter::once(held).chain(0..count).find(is_opened)
    }

    /// Drops every edit, back to the item as it was opened or last applied.
    pub fn discard(&mut self) {
        self.snapshot = applies::opened(&self.form, &self.pristine);
        self.incomplete.clear();
    }

    /// Stores the item into `draft`, answering where it now sits, or `None`
    /// where an item already in the plan holds no edits to store. The draft
    /// is left for the caller to record and revalidate. A refusal keeps the
    /// values that caused it, the schema's reported under `label` or, with
    /// none, the kind of item.
    ///
    /// # Errors
    ///
    /// Why nothing was stored: a read-only draft, a field not yet making a
    /// value, an item changed underneath, or what the schema refuses.
    pub fn apply(
        &mut self,
        draft: &mut Draft,
        label: Option<&str>,
    ) -> Result<Option<usize>, String> {
        if self.form.target == Target::Draft
            && let Some(reason) = draft.refuse_if_read_only()
        {
            return Err(reason);
        }
        if !self.is_dirty() && self.index().is_some() {
            return Ok(None);
        }
        if let Some(complaint) = self.complaint() {
            return Err(complaint);
        }
        let Some(index) = self.stored_at(draft) else {
            return Err(format!("{}: {CHANGED_UNDERNEATH}", self.title()));
        };
        let written = self.written();
        if let Err(message) = (self.form.store)(draft, index, written.clone()) {
            let form = self.form;
            let named = label.unwrap_or_else(|| form.list.map_or(form.title, |list| list.singular));
            return Err(format!("{named}: {message}"));
        }
        self.pristine = written;
        self.index = Some(index);
        Ok(Some(index))
    }
}

#[cfg(test)]
mod tests {
    use retiretui_engine::plan::Plan;

    use super::*;
    use crate::forms::{DOMAINS, DomainId};
    use crate::setup::EXAMPLES;

    fn accounts() -> Form {
        let is_accounts = |form: &&Form| form.domain == Some(DomainId::Accounts);
        *DOMAINS.iter().find(is_accounts).expect("a domain")
    }

    fn draft(is_read_only: bool) -> Draft {
        Draft::new(
            Plan::from_toml_str(EXAMPLES[0].2).expect("parses"),
            is_read_only,
        )
    }

    #[test]
    fn an_edit_is_stored_where_the_item_sat_and_only_once_it_changed() {
        let mut draft = draft(false);
        let mut edit = ItemEdit::open(accounts(), &draft, 0);
        assert_eq!(edit.apply(&mut draft, None), Ok(None), "nothing to store");
        edit.set("name", Some(Value::String("Renamed".to_owned())), None);
        assert!(edit.is_dirty());
        assert_eq!(edit.apply(&mut draft, None), Ok(Some(0)));
        assert_eq!(draft.plan.accounts[0].name.as_deref(), Some("Renamed"));
        assert!(!edit.is_dirty(), "applied is the new pristine");
    }

    #[test]
    fn an_edit_is_refused_while_read_only_or_unmade() {
        let mut read_only = draft(true);
        let mut edit = ItemEdit::open(accounts(), &read_only, 0);
        edit.set("name", Some(Value::String("Renamed".to_owned())), None);
        assert!(edit.apply(&mut read_only, None).is_err());
        let mut draft = draft(false);
        let mut edit = ItemEdit::create(accounts(), &draft);
        edit.set("locked_until", None, Some("half made"));
        let refusal = edit.apply(&mut draft, None).expect_err("held back");
        assert!(refusal.ends_with("half made"), "{refusal}");
    }
}
