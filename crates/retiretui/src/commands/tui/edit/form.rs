//! What a form's widgets do: every edit lands in the session's snapshot,
//! which reaches the draft only when the item is applied.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::{ChildOf, Commands, Entity, Local, Mut, On, Query, Res, ResMut, With};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::{FocusCause, FocusedInput, InputFocus};
use plurimus::core::UiWidget;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{KeyBinding, ModalDismiss, first_bound};
use plurimus::widgets::ratatui_widgets::paragraph::{Paragraph, Wrap};
use plurimus::widgets::{Activate, Submit, TextInput, ValueChange};
use toml::Value;

use super::LocatedIssue;
use super::build::{EditForm, FieldLabel, FormButton, FormField, HelpFoot};
use super::cells::parse_field;
use super::codec::{is_within, parse_text, set_path};
use super::domain::{FieldKind, FieldSpec};
use super::draft::{Draft, DraftEditor};
use super::editing::{self, EditSession, SessionFocus};
use super::field::{self, Fields, FormTree};
use super::group;
use super::select::{PickChanged, Select};
use super::table::Row;
use super::trigger;
use crate::commands::tui::pane::Framed;
use crate::commands::tui::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<EditSession>();
    app.add_systems(
        Update,
        (editing::sync_item_form, seed_fields)
            .chain()
            .in_set(super::EditSystems::Seed),
    );
    app.add_systems(
        Update,
        (mark_issues, show_help, field::light_brackets).in_set(Repainted),
    );
    app.add_systems(Update, field::place_brackets.after(trigger::place_slots));
    app.add_systems(
        Update,
        (
            group::place_dependents,
            group::place_stops
                .after(group::place_dependents)
                .after(trigger::place_slots),
            group::offer_unused.before(super::select::sync_selects),
        )
            .in_set(super::EditSystems::Place),
    );
    app.add_observer(handle_text_change);
    app.add_observer(handle_text_submit);
    app.add_observer(handle_check_change);
    app.add_observer(handle_pick_change);
    app.add_observer(handle_slider_change);
    app.add_observer(handle_modal_dismiss);
}

/// A press outside the form leaves the open item, which is what
/// plurimus's modal guard reports.
fn handle_modal_dismiss(
    dismiss: On<ModalDismiss>,
    forms: Query<(), With<EditForm>>,
    mut state: SessionFocus,
) {
    if forms.contains(dismiss.entity) {
        state.leave();
    }
}

#[derive(SystemParam)]
pub struct FormTargets<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    forms: Query<'w, 's, &'static EditForm>,
}

impl FormTargets<'_, '_> {
    /// The form `target` is, or sits in.
    fn owner(&self, target: Entity) -> Option<(Entity, &EditForm)> {
        std::iter::once(target)
            .chain(self.parents.iter_ancestors(target))
            .find_map(|entity| Some((entity, self.forms.get(entity).ok()?)))
    }

    pub fn form_of(&self, target: Entity) -> Option<Entity> {
        self.owner(target).map(|(form, _)| form)
    }

    /// The field `target` edits, or is part of: a menu's rows sit under
    /// the select that opened it.
    fn field<'a>(&self, target: Entity, fields: &'a Query<&FormField>) -> Option<&'a FormField> {
        std::iter::once(target)
            .chain(self.parents.iter_ancestors(target))
            .find_map(|entity| fields.get(entity).ok())
    }
}

#[derive(SystemParam)]
struct FormIssues<'w, 's> {
    draft: Res<'w, Draft>,
    session: Res<'w, EditSession>,
    targets: FormTargets<'w, 's>,
}

impl FormIssues<'_, '_> {
    /// The form an item is open in, and the item. It moves far less often
    /// than the session, which every keystroke in a field marks changed.
    fn shown(&self) -> Option<(Entity, Option<Row>)> {
        let editing = self.session.0.as_ref()?;
        Some((editing.form?, editing.slot.row()))
    }

