//! The item being edited: the snapshot its form's fields work on, what it
//! was when it was opened, and what is asked before an edit is dropped.

use std::collections::BTreeMap;
use std::iter;

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut, Mut};
use bevy_ecs::prelude::{Commands, Component, Entity, In, Query, Res, ResMut, Resource, World};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::{FocusCause, InputFocus};
use plurimus::ui::ModalOpen;
use toml::Table;

use super::build;
use super::domain::{ListOps, Ops, Target};
use super::draft::{Draft, DraftEditor};
use super::table::{DomainTable, Row};
use crate::confirm::{Answer, Confirm};
use crate::hints::Hints;
use crate::journal;
use crate::nav::Page;
use crate::overlay::{self, Standing};
use crate::pane::Framed;
use crate::scope::KeyScope;
use crate::session::Session;
use retiretui_client::codec::{get_path, set_path};
use retiretui_client::forms::FieldSpec;
use retiretui_client::forms::applies::{self, HAPPENS, ON_KEY, ONCE_UNDATED, happens_once};
use retiretui_client::forms::cells::field_of;

/// Where an item is stored: where it sat in the plan when opened - the
/// first place it is looked for - or, while a new one is composed, the
/// list it is to join.
#[derive(Clone, Copy)]
pub enum Slot {
    At(Row),
    New(ListOps),
}

impl Slot {
    pub(super) const fn row(self) -> Option<Row> {
        match self {
            Self::At(row) => Some(row),
            Self::New(_) => None,
        }
    }
}

#[derive(Resource, Default)]
pub struct EditSession(pub(super) Option<Editing>);

pub(super) struct Editing {
    pub(super) ops: Ops,
    /// The table the item is one of; `None` for a single-item domain.
    pub(super) table: Option<Entity>,
    pub(super) slot: Slot,
    pub(super) form: Option<Entity>,
    /// What the form's fields write; applying copies it into the draft.
    pub(super) snapshot: Table,
    pristine: Table,
    /// Fields the item had no use for as it was opened, yet held: they stay
    /// on show, to be seen and cleared by hand rather than by the form.
    pub(super) stale: Vec<&'static str>,
    /// Stale fields cleared by hand and left: no longer kept on show for
    /// being stale, though still stale, so applying writes them cleared.
    pub(super) cleared: Vec<&'static str>,
    /// What is wrong with each field whose widgets do not yet make a value,
    /// by its key: what they hold is in no table the snapshot could.
    pub(super) incomplete: BTreeMap<&'static str, &'static str>,
    /// Whether the form's widgets show the snapshot yet.
    pub(super) is_seeded: bool,
    /// Whether the keyboard is owed to the form's first field.
    pub(super) takes_keyboard: bool,
    /// Whether the form stands where the shell shows nothing: the new
    /// plan's, with no document beneath it. It then rests on itself, so
    /// the shell's keys reach, and offers no way to cancel.
    pub(super) is_alone: bool,
}

impl Editing {
    /// What is wrong with the first field that does not yet make a value,
    /// under the label the form shows for it.
    fn complaint(&self) -> Option<String> {
        let (key, complaint) = self.held_back()?;
        let spec = super::cells::field_of(self.ops.fields, key);
        let named = spec.map_or(key, |spec| spec.label);
        Some(format!("{named}: {complaint}"))
    }

    pub(super) fn new(ops: Ops, table: Option<Entity>, slot: Slot, draft: &Draft) -> Self {
        let pristine = match slot {
            Slot::At(Row(index)) => (ops.item)(draft, index).unwrap_or_default(),
            Slot::New(list) => (list.blank)(&draft.plan),
        };
        let snapshot = applies::opened(&ops, &pristine);
        Self {
            ops,
            table,
            slot,
            form: None,
            stale: applies::stale(&ops, &pristine, &snapshot),
            cleared: Vec::new(),
            pristine,
            snapshot,
            incomplete: BTreeMap::new(),
            is_seeded: false,
            takes_keyboard: false,
            is_alone: false,
        }
    }

