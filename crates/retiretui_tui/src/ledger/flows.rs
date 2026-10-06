//! The Ledger's Flows pane: each account's year from its open to its
//! close, with what came in and went out named by where from or to, and
//! every account as one beneath them; as tall as its rows, and giving them
//! up before the money under it loses its own.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Local, Query, With};
use bevy_ui::{Node, Val};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use plurimus::widgets::TableColumns;
use retiretui_client::ledger::{AccountFlows, FLOW_COLUMNS, FLOWS, Year};

use super::arrange::DetailStop;
use super::{Detail, LedgerSystems, shows_the_year};
use crate::edit::table_bundle;
use crate::hints::Hints;
use crate::layout::{self, filling, placed};
use crate::pane::{self, Pane};
use crate::tabulate;

pub(super) fn plugin(app: &mut App) {
    let drawn = refresh.run_if(shows_the_year);
    app.add_systems(Update, drawn.in_set(LedgerSystems::Draw));
}

/// The cells between the columns, past the one the table leaves.
const FLOW_GAP: u16 = 1;
/// Moves take what the name and the figures leave, so a narrow pane clips
/// the end of a move rather than every column.
const MOVES: usize = 2;
/// The rows the pane keeps, borders included, however short the page.
const FLOWS_LEAST: f32 = 5.0;
const HEADER_ROWS: usize = 1;

#[derive(Component)]
struct FlowsTable;

