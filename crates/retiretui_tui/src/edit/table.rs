//! The view side of an editing page: one grid per domain, its rows drawn
//! from the draft.

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{Commands, Component, Entity, Mut, On, Query, Res, ResMut};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::FocusedInput;
use bevy_ui::ComputedNode;
use plurimus::bui::ComputedNodeRect;
use plurimus::core::ratatui_core::layout::{Constraint, Rect, Size};
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{KeyBinding, ScrollArea, UiStyle, first_bound};
use plurimus::widgets::{ActiveDescendant, TableAction, TableKeys, TableSelection};

use super::domain::{ListOps, Ops};
use super::draft::Draft;
use super::editing::{self, Slot};
use super::widths::Laid;
use crate::grid::{self, Grid, RowClick};
use crate::layout;
use crate::motion::{Cues, Play};
use crate::nav::{self, Page, ShownSurface};
use crate::theme::Theme;
use retiretui_client::forms::sort::{self, Sort};

#[derive(Component)]
pub struct DomainTable {
    pub ops: Ops,
    /// The list the table draws.
    pub list: ListOps,
    /// The order the rows are shown in; `None` is the plan's own.
    pub sort: Option<Sort>,
    /// The row to put the cursor on after the next rebuild.
    pub wanted: Option<Row>,
    /// Whether an item was just applied and its row is still to be flashed.
    pub is_applied: bool,
    /// The width the rows were last laid out over; none while they are
    /// behind what they are drawn from.
    pub built_for: Option<u16>,
}

/// The item a body row stands for, by where it sits in the plan.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Row(pub usize);

/// Turns to a page, the cursor on the item of its table at `index`.
#[derive(SystemParam)]
pub struct Turn<'w, 's> {
    turn: nav::Turn<'w>,
    tables: Query<'w, 's, &'static mut DomainTable>,
}

impl Turn<'_, '_> {
    pub fn to(&mut self, page: Page, index: Option<usize>) {
        self.turn.to(page);
        let Some(index) = index else {
            return;
        };
        let mut tables = self.tables.iter_mut();
        if let Some(mut table) = tables.find(|table| table.ops.surface == Some(page)) {
            table.wanted = Some(Row(index));
        }
    }
}

pub(super) const OPEN_KEYS: &[(KeyBinding, ())] = &[(KeyBinding::new(Key::Enter), ())];

/// The table's keys, without the column moves a row-selecting table has no
/// use for, and without Enter: a press selects a row as Enter would, and
/// the two are told apart by keeping Enter for [`handle_table_key`].
pub fn table_keys() -> TableKeys {
    let mut keys = TableKeys::default();
    keys.0.retain(|(_, action)| {
        !matches!(
            action,
            TableAction::ColumnPrev | TableAction::ColumnNext | TableAction::Select
        )
    });
    keys
}

/// A plurimus table, for the pages not yet drawn as grids. The columns
/// are left unshared until the first rebuild measures what they hold; the
/// scroll area's content size is left at zero, which the widget crate
/// keeps in step with the rows.
pub fn table_bundle() -> impl bevy_ecs::bundle::Bundle {
    (
        plurimus::widgets::table([]),
        TableSelection::Row,
        layout::table_cursor(),
        table_keys(),
        plurimus::widgets::TableStripe(Style::new()),
        ScrollArea::new(Size::default()),
    )
}

/// What the rows are drawn from: the draft they stand for, the surface on
/// show, and the theme an empty domain's line is dimmed by.
#[derive(SystemParam)]
pub struct TableView<'w> {
    draft: Res<'w, Draft>,
    theme: Res<'w, Theme>,
    shown: ShownSurface<'w>,
}