    /// Where the item now sits, found by what it was when opened, so one
    /// moved underneath is followed; `None` once nothing is what was opened.
    fn stored_at(&self, draft: &Draft) -> Option<usize> {
        let held = match self.slot {
            Slot::At(Row(held)) => held,
            Slot::New(list) => return Some((list.count)(&draft.plan)),
        };
        let count = self.ops.list.map_or(1, |list| (list.count)(&draft.plan));
        let is_opened =
            |index: &usize| (self.ops.item)(draft, *index).as_ref() == Some(&self.pristine);
        iter::once(held).chain(0..count).find(is_opened)
    }

    pub(super) fn is_dirty(&self) -> bool {
        self.held_back().is_some() || self.written() != self.stripped(self.pristine.clone())
    }

    /// What the item's form is called: the item's own name, or the kind of
    /// item being made.
    pub(super) fn title(&self) -> String {
        let Some(list) = self.ops.list else {
            return self.ops.title.to_owned();
        };
        let name = super::offers::display_name(&self.snapshot, list.identity, self.ops.fields);
        match (self.slot, name) {
            (Slot::New(_), _) => format!("New {}", list.singular),
            (Slot::At(_), Some(name)) => format!("Edit {name}"),
            (Slot::At(_), None) => format!("Edit {}", list.singular),
        }
    }
}

/// The form standing over a table while one of its items is open.
#[derive(Component, Default, Debug)]
pub struct ItemForm;

/// An item is followed by what it was when opened, so one changed or
/// renamed underneath may be another item altogether.
const CHANGED_UNDERNEATH: &str =
    "the plan changed under this edit; discard it and open the item again";

pub(super) const ITEM_HINTS: Hints = Hints(&[("⇥", "next"), ("⏎", "apply"), ("esc", "close")]);
/// A form standing alone has nothing to leave it for.
const ALONE_HINTS: Hints = Hints(&[("⏎", "edit")]);

impl EditSession {
    pub fn is_dirty(&self) -> bool {
        self.0.as_ref().is_some_and(Editing::is_dirty)
    }

    pub fn editing_table(&self) -> Option<Entity> {
        self.0.as_ref().and_then(|editing| editing.table)
    }

    /// Whether an item of `surface` is open.
    pub fn is_over(&self, surface: Option<Page>) -> bool {
        self.0
            .as_ref()
            .is_some_and(|editing| editing.ops.surface == surface)
    }

    pub fn is_open(&self) -> bool {
        self.0.is_some()
    }

    /// Drops the item, edits and all: for a form whose page has gone.
    pub fn close(&mut self) {
        self.0 = None;
    }
}

/// Opens `row` of `ops` for editing - of `table` where the domain has
/// one - in the form that stands over the page, the keyboard in its first
/// field.
pub fn open_item(
    In((ops, table, slot)): In<(Ops, Option<Entity>, Slot)>,
    (draft, shell): (Res<Draft>, Res<Session>),
    mut state: SessionFocus,
) {
    if ops.target == Target::Draft
        && let Some(reason) = draft.refuse_if_read_only()
    {
        journal::warn(reason);
        return;
    }
    let mut editing = Editing::new(ops, table, slot, &draft);
    editing.takes_keyboard = true;
    editing.is_alone = shell.is_empty();
    state.session.0 = Some(editing);
}