    /// The issue among `located` against the field `spec` of the item
    /// `form` shows. A table's form showing no item of the plan has none.
    fn of<'a>(
        &self,
        located: &[LocatedIssue<'a>],
        form: Entity,
        spec: &FieldSpec,
    ) -> Option<&'a str> {
        if located.is_empty() {
            return None;
        }
        let ops = self.targets.forms.get(form).ok()?.ops;
        let index = match (ops.list, self.shown()) {
            (None, _) => None,
            (Some(_), Some((shown, Some(Row(index))))) if shown == form => Some(index),
            (Some(_), _) => return None,
        };
        let item = (ops.item)(&self.draft, index.unwrap_or(0));
        super::field_issue(located, (ops, index), spec, item.as_ref())
    }
}

const ISSUE_MARK: &str = " !";

fn mark_issues(
    issues: FormIssues,
    theme: Res<Theme>,
    mut labels: Query<(Entity, Mut<FieldLabel>, &mut UiWidget)>,
    mut last_shown: Local<Option<(Entity, Option<Row>)>>,
) {
    let shown = issues.shown();
    // A form standing over its table is spawned after the session that
    // opened it changed, so its labels are new when nothing else is.
    let is_stale = issues.draft.is_changed()
        || theme.is_changed()
        || *last_shown != shown
        || labels.iter().any(|(_, label, _)| label.is_added());
    if !is_stale {
        return;
    }
    *last_shown = shown;
    let located = super::located_issues(&issues.draft);
    for (entity, mut label, mut widget) in &mut labels {
        let form = issues.targets.owner(entity).map(|(form, _)| form);
        let is_marked = form.is_some_and(|form| issues.of(&located, form, &label.spec).is_some());
        if is_marked == label.is_marked && !theme.is_changed() {
            continue;
        }
        label.is_marked = is_marked;
        *widget = UiWidget::new(if is_marked {
            Paragraph::new(format!("{}{ISSUE_MARK}", label.spec.label)).style(theme.exceeded())
        } else {
            Paragraph::new(label.spec.label)
        });
    }
}

/// A form says under its fields what the one holding the keyboard means,
/// or what the draft holds against it. A form the keyboard is not in says
/// nothing.
fn show_help(
    issues: FormIssues,
    focus: Res<InputFocus>,
    theme: Res<Theme>,
    fields: Query<&FormField>,
    mut feet: Query<(&ChildOf, &mut UiWidget), With<HelpFoot>>,
) {
    if !issues.draft.is_changed() && !focus.is_changed() && !theme.is_changed() {
        return;
    }
    let targets = &issues.targets;
    let focused = focus.get();
    let form = focused.and_then(|widget| targets.owner(widget));
    let field = focused.and_then(|widget| targets.field(widget, &fields));
    let located = super::located_issues(&issues.draft);
    for (parent, mut widget) in &mut feet {
        let here = field.filter(|_| form.is_some_and(|(form, _)| form == parent.parent()));
        let issue = here.and_then(|field| issues.of(&located, parent.parent(), &field.spec));
        let (said, style) = match (issue, here) {
            (Some(issue), _) => (issue, theme.exceeded()),
            (None, Some(field)) => (field.spec.help, theme.dimmed()),
            (None, None) => ("", theme.dimmed()),
        };
        let text = Paragraph::new(said.to_owned())
            .style(style)
            .wrap(Wrap { trim: true });
        *widget = UiWidget::new(text);
    }
}

/// The widgets an edit landed from.
#[derive(SystemParam)]
pub struct Items<'w, 's> {
    fields: Query<'w, 's, (&'static FormField, &'static ChildOf)>,
    texts: Query<'w, 's, (), With<TextInput>>,
    parts: Query<
        'w,
        's,
        (
            Entity,
            &'static FormField,
            Option<&'static TextInput>,
            Option<&'static Select>,
        ),
    >,
    targets: FormTargets<'w, 's>,
    rows: Query<'w, 's, &'static Children>,
    slots: trigger::Slots<'w, 's>,
}

