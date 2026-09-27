//! The table's order under the pointer and the `sort` command; the order
//! itself is the client's.

use bevy_ecs::prelude::{On, Query};
use plurimus::widgets::TableHeaderClick;
use retiretui_client::forms::sort::Sort;

use super::commands::FocusedTable;
use super::table::DomainTable;
use crate::command::Outcome;

pub fn handle_header_click(click: On<TableHeaderClick>, mut tables: Query<&mut DomainTable>) {
    if let Ok(mut table) = tables.get_mut(click.entity) {
        table.sort = Sort::pressed(table.sort, click.column);
    }
}

/// The `sort` command, on the table the keyboard is in or whose item it
/// is in.
pub fn sort(mut focused: FocusedTable) -> Outcome {
    let acting = focused.acting().map(|(entity, ..)| entity);
    let Some((mut table, _)) = acting.and_then(|table| focused.tables.get_mut(table).ok()) else {
        return Outcome::Refused("nothing to sort here".to_owned());
    };
    table.sort = Sort::stepped(table.sort, table.list.columns.len());
    Outcome::Done
}
