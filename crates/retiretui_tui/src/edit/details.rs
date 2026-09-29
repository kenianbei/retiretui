//! What is read rather than edited: every field of an item in the form's
//! words, beside a table for the row under its cursor, or filling a page
//! for the one item a domain or a tool has. ⏎ opens the item's form over
//! the page. A person's details end with their earnings record.

use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, Res, With};
use bevy_input::keyboard::KeyboardInput;
use bevy_input_focus::FocusedInput;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{ScrollArea, UiStyle, first_bound};
use plurimus::widgets::ActiveDescendant;

use super::domain::Ops;
use super::draft::Draft;
use super::editing::{self, Slot};
use super::table::{DomainTable, OPEN_KEYS, Row, cursor_row, table_bundle};
use crate::hints::Hints;
use crate::layout::{self, filling, placed};
use crate::nav::{FocusStop, ShownSurface};
use crate::pane::Pane;
use crate::tabulate;
use retiretui_client::forms::details::{self, ReadRow};

/// The width of the pane beside a table, borders included: the longest
/// label, a gap, and a value's worth of cells; beside the sidebar and a
/// table it fits the narrowest terminal.
pub const DETAILS_COLS: u16 = 36;
const GAP: u16 = 1;
const HEADER: [&str; 2] = ["", ""];
const TEXT_COLUMNS: [usize; 2] = [0, 1];
/// A field read out as what its blank stands for, dimmed beside the
/// values the plan states.
const UNSTATED: Style = Style::new().add_modifier(Modifier::DIM);
/// What sets a field in under the heading that gathers it.
const GATHERED: &str = "  ";
/// A heading over the fields it gathers.
const HEADING: Style = Style::new().add_modifier(Modifier::BOLD);
const BESIDE_HINTS: Hints = Hints(&[("↑↓", "scroll"), ("⏎", "edit")]);

/// The table an item's details are rows of: of the domain `ops`, and of
/// the row under `table`'s cursor where the domain has a table.
#[derive(Component)]
pub struct DetailsTable {
    ops: Ops,
    table: Option<Entity>,
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
        let read = item.map_or_else(Vec::new, |item| {
            details::rows(&shown_of.ops, &item, &draft.plan)
        });
        let (rows, styles) = drawn(&read);
        commands
            .entity(table)
            .insert(tabulate::labelled(&rows, GAP));
        let header = HEADER.map(str::to_owned);
        let spawned = tabulate::refill(
            &mut commands,
            (table, &mut scroll),
            (&header, &rows),
            &TEXT_COLUMNS,
        );
        for (&row, style) in spawned.iter().zip(styles) {
            if let Some(style) = style {
                commands.entity(row).insert(UiStyle(style));
            }
        }
    }
}

/// The rows `read` is drawn as, each heading above the fields it gathers,
/// and the style each row stands out in, where it does.
fn drawn(read: &[ReadRow]) -> (Vec<Vec<String>>, Vec<Option<Style>>) {
    let mut rows = Vec::with_capacity(read.len());
    let mut styles = Vec::with_capacity(read.len());
    let mut before = None;
    for row in read {
        if let Some(group) = row.group.filter(|&group| before != Some(group)) {
            rows.push(vec![group.to_owned(), String::new()]);
            styles.push(Some(HEADING));
        }
        before = row.group;
        let label = match row.group {
            Some(_) => format!("{GATHERED}{}", row.label),
            None => row.label.clone(),
        };
        rows.push(vec![label, row.text.clone()]);
        styles.push(row.is_unstated.then_some(UNSTATED));
    }
    (rows, styles)
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
    use crate::edit::screens;
    use crate::nav::Page;

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
        for ops in screens() {
            let Some(list) = ops.list else {
                continue;
            };
            let page = ops.surface.unwrap();
            assert_eq!(
                details::has_details(&ops, list),
                with_pane.contains(&page),
                "{page:?}"
            );
        }
    }
}