impl Items<'_, '_> {
    /// Writes the field `widget` edits into the item's snapshot.
    fn set(&self, widget: Entity, value: Option<Value>, state: &mut SessionFocus) {
        let (Ok((field, row)), Some(editing)) = (self.fields.get(widget), state.session.0.as_mut())
        else {
            return;
        };
        let key = field.spec.key;
        // A trigger is composed from all of its row's widgets rather than
        // the one that changed, and a list from every row that holds it.
        let (value, complaint) = match field.spec.kind {
            FieldKind::Trigger => {
                let group = self
                    .rows
                    .get(row.parent())
                    .map_or(&[][..], |group| &group[..]);
                trigger::held(group, &self.slots)
            }
            FieldKind::Listed(_) => group::listed(self.parts(widget, key)),
            FieldKind::Order(..) => group::ordered(self.parts(widget, key)),
            FieldKind::Presence(ticked) => {
                // What the table's own rows show is what it held before.
                editing.is_seeded = false;
                editing.incomplete.retain(|held, _| !is_within(held, key));
                let is_ticked = value.as_ref().and_then(Value::as_bool) == Some(true);
                let table = is_ticked.then(|| Value::Table(ticked.parse().unwrap_or_default()));
                (table, None)
            }
            _ => (value, None),
        };
        match complaint {
            Some(complaint) => editing.incomplete.insert(key, complaint),
            None => editing.incomplete.remove(key),
        };
        set_path(&mut editing.snapshot, key, value);
    }

    /// What each row holding a part of the list `key` holds, in the form
    /// `widget` sits in, beside the place its kind gives it in the list.
    fn parts(&self, widget: Entity, key: &str) -> impl Iterator<Item = (usize, Option<Value>)> {
        let form = self.targets.form_of(widget);
        self.parts
            .iter()
            .filter_map(move |(entity, field, text, select)| {
                let (FieldKind::Listed(place) | FieldKind::Order(_, place)) = field.spec.kind
                else {
                    return None;
                };
                let is_part = field.spec.key == key && self.targets.form_of(entity) == form;
                let value = match (text, select) {
                    (Some(text), _) => parse_field(field.spec.kind, text.value()),
                    (_, select) => select.and_then(Select::value),
                };
                is_part.then_some((place, value))
            })
    }

    fn read(&self, widget: Entity, text: &str) -> Option<Value> {
        let kind = self.fields.get(widget).map(|(field, ..)| field.spec.kind);
        kind.map_or_else(|_| parse_text(text), |kind| parse_field(kind, text))
    }

    /// The key of the field `widget` edits, which a refusal is reported
    /// under.
    fn key_of(&self, widget: Entity) -> Option<&'static str> {
        self.fields
            .get(widget)
            .ok()
            .map(|(field, ..)| field.spec.key)
    }
}

/// An item newly in the session fills its form's widgets and names the
/// form; an item opened takes the keyboard to the first of them.
fn seed_fields(
    mut session: ResMut<EditSession>,
    draft: Res<Draft>,
    mut fields: Fields,
    mut frames: Query<&mut Framed>,
    mut focus: ResMut<InputFocus>,
) {
    let is_edited = session.is_changed();
    let Some(editing) = session.bypass_change_detection().0.as_mut() else {
        return;
    };
    let Some(form) = editing.form else {
        return;
    };
    if editing.is_seeded && is_edited {
        fields.show_rates(form, editing, focus.get());
    }
    let mut is_hidden = false;
    if editing.is_seeded && focus.is_changed() {
        // What a field reads as depends on whether the keyboard is in it.
        fields.show_texts(form, editing, focus.get());
        let held = focus.get().and_then(|widget| fields.tree.key_of(widget));
        is_hidden = editing.hide_cleared(held);
    }
    if !editing.is_seeded {
        editing.is_seeded = true;
        fields.show(form, editing, &draft.plan, focus.get());
        if let Ok(mut framed) = frames.get_mut(form) {
            Framed::retitle(&mut framed, &editing.title());
        }
    }
    // A form standing alone rests on itself until ⏎ goes into its fields,
    // so the shell's keys reach.
    if std::mem::take(&mut editing.takes_keyboard) {
        let held = if editing.is_alone {
            Some(form)
        } else {
            fields.tree.first(form)
        };
        if let Some(held) = held {
            focus.set(held, FocusCause::Navigated);
        }
    }
    if is_hidden {
        session.set_changed();
    }
}

fn handle_text_change(change: On<ValueChange<String>>, items: Items, mut state: SessionFocus) {
    items.set(
        change.source,
        items.read(change.source, &change.value),
        &mut state,
    );
}

