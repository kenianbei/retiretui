//! What can be done for the person under the People cursor: each a
//! command of the table, run by its key or from the actions ⏎ offers.

use bevy_ecs::prelude::{In, Res, ResMut};
use retiretui_client::searches::claims::{
    self, NOBODY, clear_question, hold_said, remove_question,
};
use retiretui_engine::plan::{Item, Plan};

use super::people::{HeldClaims, PersonCursor};
use crate::command::Outcome;
use crate::confirm::Confirm;
use crate::documents::{Browsing, Pickers};
use crate::edit::{Draft, DraftEditor, Importing};
use crate::journal;

/// The `import-statement` command: asks which statement to record on the
/// person under the cursor.
pub fn import_statement(
    draft: Res<Draft>,
    cursor: Res<PersonCursor>,
    pickers: Res<Pickers>,
    mut browsing: ResMut<Browsing>,
    mut importing: ResMut<Importing>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(person) = cursor.person(&draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    importing.ask(person.id.clone(), &pickers, &mut browsing);
    Outcome::Done
}

/// The `fill-career` command: records a career at the salary the plan pays
/// the person under the cursor, where they have no record, as one step of
/// history.
pub fn fill_career(cursor: Res<PersonCursor>, mut editor: DraftEditor) -> Outcome {
    if let Some(refusal) = editor.draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    if let Some(issue) = editor.draft.issues().first() {
        return Outcome::Refused(format!("not filled: {issue}"));
    }
    let tables = editor.session.tables.clone();
    edit_person(&cursor, &mut editor, |plan, id| {
        claims::fill_career(plan, &tables, id)
    })
}

/// The `compute-benefit` command: drops the typed figure of the cursor's
/// person's `social-security` income, so it is computed from their record,
/// as one step of history.
pub fn compute_benefit(cursor: Res<PersonCursor>, mut editor: DraftEditor) -> Outcome {
    edit_person(&cursor, &mut editor, claims::compute_benefit)
}

/// Makes `edit` to the cursor's person as one step of history, saying what
/// it did; an edit refused changes nothing.
fn edit_person(
    cursor: &PersonCursor,
    editor: &mut DraftEditor,
    edit: impl FnOnce(&mut Plan, &str) -> Result<String, String>,
) -> Outcome {
    if let Some(refusal) = editor.draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(id) = cursor
        .person(&editor.draft.plan)
        .map(|person| person.id.clone())
    else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    match edit(&mut editor.draft.plan, &id) {
        Ok(said) => {
            editor.commit();
            journal::say(said);
            Outcome::Done
        }
        Err(refusal) => Outcome::Refused(refusal),
    }
}

/// The `clear-record` command: asks before emptying the cursor's person's
/// earnings record.
pub fn clear_record(
    cursor: Res<PersonCursor>,
    draft: Res<Draft>,
    mut confirm: ResMut<Confirm>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(person) = cursor.person(&draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    if person.earnings.is_empty() {
        return Outcome::Refused(format!("{} has no earnings record", person.display_name()));
    }
    let id = person.id.clone();
    confirm.ask(
        clear_question(person.display_name()),
        "Clear",
        move |commands| {
            commands.run_system_cached_with(clear_now, id);
        },
    );
    Outcome::Done
}

/// Empties `id`'s record, as one step of history.
fn clear_now(In(id): In<String>, editor: DraftEditor) {
    edit_now(&id, editor, claims::clear_record);
}

/// The `remove-benefit` command: asks before taking the cursor's person's
/// `social-security` income out of the plan.
pub fn remove_benefit(
    cursor: Res<PersonCursor>,
    draft: Res<Draft>,
    mut confirm: ResMut<Confirm>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(person) = cursor.person(&draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    let name = person.display_name();
    if claims::benefit(&draft.plan, &person.id).is_none() {
        return Outcome::Refused(format!("{name} has no Social Security income"));
    }
    let id = person.id.clone();
    confirm.ask(remove_question(name), "Remove", move |commands| {
        commands.run_system_cached_with(remove_now, id);
    });
    Outcome::Done
}

/// Takes `id`'s `social-security` income out, as one step of history.
fn remove_now(In(id): In<String>, editor: DraftEditor) {
    edit_now(&id, editor, claims::remove_benefit);
}

/// Makes `edit` to `id` once asked, as one step of history; what the
/// question was about may have gone meanwhile.
fn edit_now(
    id: &str,
    mut editor: DraftEditor,
    edit: fn(&mut Plan, &str) -> Result<String, String>,
) {
    if let Ok(said) = edit(&mut editor.draft.plan, id) {
        editor.commit();
        journal::say(said);
    }
}

/// The `hold-claim` command: keeps the cursor's person's claim as the plan
/// states it while the others' are searched, or lets it be searched again.
pub fn hold_claim(
    cursor: Res<PersonCursor>,
    draft: Res<Draft>,
    mut held: ResMut<HeldClaims>,
) -> Outcome {
    let Some(person) = cursor.person(&draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    let is_held = !held.0.remove(&person.id);
    if is_held {
        held.0.insert(person.id.clone());
    }
    journal::say(hold_said(person.display_name(), is_held));
    Outcome::Done
}
