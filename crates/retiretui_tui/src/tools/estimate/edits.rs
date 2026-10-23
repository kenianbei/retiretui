//! What the Spending Estimate page's commands do: an export read or
//! dropped, a row or the plan's `[estimate]` table opened in its form and
//! applied, a named payee remembered, and the proposals taken or written.

use std::path::{Path, PathBuf};

use bevy_ecs::prelude::{Commands, In, Query, Res, ResMut, With, World};
use plurimus::widgets::ActiveDescendant;
use retiretui_client::estimate::{
    self, Applied, MoveAnswers, NEW_FIELDS, NewAnswers, PAYEE_FIELDS, PayeeAnswers, RowForm,
    SET_FIELDS, SETTINGS_FIELDS, SetAnswers, SettingsAnswers, assign, settings_table,
    take_question, take_settings, taken, with_payee,
};
use retiretui_client::present::counted;
use retiretui_engine::estimate::{Category, Estimate, apply};

use super::{Cursor, FileList, FileRow, Held, PatternsFile};
use crate::command::Outcome;
use crate::confirm::{Answer, Confirm};
use crate::documents::{Browsing, Pickers};
use crate::edit::{self, Draft, DraftEditor, FormButton, Ops, Row, Slot};
use crate::journal;
use crate::nav::Page;
use crate::session::{self, Session};
use crate::tools::write;

const ACTIONS: [&str; 2] = ["Discard", "Apply"];
const PAGE: Option<Page> = Some(Page::SpendingEstimate);
const SET_OPS: Ops =
    Ops::tool::<SetAnswers>(PAGE, RowForm::Set.title(), SET_FIELDS).acting(ACTIONS, after_row);
const NEW_OPS: Ops =
    Ops::tool::<NewAnswers>(PAGE, RowForm::New.title(), NEW_FIELDS).acting(ACTIONS, after_row);
const MOVE_OPS: Ops = Ops::tool::<MoveAnswers>(PAGE, RowForm::Move.title(), RowForm::Move.fields())
    .acting(ACTIONS, after_row);
const PAYEE_OPS: Ops = Ops::tool::<PayeeAnswers>(PAGE, RowForm::Payee.title(), PAYEE_FIELDS)
    .acting(ACTIONS, after_row);
const SETTINGS_OPS: Ops =
    Ops::tool::<SettingsAnswers>(PAGE, "Settings", SETTINGS_FIELDS).acting(ACTIONS, after_settings);
const _: () = assert!(
    edit::help_fits(SET_FIELDS)
        && edit::help_fits(NEW_FIELDS)
        && edit::help_fits(RowForm::Move.fields())
        && edit::help_fits(PAYEE_FIELDS)
        && edit::help_fits(SETTINGS_FIELDS),
    "a field's help is missing or too long"
);

pub(super) const NO_EXPORTS: &str = "no export to drop";
pub(super) const NO_PROPOSALS: &str = "nothing proposed yet";
const NOTHING_TO_EDIT: &str = "nothing to edit on this row";

/// The `add-export` command: asks which export to read.
pub fn add_picker(pickers: Res<Pickers>, mut browsing: ResMut<Browsing>) -> Outcome {
    browsing.open(pickers.export);
    Outcome::Done
}

/// Reads the export at `path` into the session, in place of one of the
/// same name.
pub fn add_export(In(path): In<PathBuf>, session: Res<Session>, mut held: ResMut<Held>) {
    let name = session::file_name(&path).into_owned();
    let read = session
        .store
        .read(&path)
        .map_err(|error| format!("{}: {error}", path.display()))
        .and_then(|text| held.0.add(&name, &text).map_err(|error| error.to_string()));
    match read {
        Ok(count) => journal::say(format!(
            "read {} from {name}",
            counted(count, "row", "rows")
        )),
        Err(reason) => journal::warn(format!("not read: {reason}")),
    }
}

/// The `remove-export` command: drops the export under the files' cursor.
pub fn remove_export(
    lists: Query<&ActiveDescendant, With<FileList>>,
    rows: Query<&FileRow>,
    mut held: ResMut<Held>,
) -> Outcome {
    let at = lists
        .iter()
        .find_map(|on| on.0.and_then(|row| rows.get(row).ok()))
        .map(|row| row.0);
    let listed = at.and_then(|at| held.0.exports().listed().nth(at));
    let Some(name) = listed.map(|(name, _)| name.to_owned()) else {
        return Outcome::Refused(NO_EXPORTS.to_owned());
    };
    held.0.remove(&name);
    journal::say(format!("dropped {name}"));
    Outcome::Done
}

/// The `edit-proposal` command: opens the highlighted row in its form.
pub fn edit_row(
    (held, cursor): (Res<Held>, Res<Cursor>),
    mut draft: ResMut<Draft>,
    mut commands: Commands,
) -> Outcome {
    let Some((form, answers)) = held.0.form(cursor.0) else {
        return Outcome::Refused(NOTHING_TO_EDIT.to_owned());
    };
    let ops = match form {
        RowForm::Set => {
            draft.set_answers::<SetAnswers>(answers);
            SET_OPS
        }
        RowForm::New => {
            draft.set_answers::<NewAnswers>(answers);
            NEW_OPS
        }
        RowForm::Move => {
            draft.set_answers::<MoveAnswers>(answers);
            MOVE_OPS
        }
        RowForm::Payee => {
            draft.set_answers::<PayeeAnswers>(answers);
            PAYEE_OPS
        }
    };
    commands.run_system_cached_with(edit::open_item, (ops, None, Slot::At(Row(0))));
    Outcome::Done
}

