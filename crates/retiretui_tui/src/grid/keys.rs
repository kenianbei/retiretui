//! The keys a grid answers while it holds the keyboard - the table's own
//! bindings, shared so the two cannot drift: the cursor stepped a row, a
//! page or to either end, and passed on once it can move nothing.

use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::{Commands, On, Query, Without};
use bevy_input::keyboard::KeyboardInput;
use bevy_input_focus::FocusedInput;
use bevy_ui::ComputedNode;
use plurimus::term::HeldModifiers;
use plurimus::ui::first_bound;
use plurimus::widgets::{ActiveDescendant, TableAction, TableKeys};

use super::{Grid, Said};
use crate::layout;

pub(super) fn handle_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    grids: Query<(&Grid, &TableKeys, &ActiveDescendant), Without<Said>>,
    bodies: Query<(&ComputedNode, &Children)>,
    mut commands: Commands,
) {
    let table = input.focused_entity;
    let Ok((grid, keys, cursor)) = grids.get(table) else {
        return;
    };
    let Some(action) = first_bound(&keys.0, &input.input, held.get()) else {
        return;
    };
    let Ok((body, rows)) = bodies.get(grid.body) else {
        return;
    };
    let Some(last) = rows.len().checked_sub(1) else {
        return;
    };
    let page = usize::from(layout::node_rows(body)).max(1);
    let at = cursor
        .0
        .and_then(|row| rows.iter().position(|&each| each == row));
    let to = match (action, at) {
        (TableAction::RowPrev, Some(at)) => at.saturating_sub(1),
        (TableAction::RowNext, Some(at)) => (at + 1).min(last),
        (TableAction::PageUp, Some(at)) => at.saturating_sub(page),
        (TableAction::PageDown, Some(at)) => (at + page).min(last),
        (TableAction::RowLast, _) => last,
        (
            TableAction::RowFirst
            | TableAction::RowPrev
            | TableAction::RowNext
            | TableAction::PageUp
            | TableAction::PageDown,
            None,
        )
        | (TableAction::RowFirst, _) => 0,
        _ => return,
    };
    if cursor.0 != Some(rows[to]) {
        commands
            .entity(table)
            .insert(ActiveDescendant(Some(rows[to])));
        input.propagate(false);
    }
}
