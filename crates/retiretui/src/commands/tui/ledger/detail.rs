//! The Ledger's Income & Tax pane: what the cursor year brought in, and
//! where it went besides the accounts.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, With};
use plurimus::ui::ScrollArea;
use plurimus::widgets::WidgetSystems;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::YearRow;

use crate::commands::table::basis_amount;

use super::super::edit::table_bundle;
use super::super::hints::Hints;
use super::super::layout::{self, filling, placed};
use super::super::nav::FocusStop;
use super::super::pane::Pane;
use super::super::present;
use super::super::session::Shown;
use super::super::tabulate;
use super::DETAIL_GAP;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, refresh_detail.before(WidgetSystems::Layout));
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
    let line = |label: &str, amount: Dollars| {
        let amount = basis_amount(amount, row.deflator, is_nominal);
        vec![label.to_owned(), present::money(amount)]
    };
    let income = row
        .income
        .iter()
        .map(|(source, &amount)| line(present::income_name(plan, source), amount));
    let paid = [
        ("Spending", row.expenses),
        ("Ordinary tax", row.taxes.ordinary),
        ("State tax", row.taxes.state),
        ("Capital gains", row.taxes.ltcg),
        ("Penalties", row.taxes.penalty),
        ("Medicare", row.medicare),
        ("Surplus", row.surplus),
        ("Unfunded", row.unfunded),
    ]
    .map(|(label, amount)| line(label, amount));
    let mut rows: Vec<Vec<String>> = income.collect();
    if !rows.is_empty() {
        rows.push(vec![String::new(); 2]);
    }
    rows.extend(paid);
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
        assert_eq!(labels.last(), Some(&"Unfunded"));
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
