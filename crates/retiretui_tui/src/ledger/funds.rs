//! The three panes under the flows: what the cursor year lived on, where
//! that went, and its tax - each a label beside its amount, what the kinds
//! of them come to dimmed beneath, the amounts whole where a narrow pane
//! clips.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, Res, With};
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::ui::{ScrollArea, UiStyle};
use plurimus::widgets::TableColumns;
use retiretui_client::ledger::{DetailLine, Funds, MONEY_IN, MONEY_OUT, TAX, Year};

use super::arrange::{DetailStop, Roomy};
use super::{Detail, LedgerSystems, shows_the_year};
use crate::edit::table_bundle;
use crate::hints::Hints;
use crate::layout::{self, filling, placed};
use crate::nav::Page;
use crate::pane::{self, Pane};
use crate::tabulate;
use crate::tools::{EnterRuns, handle_enter};

pub(super) fn plugin(app: &mut App) {
    let drawn = refresh.run_if(shows_the_year);
    app.add_systems(Update, drawn.in_set(LedgerSystems::Draw));
}

/// Where a row's amount is among its cells.
const AMOUNT: usize = 1;

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

/// The rows the three panes keep, borders included, however many
/// accounts the year touches.
const FUNDS_LEAST: f32 = 7.0;

/// The row the three panes stand in.
#[derive(Component)]
struct FundsRow;

pub(super) fn spawn_panes(commands: &mut Commands, detail: Entity) {
    let row = Node {
        flex_direction: FlexDirection::Row,
        min_height: Val::Px(FUNDS_LEAST),
        ..Node::default()
    };
    let row = commands.spawn((row, FundsRow, ChildOf(detail))).id();
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

/// What was paid, then what the tax was worked out from.
fn tax_rows(year: &Year) -> Vec<Row> {
    let paid = year.tax.iter().cloned().map(Row::Line);
    let worked = year.worked_from.iter().cloned().map(Row::Line);
    paid.chain([Row::Gap]).chain(worked).collect()
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

/// The row as tall as its longest pane where the histories stand under
/// it, and taking what the page leaves where they do not.
fn fit(node: &mut Node, longest: usize, is_roomy: bool) {
    let rows = u16::try_from(longest)
        .unwrap_or(u16::MAX)
        .saturating_add(pane::BORDERS);
    let (height, grow, basis) = if is_roomy {
        (Val::Px(f32::from(rows)), 0.0, Val::Auto)
    } else {
        (Val::Auto, 1.0, Val::Px(0.0))
    };
    (node.height, node.flex_grow, node.flex_basis) = (height, grow, basis);
}

/// Rewrites each pane whenever the year said moves, or the page gains or
/// loses the room for the histories. The label gives way to the amount,
/// which is as wide as the widest of them.
fn refresh(
    (detail, roomy): (Detail, Res<Roomy>),
    mut tables: Query<(Entity, &Said, &mut ScrollArea)>,
    mut funds_rows: Query<&mut Node, With<FundsRow>>,
    mut commands: Commands,
) {
    let Some((year, theme)) = detail.due(roomy.is_changed()) else {
        return;
    };
    let dimmed = theme.dimmed();
    let mut longest = 0;
    for (table, said, mut scroll) in &mut tables {
        let rows = said.rows(year);
        longest = longest.max(rows.len());
        let cells = |row: &Row| {
            let line = row.line();
            line.map_or_else(Vec::new, |line| {
                vec![line.label.clone(), line.amount.clone()]
            })
        };
        let cells: Vec<Vec<String>> = rows.iter().map(cells).collect();
        let amounts = cells.iter().filter_map(|row| row.get(AMOUNT));
        let widest = amounts.map(|amount| amount.chars().count()).max();
        let widest = u16::try_from(widest.unwrap_or(0)).unwrap_or(u16::MAX);
        let columns = vec![Constraint::Fill(1), Constraint::Length(widest)];
        commands.entity(table).insert(TableColumns(columns));
        let spawned = tabulate::refill(&mut commands, (table, &mut *scroll), (&[], &cells), &[0]);
        for (row, entity) in rows.iter().zip(spawned) {
            if matches!(row, Row::Sum(_)) {
                commands.entity(entity).insert(UiStyle(dimmed));
            }
        }
    }
    for mut node in &mut funds_rows {
        fit(&mut node, longest, roomy.0);
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
            run: None,
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
