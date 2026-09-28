//! The Ledger's Income & Tax pane: what the cursor year brought in, and
//! where it went besides the accounts.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, With};
use plurimus::ui::ScrollArea;
use plurimus::widgets::WidgetSystems;
use retiretui_client::ledger::{DetailLine, income_and_tax};
use retiretui_engine::plan::Plan;
use retiretui_engine::project::YearRow;

use super::super::edit::table_bundle;
use super::super::hints::Hints;
use super::super::layout::{self, filling, placed};
use super::super::nav::FocusStop;
use super::super::pane::Pane;
use super::super::session::Shown;
use super::super::tabulate;
use super::DETAIL_GAP;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        refresh_detail
            .in_set(super::split::DetailFilled)
            .before(WidgetSystems::Layout),
    );
}

const TITLE: &str = "Income & Tax";
const HEADER: [&str; 2] = ["", "Amount"];
/// The pane's width, borders included: the longest label and a
/// seven-figure amount.
const DETAIL_COLS: f32 = 30.0;

#[derive(Component)]
struct DetailTable;

pub(super) fn spawn_pane(commands: &mut Commands, parent: Entity) {
    let pane = Pane::new(TITLE).wide(DETAIL_COLS).spawn(commands, parent);
    commands.spawn((
        table_bundle(),
        DetailTable,
        FocusStop,
        Hints(&[("↑↓", "line")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(pane),
    ));
}

fn refresh_detail(
    shown: Shown,
    mut tables: Query<(Entity, &mut ScrollArea), With<DetailTable>>,
    mut commands: Commands,
) {
    if !shown.is_changed() {
        return;
    }
    let Some(row) = shown.row() else {
        return;
    };
    let rows = detail_rows(row, &shown.ledger().plan, shown.basis.nominal);
    let header = HEADER.map(str::to_owned);
    let widths = tabulate::columns((&header, &rows), DETAIL_GAP);
    for (table, mut scroll) in &mut tables {
        commands.entity(table).insert(widths.clone());
        tabulate::refill(&mut commands, (table, &mut scroll), (&header, &rows), &[0]);
    }
}

/// Income by source, then a blank line where there was any, then spending
/// and what the year paid besides.
fn detail_rows(row: &YearRow, plan: &Plan, is_nominal: bool) -> Vec<Vec<String>> {
    let (income, paid) = income_and_tax(plan, row, is_nominal);
    let line = |line: DetailLine| vec![line.label, line.amount];
    let mut rows: Vec<Vec<String>> = income.into_iter().map(line).collect();
    if !rows.is_empty() {
        rows.push(vec![String::new(); 2]);
    }
    rows.extend(paid.into_iter().map(line));
    rows
}

#[cfg(test)]
mod tests {
    use super::super::super::support::test_projected;
    use super::*;

    #[test]
    fn detail_lists_income_then_what_was_paid() {
        let projected = test_projected();
        let row = &projected.projection.years[0];
        let rows = detail_rows(row, &projected.plan, true);
        let labels: Vec<&str> = rows.iter().map(|cells| cells[0].as_str()).collect();
        let blank = labels.iter().position(|label| label.is_empty()).unwrap();
        assert!(
            labels[..blank].iter().any(|label| label.contains("alary")),
            "{labels:?}"
        );
        assert_eq!(labels[blank + 1], "Spending");
        assert!(
            rows.iter().all(|cells| cells[1] != "$0"),
            "no line for what the year did not pay: {labels:?}"
        );
    }

    #[test]
    fn detail_follows_the_basis() {
        let projected = test_projected();
        let later = &projected.projection.years[5];
        let plan = &projected.plan;
        assert_ne!(
            detail_rows(later, plan, true),
            detail_rows(later, plan, false)
        );
    }
}