/// Stands the open item in a form over the page, and takes the form down
/// when the session closes.
pub fn sync_item_form(
    mut session: ResMut<EditSession>,
    mut standing: Standing<ItemForm>,
    mut commands: Commands,
) {
    if !session.is_changed() {
        return;
    }
    let Some(editing) = session.bypass_change_detection().0.as_mut() else {
        if standing.is_open() {
            standing.close(&mut commands);
        }
        return;
    };
    if editing.form.is_some() {
        return;
    }
    let Some(form) = standing.open(&mut commands) else {
        return;
    };
    editing.form = Some(form);
    let hints = if editing.is_alone {
        ALONE_HINTS
    } else {
        ITEM_HINTS
    };
    commands.entity(form).insert((
        build::centred(),
        Framed::over(editing.title()),
        ModalOpen,
        hints,
    ));
    // Alone, the shell's keys reach through the form as they did through
    // the page it replaced; a field keeps only the plain ones.
    if editing.is_alone {
        commands.entity(form).remove::<KeyScope>();
    }
    build::spawn_form(&mut commands, form, editing.ops, editing.is_alone);
}

/// Applies or discards the edits a question was asked about.
fn resolve(In(applies): In<bool>, mut state: SessionFocus, mut editor: DraftEditor) {
    if applies {
        state.apply(None, &mut editor);
    } else {
        state.discard();
    }
}

/// The session, and the table it leaves a cursor on.
#[derive(SystemParam)]
pub struct SessionFocus<'w, 's> {
    pub(super) session: ResMut<'w, EditSession>,
    focus: ResMut<'w, InputFocus>,
    overlays: ResMut<'w, overlay::Focus>,
    pub(super) tables: Query<'w, 's, &'static mut DomainTable>,
    pub confirm: ResMut<'w, Confirm>,
}

impl SessionFocus<'_, '_> {
    /// Whether the item holds edits leaving it would drop; where it does,
    /// what to do with them is asked.
    pub fn holds_back(&mut self) -> bool {
        let Some(editing) = self.session.0.as_ref().filter(|held| held.is_dirty()) else {
            return false;
        };
        let answer = |applies: bool| {
            move |commands: &mut Commands| {
                commands.run_system_cached_with(resolve, applies);
            }
        };
        // What is refused may already have moved the keyboard by the time
        // the question closes; cancelling gives it back to what held it.
        let resumed = self.focus.get();
        let resume = move |commands: &mut Commands| {
            commands.queue(move |world: &mut World| {
                world.resource_scope(|world, mut overlays: Mut<overlay::Focus>| {
                    overlays.rebase(resumed, &mut world.resource_mut::<InputFocus>());
                });
            });
        };
        self.confirm.ask_among(
            format!("{} has changes that were not applied.", editing.title()),
            vec![
                Answer::running("Cancel", resume),
                Answer::running("Discard", answer(false)).destructive(),
                Answer::running("Apply", answer(true)).primary(),
            ],
        );
        true
    }

    /// Gives `entity` the keyboard, or names it to be given it once what
    /// stands over the page - the question that closed the item - is gone.
    pub(super) fn focus(&mut self, entity: Entity) {
        self.overlays.rebase(Some(entity), &mut self.focus);
    }

    /// Gives `entity`, a widget of the form that stands, the keyboard now.
    pub(super) fn enter(&mut self, entity: Entity) {
        self.focus.set(entity, FocusCause::Navigated);
    }

    /// Stores the item into the draft and answers whether it took. A
    /// table's item then closes, leaving the cursor on it; a single item
    /// stays. A refusal keeps the values that caused it, reported under
    /// `label` or, with none, the kind of item.
    pub(super) fn apply(&mut self, label: Option<&str>, editor: &mut DraftEditor) -> bool {
        let Some(editing) = self.session.0.as_mut() else {
            return false;
        };
        let target = editing.ops.target;
        if target == Target::Draft
            && let Some(reason) = editor.draft.refuse_if_read_only()
        {
            journal::warn(reason);
            return false;
        }
        if !editing.is_dirty() && editing.slot.row().is_some() {
            self.close();
            return true;
        }
        if let Some(complaint) = editing.complaint() {
            journal::warn(complaint);
            return false;
        }
        let draft = editor.draft.bypass_change_detection();
        let Some(index) = editing.stored_at(draft) else {
            journal::warn(format!("{}: {CHANGED_UNDERNEATH}", editing.title()));
            return false;
        };
        let written = editing.written();
        if let Err(message) = (editing.ops.store)(draft, index, written.clone()) {
            let ops = editing.ops;
            let named = label.unwrap_or_else(|| ops.list.map_or(ops.title, |list| list.singular));
            journal::warn(format!("{named}: {message}"));
            return false;
        }
        editing.pristine = written;
        editing.slot = Slot::At(Row(index));
        if let Some(mut table) = editing
            .table
            .and_then(|table| self.tables.get_mut(table).ok())
        {
            table.wanted = Some(Row(index));
            table.is_applied = true;
        }
        if target == Target::Draft {
            editor.commit();
        }
        self.close();
        true
    }

