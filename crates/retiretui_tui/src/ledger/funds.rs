//! The three panes under the flows: what the cursor year lived on, where
//! that went, and its tax - each a label beside its amount, a blank line
//! between the groups, the amounts whole where a narrow pane clips.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::ScrollArea;
use plurimus::widgets::{ActiveDescendant, TableColumns, table_row};
use retiretui_client::ledger::{DetailLine, Funds, MONEY_IN, MONEY_OUT, TAX, Year};

use super::arrange::DetailStop;
use super::{Detail, LedgerSystems};
use crate::edit::table_bundle;
use crate::hints::Hints;
use crate::layout::{self, filling, placed};
use crate::nav::Page;
use crate::pane::Pane;
use crate::tools::{EnterRuns, handle_enter};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, refresh.in_set(LedgerSystems::Draw));
}

/// One of the three panes, by what it says of the year.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Said {
    MoneyIn,
    MoneyOut,
    Tax,
}

const PANES: [(Said, &str); 3] = [
    (Said::MoneyIn, MONEY_IN),
    (Said::MoneyOut, MONEY_OUT),
    (Said::Tax, TAX),
];

pub(super) fn spawn_panes(commands: &mut Commands, row: Entity) {
    for (said, title) in PANES {
        let pane = Pane::new(title).sharing(1.0).spawn(commands, row);
        let mut table = commands.spawn((
            table_bundle(),
            said,
            DetailStop,
            Hints(&[("↑↓", "line")]),
            layout::Rests,
            filling(),
            placed(),
            ChildOf(pane),
        ));
        if said == Said::Tax {
            let tables = Page::TaxTables.label();
            table
                .insert((
                    EnterRuns(tables),
                    Hints(&[("↑↓", "line"), ("⏎", "tax tables")]),
                ))
                .observe(handle_enter);
        }
    }
}

type Row = Option<DetailLine>;

/// A side's groups, a blank line between them, and then all of it.
fn funds_rows(funds: &Funds) -> Vec<Row> {
    let mut rows = Vec::new();
    for group in &funds.groups {
        rows.extend(group.lines.iter().cloned().map(Some));
        rows.extend(group.subtotal.clone().map(Some));
        rows.push(None);
    }
    rows.push(Some(funds.total.clone()));
    rows
}

/// What was paid, then the bracket reached and what the tax was worked
/// out from.
fn tax_rows(year: &Year) -> Vec<Row> {
    let paid = year.tax.iter().cloned().map(Some);
    let worked = year.bracket.iter().chain(&year.picture).cloned().map(Some);
    paid.chain([None]).chain(worked).collect()
}

impl Said {
    fn rows(self, year: &Year) -> Vec<Row> {
        match self {
            Self::MoneyIn => funds_rows(&year.money_in),
            Self::MoneyOut => funds_rows(&year.money_out),
            Self::Tax => tax_rows(year),
        }
    }
}

/// Rewrites each pane whenever the year said moves. The label gives way to
/// the amount, which is as wide as the widest of them.
fn refresh(
    mut detail: Detail,
    mut tables: Query<(Entity, &Said, &mut ScrollArea)>,
    mut commands: Commands,
) {
    let Some(year) = detail.due(false) else {
        return;
    };
    for (table, said, mut scroll) in &mut tables {
        let rows = said.rows(year);
        let amounts = rows
            .iter()
            .flatten()
            .map(|line| line.amount.chars().count());
        let widest = u16::try_from(amounts.max().unwrap_or(0)).unwrap_or(u16::MAX);
        let columns = vec![Constraint::Fill(1), Constraint::Length(widest)];
        commands.entity(table).despawn_related::<Children>();
        let mut first = None;
        for line in &rows {
            let cells = line.as_ref().map_or_else(Vec::new, |line| {
                let amount = Line::from(line.amount.clone()).right_aligned();
                vec![Line::from(line.label.clone()), amount]
            });
            let row = commands.spawn((table_row(cells), ChildOf(table))).id();
            first.get_or_insert(row);
        }
        commands
            .entity(table)
            .insert((TableColumns(columns), ActiveDescendant(first)));
        scroll.content_size.height = u16::try_from(rows.len()).unwrap_or(u16::MAX);
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::ledger::Asked;
    use retiretui_engine::params::TaxTables;

    use super::*;
    use crate::support::projected_from;

    const FULL: &str = include_str!("../../../retiretui_engine/tests/fixtures/full.toml");

    fn labels(rows: &[Row]) -> Vec<&str> {
        let labelled = rows.iter().map(|row| row.as_ref());
        labelled
            .map(|row| row.map_or("", |line| line.label.as_str()))
            .collect()
    }

    #[test]
    fn each_pane_lists_its_groups_apart_and_ends_on_what_they_come_to() {
        let projected = projected_from(FULL);
        let asked = Asked {
            year: 2045,
            is_nominal: true,
            is_run: false,
        };
        let year = Year::new(&projected, &TaxTables::embedded(), asked).unwrap();
        let money_in = Said::MoneyIn.rows(&year);
        assert_eq!(
            labels(&money_in),
            [
                "db-pension",
                "ss-jordan",
                "Income",
                "",
                "From brokerage",
                "From fid-401k",
                "Withdrawn",
                "",
                "Total"
            ]
        );
        let money_out = Said::MoneyOut.rows(&year);
        assert_eq!(labels(&money_out), ["Spending", "", "Tax", "", "Total"]);
        assert_eq!(money_in.last(), money_out.last(), "the sides agree");
        let tax = labels(&Said::Tax.rows(&year)).join("|");
        assert!(
            tax.starts_with("Ordinary|State|Capital gains|Total||To top of "),
            "{tax}"
        );
        assert!(tax.ends_with("|Tax over MAGI"), "{tax}");
    }
}
