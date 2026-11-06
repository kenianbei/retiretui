//! The charted metric year by year, a column per plan; its cursor is the
//! year cursor, each moving the other.

use std::collections::BTreeSet;

use bevy_ecs::prelude::{Commands, Component, Entity};
use plurimus::widgets::ActiveDescendant;

use super::Plans;
use crate::grid::{self, Grid};
use crate::hints::Hints;
use crate::layout;
use crate::session::RowYear;
use crate::tabulate;

const YEAR: &str = "Year";
const GAP: u16 = 1;

#[derive(Component)]
pub(super) struct ByYearTable;

/// The table, under `pane`.
pub(super) fn spawn(commands: &mut Commands, pane: Entity) -> Entity {
    let table = grid::spawn(commands, pane);
    commands
        .entity(table)
        .insert((ByYearTable, Hints(&[("↑↓", "year")]), layout::Rests))
        .id()
}

/// A row per year any plan reaches, the cursor on the cursor year's.
pub(super) fn fill(commands: &mut Commands, grid: &Grid, plans: &Plans) {
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
    let spawned = grid::refill(
        commands,
        grid,
        tabulate::gapped_columns((&header, &rows), GAP),
        (&header, &rows),
        &[0],
    );
    let cursor = plans.year();
    for (&row, &year) in spawned.iter().zip(&years) {
        commands.entity(row).insert(RowYear(year));
        if year == cursor {
            commands
                .entity(grid.root)
                .insert(ActiveDescendant(Some(row)));
        }
    }
}
