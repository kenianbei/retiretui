//! What can be done for the person under the People cursor: each a
//! command of the table, run by its key or from the actions ⏎ offers.

use bevy_ecs::prelude::{In, Res, ResMut};
use retiretui_engine::optimize::career_at_salary;

use super::people::{HeldClaims, NOBODY, PersonCursor, benefit};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::confirm::Confirm;
use crate::commands::tui::documents::{Browsing, Pickers};
use crate::commands::tui::edit::{Draft, DraftEditor, Importing};
use crate::commands::tui::journal;

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
        return refusal;
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
        return refusal;
    }
    if let Some(issue) = editor.draft.issues().first() {
        return Outcome::Refused(format!("not filled: {issue}"));
    }
    let Some(at) = cursor.index(&editor.draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    let person = &editor.draft.plan.household.people[at];
    let id = person.id.clone();
    let name = person.display_name().to_owned();
    if !person.earnings.is_empty() {
        return Outcome::Refused(format!(
            "{name} has an earnings record; a statement replaces it"
        ));
    }
    let career = match career_at_salary(&editor.draft.plan, &editor.session.tables, &id) {
        Ok(career) => career,
        Err(reason) => return Outcome::Refused(reason),
    };
    let years = career.len();
    editor.draft.plan.household.people[at].earnings = career;
    editor.commit();
    journal::say(format!(
        "estimated {years} year(s) of earnings for {name} from a career at their salary"
    ));
    Outcome::Done
}

/// The `compute-benefit` command: drops the typed figure of the cursor's
/// person's `social-security` income, so it is computed from their record,
/// as one step of history.
pub fn compute_benefit(cursor: Res<PersonCursor>, mut editor: DraftEditor) -> Outcome {
    if let Some(refusal) = editor.draft.refuse_if_read_only() {
        return refusal;
    }
    let Some((id, name)) = cursor
        .person(&editor.draft.plan)
        .map(|person| (person.id.clone(), person.display_name().to_owned()))
    else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    let typed = (editor.draft.plan.income.iter())
        .position(|income| income.is_benefit_of(&id) && income.amount.is_some());
    let Some(at) = typed else {
        return Outcome::Refused(format!("{name} has no typed benefit to compute"));
    };
    editor.draft.plan.income[at].amount = None;
    editor.commit();
    journal::say(format!("{name}'s benefit is computed from their record"));
    Outcome::Done
}

/// The `clear-record` command: asks before emptying the cursor's person's
/// earnings record.
pub fn clear_record(
    cursor: Res<PersonCursor>,
    draft: Res<Draft>,
    mut confirm: ResMut<Confirm>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return refusal;
    }
    let Some(person) = cursor.person(&draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    let id = person.id.clone();
    let name = person.display_name().to_owned();
    if person.earnings.is_empty() {
        return Outcome::Refused(format!("{name} has no earnings record"));
    }
    let question = format!("Clear {name}'s earnings record?");
    confirm.ask(question, "Clear", move |commands| {
        commands.run_system_cached_with(clear_now, id);
    });
    Outcome::Done
}

/// Empties `id`'s record, as one step of history.
fn clear_now(In(id): In<String>, mut editor: DraftEditor) {
    let name = editor.draft.plan.person_name(&id).to_owned();
    let people = &editor.draft.plan.household.people;
    let Some(at) = people.iter().position(|person| person.id == id) else {
        return;
    };
    editor.draft.plan.household.people[at].earnings.clear();
    editor.commit();
    journal::say(format!("cleared {name}'s earnings record"));
}

/// The `remove-benefit` command: asks before taking the cursor's person's
/// `social-security` income out of the plan.
pub fn remove_benefit(
    cursor: Res<PersonCursor>,
    draft: Res<Draft>,
    mut confirm: ResMut<Confirm>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return refusal;
    }
    let Some(person) = cursor.person(&draft.plan) else {
        return Outcome::Refused(NOBODY.to_owned());
    };
    let id = person.id.clone();
    let name = person.display_name().to_owned();
    if benefit(&draft.plan, &id).is_none() {
        return Outcome::Refused(format!("{name} has no Social Security income"));
    }
    let question = format!("Remove {name}'s Social Security income?");
    confirm.ask(question, "Remove", move |commands| {
        commands.run_system_cached_with(remove_now, id);
    });
    Outcome::Done
}

/// Takes `id`'s `social-security` income out, as one step of history.
fn remove_now(In(id): In<String>, mut editor: DraftEditor) {
    let name = editor.draft.plan.person_name(&id).to_owned();
    let incomes = &editor.draft.plan.income;
    let Some(at) = incomes.iter().position(|income| income.is_benefit_of(&id)) else {
        return;
    };
    editor.draft.plan.income.remove(at);
    editor.commit();
    journal::say(format!("removed {name}'s Social Security income"));
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
    let id = person.id.clone();
    let name = person.display_name().to_owned();
    if held.0.remove(&id) {
        journal::say(format!("{name}'s claim is searched again"));
    } else {
        journal::say(format!("{name}'s claim is held as the plan states it"));
        held.0.insert(id);
    }
    Outcome::Done
}
