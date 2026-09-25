//! What is read rather than edited: every field of an item in the form's
//! words, beside a table for the row under its cursor, or filling a page
//! for the one item a domain or a tool has. ⏎ opens the item's form over
//! the page. A person's details end with their earnings record.

use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, Res, With};
use bevy_input::keyboard::KeyboardInput;
use bevy_input_focus::FocusedInput;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{ScrollArea, first_bound};
use plurimus::widgets::ActiveDescendant;
use retiretui_engine::plan::Plan;
use toml::Table;

use super::applies;
use super::cells::Shown;
use super::codec::get_path;
use super::domain::{FieldSpec, ListOps, Ops};
use super::draft::Draft;
use super::editing::{self, Slot};
use super::group::gate_of;
use super::offers::NAME_KEY;
use super::table::{DomainTable, OPEN_KEYS, Row, cursor_row, table_bundle};
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, filling, placed};
use crate::commands::tui::nav::{FocusStop, ShownSurface};
use crate::commands::tui::pane::Pane;
use crate::commands::tui::tabulate;

/// The width of the pane beside a table, borders included: the longest
/// label, a gap, and a value's worth of cells; beside the sidebar and a
/// table it fits the narrowest terminal.
pub const DETAILS_COLS: u16 = 34;
const GAP: u16 = 1;
const HEADER: [&str; 2] = ["", ""];
const TEXT_COLUMNS: [usize; 2] = [0, 1];
const BESIDE_HINTS: Hints = Hints(&[("↑↓", "scroll"), ("⏎", "edit")]);

/// The table an item's details are rows of: of the domain `ops`, and of
/// the row under `table`'s cursor where the domain has a table.
#[derive(Component)]
pub struct DetailsTable {
    ops: Ops,
    table: Option<Entity>,
}

/// Whether the domain's items say more than its columns show: a field no
/// column shows, or a record. The identity is known to the table whether
/// or not a column shows it, and a column showing the identity shows the
/// name in its place.
pub fn has_details(ops: Ops, list: ListOps) -> bool {
    let is_column = |key: &str| list.columns.iter().any(|column| column.key == key);
    let shows_name = is_column(list.identity);
    let is_known = |key: &str| key == list.identity || (shows_name && key == NAME_KEY);
    list.record.is_some()
        || ops
            .fields
            .iter()
            .any(|spec| !is_known(spec.key) && !is_column(spec.key))
}

/// The pane beside `table` in `beside`, following its cursor.
pub fn spawn_pane(commands: &mut Commands, beside: Entity, table: Entity, ops: Ops) {
    let singular = ops.list.map_or(ops.title, |list| list.singular);
    let pane = Pane::new(singular)
        .wide(f32::from(DETAILS_COLS))
        .spawn(commands, beside);
    spawn_into(commands, pane, ops, Some(table), BESIDE_HINTS);
}

/// The details table inside `pane`, of `ops`'s item - the one under
/// `table`'s cursor where there is a table, else the only one.
pub fn spawn_into(
    commands: &mut Commands,
    pane: Entity,
    ops: Ops,
    table: Option<Entity>,
    hints: Hints,
) {
    commands
        .spawn((
            table_bundle(),
            DetailsTable { ops, table },
            FocusStop,
            hints,
            layout::Rests,
            filling(),
            placed(),
            ChildOf(pane),
        ))
        .observe(handle_enter);
}

/// The item's rows: each field it has a use for - shown by its own rule,
/// and by the tick whose table it is in - labelled and phrased as the
/// form phrases it, then the record where the domain keeps one.
fn rows(ops: Ops, item: &Table, plan: &Plan) -> Vec<Vec<String>> {
    let opened = applies::opened(ops, item);
    let is_used = |spec: &&FieldSpec| {
        let is_held = |gate| get_path(&opened, gate).is_some();
        spec.shown.is_none_or(|shown| shown(&opened))
            && gate_of(ops.fields, spec.key).is_none_or(is_held)
    };
    let mut rows: Vec<Vec<String>> = ops
        .fields
        .iter()
        .filter(is_used)
        .map(|spec| {
            let shown = Shown::of_field(spec, ops.fields, plan);
            vec![spec.label.to_owned(), shown.cell(&opened, plan).text]
        })
        .collect();
    if let Some(record) = ops.list.and_then(|list| list.record) {
        rows.extend(record(item).into_iter().map(Vec::from));
    }
    rows
}

/// Rewrites each shown details table from its item: one following a
/// table, whenever its cursor moves, which a rebuilt table's does; the
/// only item's, whenever the draft or the page moves.
pub fn refresh(
    (draft, shown): (Res<Draft>, ShownSurface),
    tables: Query<Ref<ActiveDescendant>, With<DomainTable>>,
    cursors: Query<&Row>,
    mut details: Query<(Entity, &DetailsTable, &mut ScrollArea)>,
    mut commands: Commands,
) {
    let surface = shown.surface();
    for (table, shown_of, mut scroll) in &mut details {
        let cursor = shown_of.table.and_then(|table| tables.get(table).ok());
        let is_moved = match &cursor {
            Some(cursor) => cursor.is_changed(),
            None => draft.is_changed() || shown.is_changed(),
        };
        if shown_of.ops.surface != surface || !is_moved {
            continue;
        }
        let item = shown_row(cursor.as_deref(), &cursors)
            .and_then(|Row(index)| (shown_of.ops.item)(&draft, index));
        let rows = item.map_or_else(Vec::new, |item| rows(shown_of.ops, &item, &draft.plan));
        commands
            .entity(table)
            .insert(tabulate::labelled(&rows, GAP));
        let header = HEADER.map(str::to_owned);
        tabulate::refill(
            &mut commands,
            (table, &mut scroll),
            (&header, &rows),
            &TEXT_COLUMNS,
        );
    }
}

/// Enter on the details opens the item they show.
fn handle_enter(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    (details, tables, rows): (
        Query<&DetailsTable>,
        Query<&ActiveDescendant, With<DomainTable>>,
        Query<&Row>,
    ),
    mut commands: Commands,
) {
    let Ok(shown) = details.get(input.focused_entity) else {
        return;
    };
    if first_bound(OPEN_KEYS, &input.input, held.get()).is_none() {
        return;
    }
    input.propagate(false);
    let cursor = shown.table.and_then(|table| tables.get(table).ok());
    if let Some(row) = shown_row(cursor, &rows) {
        let opened = (shown.ops, shown.table, Slot::At(row));
        commands.run_system_cached_with(editing::open_item, opened);
    }
}

/// The row details show: the one under `cursor`, of the table they follow,
/// or the only item where they follow none.
fn shown_row(cursor: Option<&ActiveDescendant>, rows: &Query<&Row>) -> Option<Row> {
    match cursor {
        Some(cursor) => cursor_row(*cursor, rows),
        None => Some(Row(0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::edit::SCREENS;
    use crate::commands::tui::nav::Page;

    #[test]
    fn a_domain_has_a_pane_only_where_its_items_say_more_than_its_columns() {
        let with_pane = [
            Page::Accounts,
            Page::Income,
            Page::Expenses,
            Page::Conversions,
            Page::Contributions,
            Page::People,
        ];
        for ops in SCREENS {
            let Some(list) = ops.list else {
                continue;
            };
            let page = ops.surface.unwrap();
            assert_eq!(
                has_details(*ops, list),
                with_pane.contains(&page),
                "{page:?}"
            );
        }
    }
}
