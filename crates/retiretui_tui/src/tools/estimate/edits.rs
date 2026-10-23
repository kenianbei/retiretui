//! What the Spending Estimate page's commands do: an export read or
//! dropped, a row opened in its form and applied, a named payee
//! remembered, and the proposals taken or written.

use std::path::{Path, PathBuf};

use bevy_ecs::prelude::{Commands, In, Query, Res, ResMut, With, World};
use plurimus::widgets::ActiveDescendant;
use retiretui_client::estimate::{
    self, Action, Applied, Change, RowForm, SETTINGS_FORM, assign, take_question, taken,
};
use retiretui_client::present::counted;
use retiretui_engine::estimate::{Estimate, apply};

use super::{BehindTable, Cursor, FileList, FileRow, Held, PatternsFile, RecordRow, record_at};
use crate::command::Outcome;
use crate::confirm::{Answer, Confirm};
use crate::documents::{Browsing, Pickers};
use crate::edit::{self, Draft, DraftEditor, FormButton, Ops, Row, Slot};
use crate::journal;
use crate::nav::Page;
use crate::session::{self, Session};
use crate::settings;
use crate::tools::write;

const ACTIONS: [&str; 2] = ["Discard", "Apply"];
const NO_RECORD: &str = "no record highlighted";
const PAGE: Option<Page> = Some(Page::SpendingEstimate);

/// The form `row_form` is edited in on this page.
const fn row_ops(row_form: RowForm) -> Ops {
    Ops {
        form: row_form.form(),
        surface: PAGE,
        actions: ACTIONS,
        after: Some(after_row),
    }
}

/// The plan's `[estimate]` table, applied as a step of the draft's history.
pub(super) const SETTINGS_OPS: Ops = Ops {
    form: SETTINGS_FORM,
    surface: PAGE,
    actions: ACTIONS,
    after: None,
};
const _: () = assert!(
    edit::help_fits(RowForm::Set.form().fields)
        && edit::help_fits(RowForm::New.form().fields)
        && edit::help_fits(RowForm::Move.form().fields)
        && edit::help_fits(RowForm::Payee.form().fields)
        && edit::help_fits(SETTINGS_FORM.fields),
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
    let Some((row_form, answers)) = held.0.form(cursor.0) else {
        return Outcome::Refused(NOTHING_TO_EDIT.to_owned());
    };
    let ops = row_ops(row_form);
    if let Err(reason) = (ops.store)(&mut draft, 0, answers) {
        return Outcome::Refused(reason);
    }
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
        Ok(Applied::Changed(changes)) => remember(&session, memory.0.as_deref(), &changes),
    }
}

/// The `toggle-record` command: ignores the record under the Records
/// pane's cursor, or counts it again behind the Ignored row, remembered
/// where the launch names a patterns file.
pub fn toggle_record(
    (cursor, memory, session): (Res<Cursor>, Res<PatternsFile>, Res<Session>),
    (tables, records): (
        Query<&ActiveDescendant, With<BehindTable>>,
        Query<&RecordRow>,
    ),
    mut held: ResMut<Held>,
) -> Outcome {
    let Some(index) = record_at(&tables, &records) else {
        return Outcome::Refused(NO_RECORD.to_owned());
    };
    match held.0.act(cursor.0, Some(index), Action::IgnoreRecord) {
        Ok(change) => {
            remember(&session, memory.0.as_deref(), &[change]);
            Outcome::Done
        }
        Err(reason) => Outcome::Refused(reason),
    }
}

/// Makes `changes` to the user's patterns file, where the launch names
/// one, and says what was made.
pub(super) fn remember(session: &Session, path: Option<&Path>, changes: &[Change]) {
    let said: Vec<String> = changes.iter().map(Change::said).collect();
    let said = said.join("; ");
    let Some(path) = path else {
        journal::say(said);
        return;
    };
    let text = session.store.read(path).unwrap_or_default();
    let written = (changes.iter())
        .try_fold(text, |text, change| change.applied(&text))
        .and_then(|file| {
            settings::write(session.store.as_ref(), path, &file).map_err(|error| error.to_string())
        });
    match written {
        Ok(()) => journal::say(format!("{said}, kept in {}", path.display())),
        Err(reason) => journal::warn(format!("{said}, but not kept: {reason}")),
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
