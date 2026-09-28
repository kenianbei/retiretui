//! What can be done for the person under the People cursor: each a
//! command of the table, run by its key or from the actions ⏎ offers.

use bevy_ecs::prelude::{In, Res, ResMut};
use retiretui_client::searches::claims::{NOBODY, PersonAction, hold_said};
use retiretui_engine::plan::Item;

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
    edit_person(&cursor, &mut editor, PersonAction::FillCareer)
}

/// The `compute-benefit` command: drops the typed figure of the cursor's
/// person's `social-security` income, so it is computed from their record,
/// as one step of history.
pub fn compute_benefit(cursor: Res<PersonCursor>, mut editor: DraftEditor) -> Outcome {
    edit_person(&cursor, &mut editor, PersonAction::ComputeBenefit)
}

/// Makes `action`'s edit to the cursor's person as one step of history,
/// saying what it did; an edit refused changes nothing.
fn edit_person(cursor: &PersonCursor, editor: &mut DraftEditor, action: PersonAction) -> Outcome {
    if let Some(refusal) = editor.draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(id) = cursor
        .person(&editor.draft.plan)
        .map(|person| person.id.clone())
    else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    match commit_said(editor, action, &id) {
        Some(Err(refusal)) => Outcome::Refused(refusal),
        _ => Outcome::Done,
    }
}

/// Makes `action`'s edit to `id`, committing it and saying what it did
/// where it was made.
fn commit_said(
    editor: &mut DraftEditor,
    action: PersonAction,
    id: &str,
) -> Option<Result<String, String>> {
    let answer = action.apply(&mut editor.draft.plan, &editor.session.tables, id)?;
    if let Ok(said) = &answer {
        editor.commit();
        journal::say(said.clone());
    }
    Some(answer)
}

/// The `clear-record` command: asks before emptying the cursor's person's
/// earnings record.
pub fn clear_record(
    cursor: Res<PersonCursor>,
    draft: Res<Draft>,
    confirm: ResMut<Confirm>,
) -> Outcome {
    let refusal = |name: &str| format!("{name} has no earnings record");
    ask_about(
        PersonAction::ClearRecord,
        (&cursor, &draft),
        confirm,
        refusal,
    )
}

/// The `remove-benefit` command: asks before taking the cursor's person's
/// `social-security` income out of the plan.
pub fn remove_benefit(
    cursor: Res<PersonCursor>,
    draft: Res<Draft>,
    confirm: ResMut<Confirm>,
) -> Outcome {
    let refusal = |name: &str| format!("{name} has no Social Security income");
    ask_about(
        PersonAction::RemoveBenefit,
        (&cursor, &draft),
        confirm,
        refusal,
    )
}

/// Asks before `action`'s edit to the cursor's person, refused with
/// `refusal` where they have no use for it.
fn ask_about(
    action: PersonAction,
    (cursor, draft): (&PersonCursor, &Draft),
    mut confirm: ResMut<Confirm>,
    refusal: impl Fn(&str) -> String,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(person) = cursor.person(&draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    let name = person.display_name();
    let (Some(question), Some(answer)) = (action.question(name), action.answer()) else {
        return Outcome::Done;
    };
    if !action.is_offered(&draft.plan, person, false) {
        return Outcome::Refused(refusal(name));
    }
    let id = person.id.clone();
    confirm.ask(question, answer, move |commands| {
        commands.run_system_cached_with(edit_now, (action, id));
    });
    Outcome::Done
}

/// Makes `action`'s edit to `id` once asked, as one step of history; what
/// the question was about may have gone meanwhile.
fn edit_now(In((action, id)): In<(PersonAction, String)>, mut editor: DraftEditor) {
    let _ = commit_said(&mut editor, action, &id);
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