    /// Leaves the item: at once where it holds no edits, and once what is
    /// to become of them is known where it does.
    pub(super) fn leave(&mut self) {
        if !self.holds_back() {
            self.discard();
        }
    }

    pub(super) fn discard(&mut self) {
        if let Some(editing) = self.session.0.as_mut() {
            editing.snapshot = applies::opened(&editing.ops, &editing.pristine);
            editing.incomplete.clear();
            editing.is_seeded = false;
        }
        self.close();
    }

    /// Closes the item: its form is taken down, and the keyboard goes back
    /// to what opened it.
    fn close(&mut self) {
        self.session.0 = None;
    }
}

impl Editing {
    /// Hides the stale fields cleared by hand but `held`, the one the
    /// keyboard is in, answering whether any went: a row cleared and left
    /// has nothing left to be seen for.
    pub(super) fn hide_cleared(&mut self, held: Option<&str>) -> bool {
        let is_cleared = |key: &&&'static str| {
            held != Some(**key)
                && get_path(&self.snapshot, key).is_none()
                && !self.cleared.contains(key)
        };
        let newly: Vec<&'static str> = self.stale.iter().filter(is_cleared).copied().collect();
        self.cleared.extend(&newly);
        !newly.is_empty()
    }

    /// Whether the field `key` is on show: used by its own rule, or stale
    /// and not yet cleared and left.
    pub(super) fn is_on_show(&self, key: &str) -> bool {
        let spec = field_of(self.ops.fields, key);
        spec.is_none_or(|spec| {
            spec.shown.is_none_or(|shown| shown(&self.snapshot))
                || (self.stale.contains(&spec.key) && !self.cleared.contains(&spec.key))
        })
    }

    fn applies(&self, spec: &FieldSpec) -> bool {
        spec.shown.is_none_or(|shown| shown(&self.snapshot)) || self.stale.contains(&spec.key)
    }

    /// Whether the item has a use for the field `key`.
    pub(super) fn uses(&self, key: &str) -> bool {
        let spec = field_of(self.ops.fields, key);
        spec.is_none_or(|spec| self.applies(spec))
    }

    /// The first field on show that does not yet make a value, and what is
    /// wrong with it. Once is only a pick until it has a date, which no
    /// file could state, so it is held back as a half-made trigger is.
    pub(super) fn held_back(&self) -> Option<(&'static str, &'static str)> {
        let mut unmade = self.incomplete.iter().filter(|(key, _)| self.uses(key));
        if let Some((key, complaint)) = unmade.next() {
            return Some((key, complaint));
        }
        let asks_timing = self.snapshot.contains_key(HAPPENS);
        let is_undated = happens_once(&self.snapshot) && !self.snapshot.contains_key(ON_KEY);
        (asks_timing && is_undated).then_some((ON_KEY, ONCE_UNDATED))
    }

    /// `item` less what the item being edited has no use for, and with
    /// each field no file holds turned back into what one does.
    pub(super) fn stripped(&self, mut item: Table) -> Table {
        for spec in self.ops.fields {
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
    pub(super) fn written(&self) -> Table {
        self.stripped(self.snapshot.clone())
    }
}
