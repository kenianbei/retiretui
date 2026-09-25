//! The charted metric year by year, a column per plan; its cursor is the
//! year cursor, each moving the other.

use std::collections::BTreeSet;

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity};
use plurimus::ui::ScrollArea;
use plurimus::widgets::ActiveDescendant;

use super::Plans;
use crate::commands::tui::edit::table_bundle;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, placed};
use crate::commands::tui::session::RowYear;
use crate::commands::tui::tabulate;

const YEAR: &str = "Year";
const GAP: u16 = 1;

#[derive(Component)]
pub(super) struct ByYearTable;

/// The table, under `pane`.
pub(super) fn spawn(commands: &mut Commands, pane: Entity) -> Entity {
    commands
        .spawn((
            table_bundle(),
            ByYearTable,
            Hints(&[("↑↓", "year")]),
            layout::Rests,
            placed(),
            ChildOf(pane),
        ))
        .id()
}

/// A row per year any plan reaches, the cursor on the cursor year's.
pub(super) fn fill(
    commands: &mut Commands,
    (table, scroll): (Entity, &mut ScrollArea),
    plans: &Plans,
) {
    let header: Vec<String> = std::iter::once(YEAR.to_owned())
        .chain(plans.each().map(|(name, _)| name.into_owned()))
        .collect();
    let years: BTreeSet<i16> = plans
        .each()
        .flat_map(|(_, projection)| projection.years.iter().map(|row| row.year))
        .collect();
    let amounts = plans.amounts(plans.charted.metric);
    let rows: Vec<Vec<String>> = years
        .iter()
        .map(|&year| {
            std::iter::once(year.to_string())
                .chain(plans.figures_in(&amounts, year))
                .collect()
        })
        .collect();
    commands
        .entity(table)
        .insert(tabulate::gapped_columns((&header, &rows), GAP));
    let spawned = tabulate::refill(commands, (table, scroll), (&header, &rows), &[0]);
    let cursor = plans.year();
    for (&row, &year) in spawned.iter().zip(&years) {
        commands.entity(row).insert(RowYear(year));
        if year == cursor {
            commands.entity(table).insert(ActiveDescendant(Some(row)));
        }
    }
}
