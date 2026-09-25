use std::path::{Path, PathBuf};

use bevy_ecs::prelude::{Commands, Entity, In, Query, Res, ResMut, Resource};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::InputFocus;
use plurimus::widgets::ActiveDescendant;
use retiretui_engine::plan::Plan;
use retiretui_engine::statement;
use toml::Table;

use super::codec::to_text;
use super::domain::Domain;
use super::draft::{Draft, DraftEditor};
use super::editing::{self, EditSession, Slot};
use super::household::People;
use super::offers::display_name;
use super::table::{DomainTable, Row, cursor_row};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::confirm::Confirm;
use crate::commands::tui::documents::{Browsing, Pickers};
use crate::commands::tui::journal;

const NO_TABLE: &str = "nothing to add or delete here";
const NOT_A_PERSON: &str = "earnings are imported from the People page";
const NOBODY_HIGHLIGHTED: &str = "nobody highlighted to import onto";

/// The person a statement is being picked for, held from the command to
/// the pick so the cursor moving under the picker changes nothing.
#[derive(Resource, Default)]
pub struct Importing(Option<String>);

impl Importing {
    /// Asks which statement to record on `person`.
    pub fn ask(&mut self, person: String, pickers: &Pickers, browsing: &mut Browsing) {
        self.0 = Some(person);
        browsing.open(pickers.earnings);
    }

    #[cfg(test)]
    pub fn asked_for(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// The table the keyboard is acting on: the focused one, or the one whose
/// item is open.
#[derive(SystemParam)]
pub struct FocusedTable<'w, 's> {
    focus: Res<'w, InputFocus>,
    session: Res<'w, EditSession>,
    pub(super) tables: Query<'w, 's, (&'static mut DomainTable, &'static ActiveDescendant)>,
    rows: Query<'w, 's, &'static Row>,
}

impl FocusedTable<'_, '_> {
    /// The acting table, and its cursor.
    pub(super) fn acting(&self) -> Option<(Entity, &DomainTable, ActiveDescendant)> {
        let entity = self.session.editing_table().or_else(|| self.focus.get())?;
        let (table, cursor) = self.tables.get(entity).ok()?;
        Some((entity, table, *cursor))
    }

    /// The acting table and the item its cursor is on, where it is on one:
    /// an empty domain draws a line no item stands behind.
    fn cursor(&self) -> Option<(Entity, &DomainTable, usize)> {
        let (entity, table, cursor) = self.acting()?;
        let Row(index) = cursor_row(cursor, &self.rows)?;
        Some((entity, table, index))
    }
}

pub fn add(focused: FocusedTable, mut commands: Commands) -> Outcome {
    let Some((entity, table, _)) = focused.acting() else {
        return Outcome::Refused(NO_TABLE.to_owned());
    };
    commands.run_system_cached_with(
        editing::open_item,
        (table.ops, Some(entity), Slot::New(table.list)),
    );
    Outcome::Done
}

/// Asks before deleting the highlighted item. The answer carries the
/// item's name as it stood, so a table that changed under the question is
/// not deleted from.
pub fn delete(focused: FocusedTable, draft: Res<Draft>, mut confirm: ResMut<Confirm>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return refusal;
    }
    if focused.acting().is_none() {
        return Outcome::Refused(NO_TABLE.to_owned());
    }
    let Some((entity, table, index)) = focused.cursor() else {
        return Outcome::Refused("nothing highlighted to delete".to_owned());
    };
    let item = (table.ops.item)(&draft, index);
    let name = item_name(table, item.as_ref());
    let shown = item
        .and_then(|item| display_name(&item, table.list.identity, table.ops.fields))
        .unwrap_or_default();
    confirm.ask(format!("Delete {shown}?"), "Delete", move |commands| {
        commands.run_system_cached_with(remove_item, (entity, index, name));
    });
    Outcome::Done
}

fn remove_item(
    In((entity, index, asked_about)): In<(Entity, usize, String)>,
    mut tables: Query<&mut DomainTable>,
    mut editor: DraftEditor,
) {
    let Ok(mut table) = tables.get_mut(entity) else {
        return;
    };
    let item = (table.ops.item)(&editor.draft, index);
    if item_name(&table, item.as_ref()) != asked_about {
        return;
    }
    (table.list.remove)(&mut editor.draft.plan, index);
    table.wanted = Some(Row(index.saturating_sub(1)));
    editor.commit();
}

/// The `import-earnings` command: asks which statement to record on the
/// highlighted person.
pub fn import_earnings(
    focused: FocusedTable,
    draft: Res<Draft>,
    pickers: Res<Pickers>,
    mut browsing: ResMut<Browsing>,
    mut importing: ResMut<Importing>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return refusal;
    }
    let Some((_, table, index)) = focused.cursor() else {
        return Outcome::Refused(NOBODY_HIGHLIGHTED.to_owned());
    };
    if table.ops.paths != [People::PATH] {
        return Outcome::Refused(NOT_A_PERSON.to_owned());
    }
    let person = item_name(table, (table.ops.item)(&draft, index).as_ref());
    importing.ask(person, &pickers, &mut browsing);
    Outcome::Done
}

/// Records the statement at `path` on the person the import was asked
/// for, as one step of history.
pub fn record_statement(
    In(path): In<PathBuf>,
    mut importing: ResMut<Importing>,
    mut editor: DraftEditor,
) {
    let Some(person) = importing.0.take() else {
        return;
    };
    match adopt(&path, &person, &mut editor.draft.plan) {
        Ok(years) => {
            editor.commit();
            let name = editor.draft.plan.person_name(&person);
            journal::say(format!("recorded {years} year(s) of earnings for {name}"));
        }
        Err(reason) => journal::warn(format!("not recorded: {reason}")),
    }
}

/// The years recorded from the statement at `path`.
fn adopt(path: &Path, person: &str, plan: &mut Plan) -> Result<usize, String> {
    let xml =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let statement = statement::parse(&xml).map_err(|error| error.to_string())?;
    plan.adopt_earnings(person, &statement)
        .map_err(|issue| issue.message)?;
    Ok(statement.earnings.len())
}

/// The field the domain knows `item` by, which is what a deletion names
/// and re-checks.
fn item_name(table: &DomainTable, item: Option<&Table>) -> String {
    item.and_then(|item| item.get(table.list.identity).map(to_text))
        .unwrap_or_default()
}