fn after_row(button: FormButton, commands: &mut Commands) {
    if matches!(button, FormButton::Apply) {
        commands.run_system_cached(apply_row);
    }
}

/// Writes the form's answers into the highlighted row: the proposal, the
/// plan's assignment of a category, or a payee's category, remembered
/// where the launch names a patterns file.
pub(super) fn apply_row(
    (cursor, memory, session): (Res<Cursor>, Res<PatternsFile>, Res<Session>),
    mut held: ResMut<Held>,
    mut editor: DraftEditor,
) {
    match held.0.apply(cursor.0, &editor.draft) {
        Err(reason) => journal::warn(reason),
        Ok(Applied::Held) => {}
        Ok(Applied::Moved { category, expense }) => {
            if let Some(refusal) = editor.draft.refuse_if_read_only() {
                journal::warn(refusal);
                return;
            }
            assign(&mut editor.draft.plan, category, expense);
            editor.commit();
        }
        Ok(Applied::Named { payee, category }) => {
            if let Some(path) = &memory.0 {
                remember(&session, path, &payee, category);
            }
        }
    }
}

/// Appends `payee`, named for `category`, to the user's patterns file.
fn remember(session: &Session, path: &Path, payee: &str, category: Category) {
    let text = session.store.read(path).ok();
    let written = with_payee(text.as_deref(), payee, category).and_then(|file| {
        if let Some(directory) = path.parent() {
            session
                .store
                .create_dir_all(directory)
                .map_err(|error| error.to_string())?;
        }
        session
            .store
            .write(path, &file)
            .map_err(|error| error.to_string())
    });
    match written {
        Ok(()) => journal::say(format!("{payee} remembered in {}", path.display())),
        Err(reason) => journal::warn(format!("{payee} not remembered: {reason}")),
    }
}

/// The `edit-estimate-settings` command: opens the plan's `[estimate]`
/// table in its form.
pub fn edit_settings(mut draft: ResMut<Draft>, mut commands: Commands) -> Outcome {
    let table = settings_table(&draft.plan);
    draft.set_answers::<SettingsAnswers>(table);
    commands.run_system_cached_with(edit::open_item, (SETTINGS_OPS, None, Slot::At(Row(0))));
    Outcome::Done
}

fn after_settings(button: FormButton, commands: &mut Commands) {
    if matches!(button, FormButton::Apply) {
        commands.run_system_cached(apply_settings);
    }
}

/// Writes the settings form's answers into the plan as one step.
fn apply_settings(mut editor: DraftEditor) {
    if let Some(refusal) = editor.draft.refuse_if_read_only() {
        journal::warn(refusal);
        return;
    }
    let table = editor.draft.answers::<SettingsAnswers>();
    match take_settings(&mut editor.draft.plan, table) {
        Ok(()) => editor.commit(),
        Err(reason) => journal::warn(reason),
    }
}

/// The `take-estimate` command: asks before the proposals set the plan's
/// expenses. The answer carries the proposals asked about, so an estimate
/// that moves under the question changes nothing.
pub fn adopt(held: Res<Held>, draft: Res<Draft>, mut confirm: ResMut<Confirm>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let found = held.0.estimate();
    if found.proposals.is_empty() {
        return Outcome::Refused(NO_PROPOSALS.to_owned());
    }
    let asked = found.clone();
    let answers = vec![
        Answer::closing("Cancel"),
        Answer::running("Take", move |commands| {
            commands.run_system_cached_with(take, asked);
        })
        .primary(),
    ];
    confirm.ask_among(take_question(found), answers);
    Outcome::Done
}

fn take(In(estimate): In<Estimate>, mut editor: DraftEditor) {
    apply(&mut editor.draft.plan, &estimate);
    editor.commit();
    journal::say(taken(&estimate));
}

/// The `write-estimate` command: asks where to write the proposals applied.
pub fn write_picker(
    pickers: Res<Pickers>,
    held: Res<Held>,
    draft: Res<Draft>,
    mut browsing: ResMut<Browsing>,
) -> Outcome {
    if held.0.estimate().proposals.is_empty() {
        return Outcome::Refused(NO_PROPOSALS.to_owned());
    }
    write::open_picker(pickers.estimate, &draft, &mut browsing)
}

/// Writes the proposals applied at `path` as a scenario over the document,
/// and compares the file written.
pub fn write_overlay(In(path): In<PathBuf>, world: &mut World) {
    let text = write::base_of(world, &path).and_then(|base| {
        let found = world.resource::<Held>().0.estimate();
        let plan = &world.resource::<Draft>().plan;
        estimate::overlay(&base, plan, found).map_err(|error| format!("not written: {error}"))
    });
    write::write(world, path, text);
}