/// Respawns a shown table's rows from the draft, keeping the cursor on the
/// same item unless the table asked for another. A table hidden or not yet
/// laid out when what it draws moves is marked behind, and rebuilt once it
/// is shown with room; one turned back to unmoved keeps its rows.
pub fn rebuild_rows(
    view: TableView,
    mut tables: Query<(Entity, &mut DomainTable, &Grid, &ActiveDescendant)>,
    (bodies, rows): (Query<&ComputedNode>, Query<&Row>),
    mut commands: Commands,
) {
    let (draft, theme) = (&view.draft, &view.theme);
    let surface = view.shown.surface();
    for (table, mut domain_table, grid, cursor) in &mut tables {
        let is_moved = draft.is_changed() || theme.is_changed() || domain_table.is_changed();
        let given = bodies.get(grid.body).map_or(0, layout::node_cols);
        if domain_table.ops.surface != surface || given == 0 {
            if is_moved {
                domain_table.bypass_change_detection().built_for = None;
            }
            continue;
        }
        if !is_moved && domain_table.built_for == Some(given) {
            continue;
        }
        let kept = cursor_row(*cursor, &rows);
        let table_ref = domain_table.bypass_change_detection();
        table_ref.built_for = Some(given);
        let wanted = table_ref.wanted.take().or(kept);
        let list = table_ref.list;
        let mut items: Vec<sort::Item> = (list.rows)(&draft.plan).into_iter().enumerate().collect();
        if let Some(sort) = table_ref.sort {
            items = sort.order(items);
        }
        if items.is_empty() {
            spawn_nothing_yet(&mut commands, (table, grid), list.purpose, theme);
            continue;
        }
        let laid = Laid::of(table_ref, &items, given);
        let indices: Vec<usize> = items.iter().map(|(index, _)| *index).collect();
        let rows = items.into_iter().map(|(_, cells)| laid.row(cells));
        let root = (table, grid);
        let spawned = grid::refill_lines(&mut commands, root, laid.columns(), laid.header(), rows);
        let mut cursor_row = None;
        for (&row, index) in spawned.iter().zip(indices) {
            commands.entity(row).insert(Row(index));
            if Some(Row(index)) == wanted {
                cursor_row = Some(row);
            }
        }
        let placed = cursor_row.or_else(|| spawned.first().copied());
        commands.entity(table).insert(ActiveDescendant(placed));
    }
}

/// What a domain with nothing in it yet says: what it is for, in place of
/// headers over no rows. A header, not a row: a line no item stands
/// behind must never take the cursor.
fn spawn_nothing_yet(
    commands: &mut Commands,
    (table, grid): (Entity, &Grid),
    purpose: &str,
    theme: &Theme,
) {
    let header = vec![Line::from(purpose.to_owned())];
    grid::refill_lines(
        commands,
        (table, grid),
        vec![Constraint::Fill(1)],
        header,
        std::iter::empty(),
    );
    commands.entity(grid.header).insert(UiStyle(theme.dimmed()));
}

/// Flashes the row of the item just applied: the receipt for a write that
/// is otherwise instant. It fades in from `dim` rather than from the
/// accent, which the cursor row it lands on is already drawn in. It waits
/// a frame, for the rebuilt rows to be scrolled to where they will rest.
/// Only a terminal lays its nodes out in cells, so only there is a rect
/// to flash.
pub fn cue_receipt(
    mut tables: Query<(Entity, Mut<DomainTable>, &ActiveDescendant)>,
    rects: Query<&ComputedNodeRect>,
    theme: Res<Theme>,
    mut cues: ResMut<Cues>,
) {
    for (table, mut domain_table, cursor) in &mut tables {
        if !domain_table.is_applied || domain_table.is_changed() {
            continue;
        }
        domain_table.bypass_change_detection().is_applied = false;
        // An applied item is the one the cursor is left on.
        let row = cursor.0.and_then(|row| rects.get(row).ok());
        if let (Some(row), Ok(across)) = (row, rects.get(table)) {
            let flashed = Rect::new(across.rect.x, row.rect.y, across.rect.width, 1);
            cues.play(Play::Receipt(theme.dim), flashed);
        }
    }
}

/// The row `cursor` rests on, where it rests on one.
pub fn cursor_row(cursor: ActiveDescendant, rows: &Query<&Row>) -> Option<Row> {
    cursor.0.and_then(|row| rows.get(row).ok()).copied()
}

/// Enter on a row opens the item it stands for.
pub fn handle_table_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    tables: Query<(&DomainTable, &ActiveDescendant)>,
    rows: Query<&Row>,
    mut commands: Commands,
) {
    let table = input.focused_entity;
    let Ok((domain, cursor)) = tables.get(table) else {
        return;
    };
    if first_bound(OPEN_KEYS, &input.input, held.get()).is_none() {
        return;
    }
    input.propagate(false);
    if let Some(row) = cursor_row(*cursor, &rows) {
        commands
            .run_system_cached_with(editing::open_item, (domain.ops, Some(table), Slot::At(row)));
    }
}

/// A click on a row opens it; the grid has put the keyboard on the table
/// first, so the form opened over it hands the keyboard back to it.
pub fn handle_row_click(
    click: On<RowClick>,
    tables: Query<&DomainTable>,
    rows: Query<&Row>,
    mut commands: Commands,
) {
    let table = click.entity;
    let (Ok(domain), Ok(&row)) = (tables.get(table), rows.get(click.row)) else {
        return;
    };
    let opened = (domain.ops, Some(table), Slot::At(row));
    commands.run_system_cached_with(editing::open_item, opened);
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_empty_domain_says_what_it_is_for_instead_of_heading_no_rows() {
        let lists = super::super::screens().filter_map(|ops| ops.list);
        for list in lists {
            assert!(!list.purpose.is_empty(), "{} says nothing", list.singular);
        }
    }
}
