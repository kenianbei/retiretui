//! The three panes under the flows: what the cursor year lived on, where
//! that went, and its tax - each a label beside its amount, what the kinds
//! of them come to dimmed beneath, the amounts whole where a narrow pane
//! clips.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ScrollArea, UiStyle};
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

/// A row of a pane: an amount, an amount said dimmed beneath the rest as
/// their summary, or the gap between two parts.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Row {
    Line(DetailLine),
    Sum(DetailLine),
    Gap,
}

impl Row {
    fn line(&self) -> Option<&DetailLine> {
        match self {
            Self::Line(line) | Self::Sum(line) => Some(line),
            Self::Gap => None,
        }
    }
}

/// A side's amounts in one list, then what each kind comes to, dimmed, and
/// all of it.
fn funds_rows(funds: &Funds) -> Vec<Row> {
    let lines = funds.lines.iter().cloned().map(Row::Line);
    let sums = funds.sums.iter().cloned().map(Row::Sum);
    let total = Row::Line(funds.total.clone());
    lines.chain([Row::Gap]).chain(sums).chain([total]).collect()
}

/// What was paid, then the bracket reached and what the tax was worked
/// out from.
fn tax_rows(year: &Year) -> Vec<Row> {
    let paid = year.tax.iter().cloned().map(Row::Line);
    let worked = year.bracket.iter().chain(&year.picture).cloned();
    paid.chain([Row::Gap])
        .chain(worked.map(Row::Line))
        .collect()
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
    let dimmed = detail.theme.dimmed();
    let Some(year) = detail.due(false) else {
        return;
    };
    for (table, said, mut scroll) in &mut tables {
        let rows = said.rows(year);
        let amounts = rows.iter().filter_map(Row::line);
        let amounts = amounts.map(|line| line.amount.chars().count());
        let widest = u16::try_from(amounts.max().unwrap_or(0)).unwrap_or(u16::MAX);
        let columns = vec![Constraint::Fill(1), Constraint::Length(widest)];
        commands.entity(table).despawn_related::<Children>();
        let mut first = None;
        for row in &rows {
            let cells = row.line().map_or_else(Vec::new, |line| {
                let amount = Line::from(line.amount.clone()).right_aligned();
                vec![Line::from(line.label.clone()), amount]
            });
            let mut spawned = commands.spawn((table_row(cells), ChildOf(table)));
            if matches!(row, Row::Sum(_)) {
                spawned.insert(UiStyle(dimmed));
            }
            first.get_or_insert(spawned.id());
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
        let label = |row| Row::line(row).map_or("", |line| line.label.as_str());
        rows.iter().map(label).collect()
    }

    #[test]
    fn each_pane_is_one_list_over_what_its_kinds_come_to_and_all_of_it() {
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
                "From brokerage",
                "From fid-401k",
                "",
                "Income",
                "Withdrawn",
                "Total"
            ]
        );
        let dimmed = |rows: &[Row]| rows.iter().filter(|row| matches!(row, Row::Sum(_))).count();
        assert_eq!(dimmed(&money_in), 2, "what each kind comes to is a summary");
        let money_out = Said::MoneyOut.rows(&year);
        assert_eq!(
            labels(&money_out),
            ["living", "travel", "Tax", "", "Spending", "Total"]
        );
        assert_eq!(dimmed(&money_out), 1);
        assert_eq!(money_in.last(), money_out.last(), "the sides agree");
        let tax = labels(&Said::Tax.rows(&year)).join("|");
        assert!(
            tax.starts_with("Ordinary|State|Capital gains|Total||To top of "),
            "{tax}"
        );
        assert!(tax.ends_with("|Tax over MAGI"), "{tax}");
    }
}
