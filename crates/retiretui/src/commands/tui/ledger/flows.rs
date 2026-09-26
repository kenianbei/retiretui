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
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{Action, ContributionNote, YearRow};

use crate::commands::actions::{collect_warnings, note_phrase};
use crate::commands::table::basis_amount;

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

const TITLE: &str = "Flows";
const HEADER: [&str; 6] = ["Account", "Open", "In", "Out", "Growth", "Close"];
/// The columns written as text, lined up on the left; the rest are figures.
const TEXT_COLUMNS: [usize; 3] = [0, 2, 3];
/// In and Out share what the name and the figures leave, so a narrow pane
/// clips the end of a flow rather than every column.
const FLOW_COLUMNS: [usize; 2] = [2, 3];
const NOTE_JOIN: &str = " · ";
/// Said after a conversion's counterpart, where a narrow pane clips first.
const CONVERSION: &str = " (conversion)";

#[derive(Component)]
struct FlowsPane;

#[derive(Component)]
struct FlowsTable;

#[derive(Component)]
struct FlowWarnings;

pub(super) fn spawn_pane(commands: &mut Commands, parent: Entity) {
    let pane = Pane::new(TITLE).sharing(1.0).spawn(commands, parent);
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
    let years = &shown.ledger().projection.years;
    // Years are contiguous from the plan's start, so the offset indexes.
    let at = usize::try_from(row.year - years[0].year).unwrap_or(0);
    let previous = at.checked_sub(1).map(|before| &years[before]);
    let rows = flow_rows(&shown.ledger().plan, previous, row, shown.basis.nominal);
    let header = HEADER.map(str::to_owned);
    let TableColumns(mut widths) = tabulate::columns((&header, &rows), DETAIL_GAP);
    for at in FLOW_COLUMNS {
        widths[at] = Constraint::Fill(1);
    }
    for (table, mut scroll) in &mut tables {
        commands.entity(table).insert(TableColumns(widths.clone()));
        let scrolled = (table, &mut *scroll);
        tabulate::refill(&mut commands, scrolled, (&header, &rows), &TEXT_COLUMNS);
    }
    let title = format!(
        "{} {TITLE} · {}",
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
/// more under it for each further flow in or out. Every figure is
/// deflated by `row`'s own year, so an account's line adds up.
fn flow_rows(
    plan: &Plan,
    previous: Option<&YearRow>,
    row: &YearRow,
    is_nominal: bool,
) -> Vec<Vec<String>> {
    let show = |amount: Dollars| present::money(basis_amount(amount, row.deflator, is_nominal));
    let mut rows = Vec::new();
    for account in &plan.accounts {
        let id = account.id.as_str();
        let open = previous.map_or(account.balance, |before| balance(before, id));
        let close = balance(row, id);
        let growth = row.growth.get(id).copied().unwrap_or(0);
        let (ins, outs) = moves(plan, row, id, &show);
        if open == 0 && close == 0 && growth == 0 && ins.is_empty() && outs.is_empty() {
            continue;
        }
        let figures = [
            account.display_name().to_owned(),
            show(open),
            signed(growth, &show),
            show(close),
        ];
        rows.extend(account_lines(figures, ins, outs));
    }
    rows
}

fn balance(row: &YearRow, id: &str) -> Dollars {
    row.balances.get(id).copied().unwrap_or(0)
}

fn signed(amount: Dollars, show: &impl Fn(Dollars) -> String) -> String {
    match amount {
        0 => String::new(),
        ..0 => show(amount),
        _ => format!("+{}", show(amount)),
    }
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

/// What came into the account `id` and what went out of it, each named by
/// where it came from or went.
fn moves(
    plan: &Plan,
    row: &YearRow,
    id: &str,
    show: &impl Fn(Dollars) -> String,
) -> (Vec<String>, Vec<String>) {
    let mut ins = Vec::new();
    let mut outs = Vec::new();
    for action in &row.actions {
        let between = match action {
            Action::Transfer { from, to, amount } => Some((from, to, amount, "")),
            Action::Conversion { from, to, amount } => Some((from, to, amount, CONVERSION)),
            _ => None,
        };
        if let Some((from, to, amount, kind)) = between {
            if to == id {
                let from = present::account_name(plan, from);
                ins.push(format!("+{} ← {from}{kind}", show(*amount)));
            }
            if from == id {
                let to = present::account_name(plan, to);
                outs.push(format!("-{} → {to}{kind}", show(*amount)));
            }
            continue;
        }
        match action {
            Action::Contribution {
                account,
                employee,
                employer,
                notes,
            } if account == id => ins.extend(paid_in(plan, (*employee, *employer), notes, show)),
            Action::Rmd { account, amount } if account == id => {
                outs.push(format!("-{} RMD", show(*amount)));
            }
            Action::Withdrawal { account, amount } if account == id => {
                outs.push(format!("-{} for spending", show(*amount)));
            }
            Action::Surplus { account, amount } if account == id => {
                ins.push(format!("+{} surplus", show(*amount)));
            }
            _ => {}
        }
    }
    (ins, outs)
}

/// A contribution's lines: the employee's and the employer's, each with the
/// notes that say how it came to be.
fn paid_in(
    plan: &Plan,
    (yours, theirs): (Dollars, Dollars),
    notes: &[ContributionNote],
    show: &impl Fn(Dollars) -> String,
) -> Vec<String> {
    let with_notes = |said: String, is_employer: bool| {
        let phrases = notes
            .iter()
            .filter(|note| is_employer_note(note) == is_employer)
            .map(|note| note_phrase(plan, note));
        std::iter::once(said)
            .chain(phrases)
            .collect::<Vec<_>>()
            .join(NOTE_JOIN)
    };
    let mut lines = Vec::new();
    if yours > 0 {
        lines.push(with_notes(format!("+{} yours", show(yours)), false));
    }
    if theirs > 0 {
        lines.push(with_notes(format!("+{} employer", show(theirs)), true));
    }
    lines
}

const fn is_employer_note(note: &ContributionNote) -> bool {
    matches!(
        note,
        ContributionNote::Match { .. } | ContributionNote::HeldToOverall
    )
}

#[cfg(test)]
mod tests {
    use retiretui_engine::params::TaxTables;
    use retiretui_engine::project::deflate;

    use super::super::super::support::{TEST_PLAN, projected_from, test_projected};
    use super::*;
    use crate::commands::table::money;

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
            "../../../../../retiretui_engine/tests/fixtures/full.toml"
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