/// Submitting a field applies the whole item. It is the field's own event
/// rather than a final change, which the blur that closing causes reports
/// as well.
fn handle_text_submit(
    submit: On<Submit>,
    items: Items,
    mut state: SessionFocus,
    mut editor: DraftEditor,
) {
    items.set(
        submit.entity,
        items.read(submit.entity, &submit.value),
        &mut state,
    );
    state.apply(items.key_of(submit.entity), &mut editor);
}

/// A flag's box reports the state it moved to.
fn handle_check_change(change: On<ValueChange<bool>>, items: Items, mut state: SessionFocus) {
    items.set(
        change.source,
        Some(Value::Boolean(change.value)),
        &mut state,
    );
}

fn handle_pick_change(change: On<PickChanged>, items: Items, mut state: SessionFocus) {
    items.set(change.entity, change.value.clone(), &mut state);
}

fn handle_slider_change(change: On<ValueChange<f32>>, items: Items, mut state: SessionFocus) {
    let value = super::field::slider_value(change.value);
    items.set(change.source, Some(value), &mut state);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormKey {
    Enter,
    Leave,
    /// Into the fields from the form itself; in a field it is the tab
    /// navigation's, and left alone.
    Into,
}

const FORM_KEYS: &[(KeyBinding, FormKey)] = &[
    (KeyBinding::new(Key::Enter), FormKey::Enter),
    (KeyBinding::new(Key::Escape), FormKey::Leave),
    (KeyBinding::new(Key::Tab), FormKey::Into),
];

/// Esc leaves the item. Enter in a field that does not take the key
/// itself - a pick, a slider, a box - applies it, and on the form itself
/// goes into the fields. Observed on the form, which the focused widget
/// bubbles to, so the widget is the event's original target. The new
/// plan's form stands alone while the shell holds no document, with
/// nothing to leave it for.
pub fn handle_form_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    (targets, items, tree, mut commands): (FormTargets, Items, FormTree, Commands),
    held: HeldModifiers,
    mut state: SessionFocus,
    mut editor: DraftEditor,
) {
    let target = input.original_event_target();
    let Some(form) = targets.form_of(target) else {
        return;
    };
    let Some(key) = first_bound(FORM_KEYS, &input.input, held.get()) else {
        return;
    };
    if key == FormKey::Into && form != target {
        return;
    }
    input.propagate(false);
    let is_alone = state.session.0.as_ref().is_some_and(|held| held.is_alone);
    match key {
        FormKey::Enter | FormKey::Into if form == target => {
            if let Some(first) = tree.first(form) {
                state.enter(first);
            }
        }
        FormKey::Enter if !items.texts.contains(target) => {
            act(
                FormButton::Apply,
                items.key_of(target),
                &mut state,
                &mut editor,
                &mut commands,
            );
        }
        FormKey::Enter | FormKey::Into => {}
        // Nothing to leave the form for: the keyboard rests on it, where
        // the shell's keys reach.
        FormKey::Leave if is_alone => state.focus(form),
        FormKey::Leave => state.leave(),
    }
}

/// Observed on the button rather than on the form, because `Activate`
/// lands on the button and does not bubble.
pub fn handle_button(
    activate: On<Activate>,
    buttons: Query<&FormButton>,
    mut state: SessionFocus,
    mut editor: DraftEditor,
    mut commands: Commands,
) {
    if let Ok(&which) = buttons.get(activate.entity) {
        act(which, None, &mut state, &mut editor, &mut commands);
    }
}

/// Does what a form's button does, then what the form does once it has:
/// nothing, for all but a tool that acts on its answers at once.
fn act(
    which: FormButton,
    label: Option<&str>,
    state: &mut SessionFocus,
    editor: &mut DraftEditor,
    commands: &mut Commands,
) {
    let after = state.session.0.as_ref().and_then(|held| held.ops.after);
    let is_done = match which {
        FormButton::Apply => state.apply(label, editor),
        FormButton::Discard => {
            state.discard();
            true
        }
    };
    if is_done && let Some(after) = after {
        after(which, commands);
    }
}
