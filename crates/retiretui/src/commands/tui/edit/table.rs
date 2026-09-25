//! The view side of an editing page: one plurimus `Table` per domain,
//! its rows drawn from the draft.

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{
    ChildOf, Children, Commands, Component, Entity, Mut, On, Query, Ref, Res, ResMut,
};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::FocusedInput;
use plurimus::core::ratatui_core::layout::{Constraint, Rect, Size};
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::Line;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{ComputedWidgetArea, KeyBinding, ScrollArea, UiStyle, first_bound};
use plurimus::widgets::{
    ActiveDescendant, TableAction, TableColumns, TableGeometry, TableKeys, TablePosition,
    TableSelection, TableStripe, ValueChange, table_header, table_row,
};

use super::domain::{ListOps, Ops};
use super::draft::Draft;
use super::editing::{self, Slot};
use super::sort::{self, Sort};
use super::widths::Laid;
use crate::commands::tui::layout;
use crate::commands::tui::motion::{Cues, Play};
use crate::commands::tui::nav::{ActivePage, Page, ShownSurface};
use crate::commands::tui::theme::Theme;

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
}

/// The item a body row stands for, by where it sits in the plan.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Row(pub usize);

/// Turns to a page, the cursor on the item of its table at `index`.
#[derive(SystemParam)]
pub struct Turn<'w, 's> {
    active: ResMut<'w, ActivePage>,
    tables: Query<'w, 's, &'static mut DomainTable>,
}

impl Turn<'_, '_> {
    pub fn to(&mut self, page: Page, index: Option<usize>) {
        self.active.0 = page;
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
    TableKeys(vec![
        (Key::ArrowUp.into(), TableAction::RowPrev),
        (Key::ArrowDown.into(), TableAction::RowNext),
        (Key::Home.into(), TableAction::RowFirst),
        (Key::End.into(), TableAction::RowLast),
        (Key::PageUp.into(), TableAction::PageUp),
        (Key::PageDown.into(), TableAction::PageDown),
    ])
}

/// A domain's table. The columns are left unshared until the first
/// rebuild measures what they hold; the scroll area's content size is
/// left at zero, which the widget crate keeps in step with the rows.
pub fn table_bundle() -> impl bevy_ecs::bundle::Bundle {
    (
        plurimus::widgets::table([]),
        TableSelection::Row,
        layout::table_cursor(),
        table_keys(),
        TableStripe(Style::new()),
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
/// same item unless the table asked for another.
pub fn rebuild_rows(
    view: TableView,
    mut tables: Query<(
        Entity,
        &mut DomainTable,
        &ActiveDescendant,
        Ref<ComputedWidgetArea>,
        &ScrollArea,
    )>,
    rows: Query<&Row>,
    mut commands: Commands,
) {
    let (draft, theme) = (&view.draft, &view.theme);
    let is_turned = view.shown.is_changed();
    let surface = view.shown.surface();
    for (table, mut domain_table, cursor, area, scroll) in &mut tables {
        // What the rows are drawn in, which is the area less the column
        // a scrolled table keeps for its bar.
        let given = scroll.content_width(area.0.width);
        // A resize changes the area the columns are shared over.
        let is_stale =
            draft.is_changed() || is_turned || domain_table.is_changed() || area.is_changed();
        if domain_table.ops.surface != surface || !is_stale {
            continue;
        }
        let kept = cursor_row(*cursor, &rows);
        let table_ref = domain_table.bypass_change_detection();
        let wanted = table_ref.wanted.take().or(kept);
        commands.entity(table).despawn_related::<Children>();
        let list = table_ref.list;
        let mut items: Vec<sort::Item> = (list.rows)(&draft.plan).into_iter().enumerate().collect();
        if let Some(sort) = table_ref.sort {
            items = sort.order(items);
        }
        if items.is_empty() {
            spawn_nothing_yet(&mut commands, table, list.purpose, theme);
            commands.entity(table).insert(ActiveDescendant(None));
            continue;
        }
        let laid = Laid::of(table_ref, &items, given);
        commands.entity(table).insert(laid.columns());
        commands.spawn((
            table_header(laid.header()),
            UiStyle(Style::new().add_modifier(Modifier::BOLD)),
            ChildOf(table),
        ));
        let placed = spawn_body(&mut commands, table, &laid, items, wanted);
        commands.entity(table).insert(ActiveDescendant(placed));
    }
}

/// Spawns a row per item; answers with the row the cursor is to rest on.
fn spawn_body(
    commands: &mut Commands,
    table: Entity,
    laid: &Laid,
    items: Vec<sort::Item>,
    wanted: Option<Row>,
) -> Option<Entity> {
    let mut cursor_row = None;
    let mut first_row = None;
    for (index, cells) in items {
        let row = Row(index);
        let spawned = commands
            .spawn((table_row(laid.row(cells)), row, ChildOf(table)))
            .id();
        first_row.get_or_insert(spawned);
        if Some(row) == wanted {
            cursor_row = Some(spawned);
        }
    }
    cursor_row.or(first_row)
}

/// What a domain with nothing in it yet says: what it is for, in place of
/// headers over no rows.
fn spawn_nothing_yet(commands: &mut Commands, table: Entity, purpose: &str, theme: &Theme) {
    commands
        .entity(table)
        .insert(TableColumns(vec![Constraint::Fill(1)]));
    let indent = usize::from(layout::CURSOR_COLS);
    // A header, not a row: the widget steps and clicks its body rows, and
    // a line no item stands behind must never take the cursor.
    commands.spawn((
        table_header(vec![Line::from(format!("{:indent$}{purpose}", ""))]),
        UiStyle(theme.dimmed()),
        ChildOf(table),
    ));
}

/// Flashes the row of the item just applied: the receipt for a write that
/// is otherwise instant. It fades in from `dim` rather than from the
/// accent, which the cursor row it lands on is already drawn in. It waits
/// a frame, for the rebuilt rows to be scrolled to where they will rest.
pub fn cue_receipt(
    mut tables: Query<(
        Entity,
        Mut<DomainTable>,
        &ActiveDescendant,
        &ComputedWidgetArea,
    )>,
    geometry: TableGeometry,
    theme: Res<Theme>,
    mut cues: ResMut<Cues>,
) {
    for (table, mut domain_table, cursor, area) in &mut tables {
        if !domain_table.is_applied || domain_table.is_changed() {
            continue;
        }
        domain_table.bypass_change_detection().is_applied = false;
        // An applied item is the one the cursor is left on.
        let row = cursor.0.and_then(|row| geometry.cell_rect(table, row, 0));
        if let Some(row) = row {
            let flashed = Rect::new(area.0.x, row.y, area.0.width, 1);
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

/// A press on a row opens it.
pub fn handle_row_select(
    select: On<ValueChange<TablePosition>>,
    tables: Query<&DomainTable>,
    rows: Query<&Row>,
    mut commands: Commands,
) {
    let Ok(domain) = tables.get(select.source) else {
        return;
    };
    if let Some(&row) = select.value.row.and_then(|row| rows.get(row).ok()) {
        let opened = (domain.ops, Some(select.source), Slot::At(row));
        commands.run_system_cached_with(editing::open_item, opened);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_empty_domain_says_what_it_is_for_instead_of_heading_no_rows() {
        let lists = super::super::SCREENS.iter().filter_map(|ops| ops.list);
        for list in lists {
            assert!(!list.purpose.is_empty(), "{} says nothing", list.singular);
        }
    }
}