pub(super) fn spawn_pane(commands: &mut Commands, detail: Entity) {
    let pane = Pane::new(FLOWS)
        .tall(f32::from(pane::BORDERS))
        .shrinking_to(FLOWS_LEAST)
        .spawn(commands, detail);
    commands.spawn((
        table_bundle(),
        FlowsTable,
        DetailStop,
        Hints(&[("↑↓", "account")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(pane),
    ));
}

/// A row per account the year touches, in the plan's order, a line more
/// under it for each further move, and then every account as one; the
/// growth beside its rate where `has_rate`.
fn flow_rows(year: &Year, has_rate: bool) -> Vec<Vec<String>> {
    let accounts = year.flows.iter();
    accounts
        .flat_map(|flows| account_lines(flows, has_rate))
        .collect()
}

/// The account's name, open, growth and close beside its first move, then
/// a line per further move, blank but for it.
fn account_lines(flows: &AccountFlows, has_rate: bool) -> Vec<Vec<String>> {
    let growth = if has_rate {
        &flows.growth_and_rate
    } else {
        &flows.growth
    };
    let first = flows.moves.first().cloned().unwrap_or_default();
    let figures = [&flows.account, &flows.open, &first, growth, &flows.close];
    let further = flows.moves.iter().skip(1).map(|moved| {
        let mut line = vec![String::new(); FLOW_COLUMNS.len()];
        line[MOVES].clone_from(moved);
        line
    });
    let figures = figures.into_iter().cloned().collect();
    std::iter::once(figures).chain(further).collect()
}

/// The columns as wide as what they hold, and the cells all of them take
/// with the one the table leaves between each.
fn measured(header: &[String], rows: &[Vec<String>]) -> (Vec<Constraint>, u16) {
    let TableColumns(widths) = tabulate::columns((header, rows), FLOW_GAP);
    let held = widths.iter().map(|width| match width {
        Constraint::Length(cells) => *cells,
        _ => 0,
    });
    let between = u16::try_from(widths.len().saturating_sub(1)).unwrap_or(u16::MAX);
    let cells = held.sum::<u16>().saturating_add(between);
    (widths, cells)
}

/// The year's rows and their columns in `width` cells: the growth beside
/// its rate where every move's name still fits, and alone where not.
fn fitted(year: &Year, header: &[String], width: u16) -> (Vec<Vec<String>>, Vec<Constraint>) {
    let rows = flow_rows(year, true);
    let (widths, cells) = measured(header, &rows);
    if cells <= width {
        return (rows, widths);
    }
    let rows = flow_rows(year, false);
    let (widths, _) = measured(header, &rows);
    (rows, widths)
}

/// Rewrites the rows whenever the year said or the pane's width moves:
/// the growth loses its rate before a move's name is clipped, and the pane
/// is as tall as its rows.
fn refresh(
    detail: Detail,
    mut drawn: Local<Option<u16>>,
    mut tables: Query<(Entity, &ChildOf, &mut ScrollArea, &ComputedWidgetArea), With<FlowsTable>>,
    mut panes: Query<&mut Node>,
    mut commands: Commands,
) {
    let Ok((table, pane, mut scroll, area)) = tables.single_mut() else {
        return;
    };
    let width = layout::row_width(*scroll, *area);
    let is_resized = drawn.replace(width) != Some(width);
    let Some((year, _)) = detail.due(is_resized) else {
        return;
    };
    let header = FLOW_COLUMNS.map(|(header, _)| header.to_owned());
    let (rows, mut widths) = fitted(year, &header, width);
    widths[MOVES] = Constraint::Fill(1);
    let text: Vec<usize> = (FLOW_COLUMNS.iter().enumerate())
        .filter_map(|(at, &(_, is_figure))| (!is_figure).then_some(at))
        .collect();
    commands.entity(table).insert(TableColumns(widths));
    tabulate::refill(
        &mut commands,
        (table, &mut *scroll),
        (&header, &rows),
        &text,
    );
    let lines = u16::try_from(rows.len() + HEADER_ROWS).unwrap_or(u16::MAX);
    let height = Val::Px(f32::from(lines + pane::BORDERS));
    if let Ok(mut node) = panes.get_mut(pane.parent())
        && node.height != height
    {
        node.height = height;
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::ledger::Asked;
    use retiretui_engine::params::TaxTables;
    use retiretui_engine::project::{Action, ContributionNote};

    use super::*;
    use crate::support::test_projected;

    /// The test plan's second year with one of every kind of action.
    fn busy_year() -> Year {
        let mut projected = test_projected();
        projected.projection.years[1].actions = vec![
            Action::Contribution {
                account: "k".to_owned(),
                employee: 20_000,
                employer: 5_000,
                notes: vec![
                    ContributionNote::Maximum,
                    ContributionNote::Match {
                        rate: 0.5,
                        up_to: 0.06,
                        of: "salary".to_owned(),
                    },
                ],
            },
            Action::Conversion {
                from: "k".to_owned(),
                to: "cash".to_owned(),
                amount: 7_000,
            },
            Action::Rmd {
                account: "k".to_owned(),
                amount: 3_000,
            },
            Action::Withdrawal {
                account: "cash".to_owned(),
                amount: 1_000,
            },
            Action::Surplus {
                account: "cash".to_owned(),
                amount: 500,
            },
        ];
        let asked = Asked {
            year: 2027,
            is_nominal: true,
            run: None,
        };
        Year::new(&projected, &TaxTables::embedded(), asked).unwrap()
    }

    #[test]
    fn an_account_takes_a_line_per_move_named_by_where_it_went() {
        let rows = flow_rows(&busy_year(), true);
        let moves: Vec<&str> = rows.iter().map(|line| &*line[MOVES]).collect();
        assert_eq!(
            moves[..7],
            [
                "+$7,000 ← k (conversion)",
                "+$500 surplus",
                "-$1,000 for spending",
                "+$20,000 yours · the maximum",
                "+$5,000 employer · 50% match up to 6% of salary",
                "-$7,000 → cash (conversion)",
                "-$3,000 RMD",
            ],
            "{rows:?}"
        );
        assert_eq!((&*rows[0][0], &*rows[3][0]), ("cash", "k"));
        for further in [1, 2, 4, 5, 6] {
            let line = &rows[further];
            let blank = [0, 1, 3, 4].iter().all(|&at| line[at].is_empty());
            assert!(blank, "a further move stands alone: {line:?}");
        }
        assert_eq!(rows.last().unwrap()[0], "All accounts");
        assert_eq!(rows.len(), 8, "{rows:?}");
    }

    #[test]
    fn growth_is_said_beside_its_rate_only_where_there_is_room() {
        let year = busy_year();
        let grown = |has_rate| flow_rows(&year, has_rate)[3][3].clone();
        let (with, without) = (grown(true), grown(false));
        assert!(with.starts_with(&without) && with.ends_with('%'), "{with}");
        assert!(
            without.starts_with("+$") && !without.contains('%'),
            "{without}"
        );
        let header = FLOW_COLUMNS.map(|(header, _)| header.to_owned());
        let (_, wide) = measured(&header, &flow_rows(&year, true));
        let (widths, narrow) = measured(&header, &flow_rows(&year, false));
        assert!(narrow < wide, "{narrow} against {wide}");
        assert_eq!(widths.len(), FLOW_COLUMNS.len());
        let has_rate = |width| fitted(&year, &header, width).0[3][3].ends_with('%');
        assert!(has_rate(wide), "what fits exactly keeps its rate");
        assert!(!has_rate(wide - 1), "a move's name outranks the rate");
        assert!(!has_rate(0));
    }
}
