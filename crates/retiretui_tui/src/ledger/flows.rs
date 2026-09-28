//! The Ledger's Flows pane: each account's year from its open to its
//! close, with what came in and went out and from or to where, over the
//! year's warnings.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, Res, With};
use bevy_ui::Node;
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::ScrollArea;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use plurimus::widgets::{TableColumns, WidgetSystems};
use retiretui_client::ledger::{FLOW_HEADERS, FLOWS, account_flows};
use retiretui_engine::plan::Plan;
use retiretui_engine::project::YearRow;

use crate::actions::collect_warnings;

use super::super::edit::table_bundle;
use super::super::hints::Hints;
use super::super::layout::{self, filling, fixed, placed};
use super::super::nav::FocusStop;
use super::super::overview::WARNING_MARK;
use super::super::pane::{Framed, Pane};
use super::super::present;
use super::super::session::{Session, Shown};
use super::super::tabulate;
use super::super::theme::{Repainted, Theme};
use super::DETAIL_GAP;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (refresh_flows, refresh_warnings)
            .in_set(super::split::DetailFilled)
            .before(Repainted)
            .before(WidgetSystems::Layout),
    );
}

/// In and Out share what the name and the figures leave, so a narrow pane
/// clips the end of a flow rather than every column.
const FLOW_COLUMNS: [usize; 2] = [2, 3];

#[derive(Component)]
struct FlowsPane;

#[derive(Component)]
struct FlowsTable;

#[derive(Component)]
struct FlowWarnings;

