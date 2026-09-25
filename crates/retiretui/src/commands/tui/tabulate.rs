//! The page's tables, drawn as every table in the shell is - the plurimus
//! table with the shared bar cursor - from rows of text measured here: the
//! first column lines up on the left, the rest on the right, unless a
//! table says which of its columns are text and sizes them itself.

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Entity};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::style::{Color, Modifier, Style};
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::{ScrollArea, UiStyle};
use plurimus::widgets::{ActiveDescendant, TableColumns, table_header, table_row};

/// Each column as wide as its widest cell, header included, and `gap`
/// cells more before the next than the one the table itself leaves.
pub(super) fn columns((header, rows): (&[String], &[Vec<String>]), gap: u16) -> TableColumns {
    let mut widths: Vec<u16> = Vec::new();
    for row in std::iter::once(header).chain(rows.iter().map(Vec::as_slice)) {
        for (at, cell) in row.iter().enumerate() {
            let width = u16::try_from(cell.chars().count()).unwrap_or(u16::MAX);
            match widths.get_mut(at) {
                Some(widest) => *widest = (*widest).max(width),
                None => widths.push(width),
            }
        }
    }
    let last = widths.len().saturating_sub(1);
    let spaced = widths.into_iter().enumerate().map(|(at, width)| {
        let gap = if at == last { 0 } else { gap };
        Constraint::Length(width.saturating_add(gap))
    });
    TableColumns(spaced.collect())
}

/// A label column as wide as its widest label and `gap` more, and the
/// values in whatever is left.
pub(super) fn labelled(rows: &[Vec<String>], gap: u16) -> TableColumns {
    let widest = rows
        .iter()
        .filter_map(|row| row.first())
        .map(|label| label.chars().count())
        .max();
    let label_cols = u16::try_from(widest.unwrap_or_default()).unwrap_or(u16::MAX);
    TableColumns(vec![
        Constraint::Length(label_cols.saturating_add(gap)),
        Constraint::Fill(1),
    ])
}

/// A row's cells, those at `text` on the left and the rest right.
fn cells(row: &[String], text: &[usize]) -> Vec<Line<'static>> {
    row.iter()
        .enumerate()
        .map(|(at, cell)| {
            let line = Line::from(cell.clone());
            if text.contains(&at) {
                line
            } else {
                line.right_aligned()
            }
        })
        .collect()
}

/// Spawns `header`, bold, and `rows` into `table`, measured as
/// [`columns`] measures them, the first column on the left and the rest
/// right, answering with each row's entity.
pub(super) fn fill(
    commands: &mut Commands,
    table: Entity,
    rows: (&[String], &[Vec<String>]),
    gap: u16,
) -> Vec<Entity> {
    commands.entity(table).insert(columns(rows, gap));
    let (header, body) = rows;
    let body = body.iter().map(|row| cells(row, &[0]));
    spawn_rows(commands, table, cells(header, &[0]), body)
}

/// What leads each of a keyed table's rows: a stroke of the line the row
/// names, in its colour.
const SWATCH: &str = "━━";

/// The cells [`SWATCH`] and the space after it take.
pub(super) const SWATCH_COLS: u16 = 3;

/// As [`columns`], the last column too `gap` wider, for a table whose
/// last column does not stand at the edge of its pane: headers lined up
/// on the right then keep the gap from their neighbours.
pub(super) fn gapped_columns(rows: (&[String], &[Vec<String>]), gap: u16) -> TableColumns {
    let mut measured = columns(rows, gap);
    if let Some(Constraint::Length(last)) = measured.0.last_mut() {
        *last = last.saturating_add(gap);
    }
    measured
}

/// As [`fill`], each row's first cell drawn in its style from `keys` and
/// led by a swatch in its colour, so the table names the lines a chart
/// draws in them.
pub(super) fn fill_keyed(
    commands: &mut Commands,
    table: Entity,
    rows: (&[String], &[Vec<String>]),
    gap: u16,
    keys: &[(Color, Style)],
) -> Vec<Entity> {
    let mut measured = gapped_columns(rows, gap);
    if let Some(Constraint::Length(first)) = measured.0.first_mut() {
        *first = first.saturating_add(SWATCH_COLS);
    }
    commands.entity(table).insert(measured);
    let (header, body) = rows;
    let mut header = cells(header, &[0]);
    if let Some(first) = header.first_mut() {
        first
            .spans
            .insert(0, Span::raw(" ".repeat(SWATCH_COLS.into())));
    }
    let body = body.iter().zip(keys).map(|(row, &(key, named))| {
        let mut row = cells(row, &[0]);
        if let Some(first) = row.first_mut() {
            first.style = named;
            first
                .spans
                .insert(0, Span::styled(format!("{SWATCH} "), Style::new().fg(key)));
        }
        row
    });
    spawn_rows(commands, table, header, body)
}

/// Replaces `table`'s rows with `header` and `rows`, the columns at `text`
/// on the left and the rest right, the cursor on the first row and the
/// scroll sized to them, answering each row's entity; the widths are the
/// caller's.
pub(super) fn refill(
    commands: &mut Commands,
    (table, scroll): (Entity, &mut ScrollArea),
    rows: (&[String], &[Vec<String>]),
    text: &[usize],
) -> Vec<Entity> {
    commands.entity(table).despawn_related::<Children>();
    let (header, body) = rows;
    let lines = body.iter().map(|row| cells(row, text));
    let spawned = spawn_rows(commands, table, cells(header, text), lines);
    commands
        .entity(table)
        .insert(ActiveDescendant(spawned.first().copied()));
    scroll.content_size.height = u16::try_from(body.len() + 1).unwrap_or(u16::MAX);
    spawned
}

fn spawn_rows(
    commands: &mut Commands,
    table: Entity,
    header: Vec<Line<'static>>,
    rows: impl Iterator<Item = Vec<Line<'static>>>,
) -> Vec<Entity> {
    commands.spawn((
        table_header(header),
        UiStyle(Style::new().add_modifier(Modifier::BOLD)),
        ChildOf(table),
    ));
    rows.map(|row| commands.spawn((table_row(row), ChildOf(table))).id())
        .collect()
}