pub(super) fn spawn_pane(commands: &mut Commands, parent: Entity) {
    let pane = Pane::new(FLOWS).sharing(1.0).spawn(commands, parent);
    commands.entity(pane).insert(FlowsPane);
    commands.spawn((
        table_bundle(),
        FlowsTable,
        FocusStop,
        Hints(&[("↑↓", "account")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(pane),
    ));
    commands.spawn((
        FlowWarnings,
        fixed(0.0),
        UiWidget::default(),
        placed(),
        ChildOf(pane),
    ));
}

fn refresh_flows(
    shown: Shown,
    mut tables: Query<(Entity, &mut ScrollArea), With<FlowsTable>>,
    mut panes: Query<&mut Framed, With<FlowsPane>>,
    mut commands: Commands,
) {
    if !shown.is_changed() {
        return;
    }
    let Some(row) = shown.row() else {
        return;
    };
    let previous = shown.ledger().projection.row(row.year - 1);
    let rows = flow_rows(&shown.ledger().plan, previous, row, shown.basis.nominal);
    let header = FLOW_HEADERS.map(|(header, _)| header.to_owned());
    let text: Vec<usize> = (FLOW_HEADERS.iter().enumerate())
        .filter_map(|(at, &(_, is_figure))| (!is_figure).then_some(at))
        .collect();
    let TableColumns(mut widths) = tabulate::columns((&header, &rows), DETAIL_GAP);
    for at in FLOW_COLUMNS {
        widths[at] = Constraint::Fill(1);
    }
    for (table, mut scroll) in &mut tables {
        commands.entity(table).insert(TableColumns(widths.clone()));
        let scrolled = (table, &mut *scroll);
        tabulate::refill(&mut commands, scrolled, (&header, &rows), &text);
    }
    let title = format!(
        "{} {FLOWS} · {}",
        row.year,
        present::basis_name(shown.basis.nominal)
    );
    for mut framed in &mut panes {
        Framed::retitle(&mut framed, &title);
    }
}

fn refresh_warnings(
    shown: Shown,
    session: Res<Session>,
    theme: Res<Theme>,
    mut texts: Query<(&mut UiWidget, &mut Node), With<FlowWarnings>>,
) {
    if !shown.is_changed() && !theme.is_changed() {
        return;
    }
    let Some(row) = shown.row() else {
        return;
    };
    let ledger = shown.ledger();
    let deflating = (!shown.basis.nominal).then_some(&ledger.projection);
    let warnings = collect_warnings(&ledger.plan, &session.tables, row, deflating);
    let lines: Vec<Line<'static>> = warnings
        .into_iter()
        .map(|warning| Line::styled(format!("{WARNING_MARK}{warning}"), theme.exceeded()))
        .collect();
    let height = f32::from(u16::try_from(lines.len()).unwrap_or(u16::MAX));
    for (mut widget, mut node) in &mut texts {
        *node = fixed(height);
        *widget = UiWidget::new(Paragraph::new(lines.clone()));
    }
}

/// A row per account the year touches, in the plan's order, and a line
/// more under it for each further flow in or out.
fn flow_rows(
    plan: &Plan,
    previous: Option<&YearRow>,
    row: &YearRow,
    is_nominal: bool,
) -> Vec<Vec<String>> {
    account_flows(plan, previous, row, is_nominal)
        .into_iter()
        .flat_map(|flows| {
            let figures = [flows.account, flows.open, flows.growth, flows.close];
            account_lines(figures, flows.ins, flows.outs)
        })
        .collect()
}

/// The account's name, open, growth and close beside its first flow in and
/// out, then a line per further flow, blank but for them.
fn account_lines(figures: [String; 4], ins: Vec<String>, outs: Vec<String>) -> Vec<Vec<String>> {
    let lines = ins.len().max(outs.len()).max(1);
    let mut figures = Some(figures);
    let mut ins = ins.into_iter();
    let mut outs = outs.into_iter();
    (0..lines)
        .map(|_| {
            let [name, open, growth, close] = figures.take().unwrap_or_default();
            let (came, went) = (ins.next(), outs.next());
            vec![
                name,
                open,
                came.unwrap_or_default(),
                went.unwrap_or_default(),
                growth,
                close,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use retiretui_engine::params::TaxTables;
    use retiretui_engine::project::{Action, ContributionNote, deflate};

    use super::super::super::support::{TEST_PLAN, projected_from, test_projected};
    use super::*;
    use crate::table::money;

    fn busy_year(row: &YearRow) -> YearRow {
        let mut row = row.clone();
        row.actions = vec![
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
        row
    }

    #[test]
    fn a_warning_names_its_amount_in_the_basis_shown() {
        let short = TEST_PLAN.replace("amount = 60000", "amount = 600000");
        let projected = projected_from(&short);
        let row = &projected.projection.years[5];
        assert!(row.unfunded > 0, "a year that runs short");
        let tables = TaxTables::embedded();
        let deflating = Some(&projected.projection);
        let said = collect_warnings(&projected.plan, &tables, row, deflating).join("\n");
        let deflated = money(deflate(row.unfunded, row.deflator));
        assert!(said.contains(&deflated), "{said}");
        assert!(!said.contains(&money(row.unfunded)), "{said}");
    }

    #[test]
    fn an_account_takes_a_line_per_flow_named_by_where_it_went() {
        let projected = test_projected();
        let years = &projected.projection.years;
        let row = busy_year(&years[1]);
        let rows = flow_rows(&projected.plan, Some(&years[0]), &row, true);
        let texts: Vec<[&str; 2]> = rows.iter().map(|line| [&*line[2], &*line[3]]).collect();
        assert_eq!(
            texts,
            [
                ["+$7,000 ← k (conversion)", "-$1,000 for spending"],
                ["+$500 surplus", ""],
                [
                    "+$20,000 yours · the maximum",
                    "-$7,000 → cash (conversion)"
                ],
                [
                    "+$5,000 employer · 50% match up to 6% of salary",
                    "-$3,000 RMD"
                ],
            ],
            "{rows:?}"
        );
        assert_eq!(rows[0][0], "cash");
        assert!(rows[0][4].is_empty(), "no growth is left blank");
        assert!(rows[1][0].is_empty() && rows[1][1].is_empty() && rows[1][5].is_empty());
        assert_eq!(rows[2][0], "k");
        assert_eq!(rows[2][1], present::money(years[0].balances["k"]));
        assert_eq!(rows[2][4], format!("+{}", present::money(row.growth["k"])));
    }

    #[test]
    fn every_action_of_every_year_is_a_flow() {
        let projected = projected_from(include_str!(
            "../../../retiretui_engine/tests/fixtures/full.toml"
        ));
        let years = &projected.projection.years;
        for (at, row) in years.iter().enumerate() {
            let previous = at.checked_sub(1).map(|before| &years[before]);
            let rows = flow_rows(&projected.plan, previous, row, true);
            let flows: Vec<&str> = rows
                .iter()
                .flat_map(|line| [&*line[2], &*line[3]])
                .collect();
            let amounts = row.actions.iter().flat_map(|action| match action {
                Action::Contribution {
                    employee, employer, ..
                } => vec![*employee, *employer],
                Action::Transfer { amount, .. }
                | Action::Conversion { amount, .. }
                | Action::Rmd { amount, .. }
                | Action::Withdrawal { amount, .. }
                | Action::Surplus { amount, .. } => vec![*amount],
            });
            for amount in amounts.filter(|&amount| amount > 0) {
                let said = present::money(amount);
                assert!(
                    flows.iter().any(|flow| flow.contains(&said)),
                    "{} {said}: {flows:?}",
                    row.year
                );
            }
        }
    }

    #[test]
    fn the_first_year_opens_on_the_plan_and_figures_follow_the_basis() {
        let projected = test_projected();
        let years = &projected.projection.years;
        let first = flow_rows(&projected.plan, None, &years[0], true);
        assert_eq!(first[1][1], "$200,000", "{first:?}");
        let later = &years[5];
        assert_ne!(
            flow_rows(&projected.plan, Some(&years[4]), later, true),
            flow_rows(&projected.plan, Some(&years[4]), later, false)
        );
    }
}
