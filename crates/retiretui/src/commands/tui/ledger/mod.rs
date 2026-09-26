use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{
    ChildOf, Commands, Component, Entity, IntoScheduleConfigs, Local, Or, Query, Res, ResMut, With,
};
use bevy_ecs::system::SystemParam;
use plurimus::core::TerminalSize;
use plurimus::core::ratatui_core::layout::{Constraint, Size};
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ScrollArea, UiStyle};
use plurimus::widgets::{
    ActiveDescendant, TableColumns, TableKeys, TableSelection, TableStripe, table, table_header,
    table_row, table_self_update,
};
use retiretui_engine::plan::{Dollars, Plan, TreatmentClass};
use retiretui_engine::project::YearRow;

use crate::commands::table::{Column, ages_text, basis_amount, present_classes, year_figures};

use super::layout::{self, Body, filling, placed};
use super::nav::{self, ActivePage, FocusStop, Page};
use super::pane::{Framed, Pane};
use super::present;
use super::session::{LedgerRun, Projected, RowYear, Shown, track_cursor};
use super::theme::Theme;

mod detail;
mod flows;
mod split;

pub fn plugin(app: &mut App) {
    app.add_plugins((detail::plugin, flows::plugin, split::plugin));
    app.add_systems(Startup, spawn_ledger.after(layout::spawn_frame));
    app.init_resource::<LedgerRun>();
    app.add_systems(
        Update,
        (
            leave_run_on_replan,
            title_ledger,
            rebuild_rows,
            follow_cursor,
            track_cursor::<LedgerTable>,
        )
            .chain(),
    );
    app.add_observer(table_self_update);
}

/// The cells between the detail tables' columns, past the one a table
/// leaves.
const DETAIL_GAP: u16 = 2;

#[derive(Component)]
pub struct LedgerTable;

#[derive(Component)]
struct LedgerHeaderRow;

const TITLE: &str = "Ledger";
const RETURN: &str = "esc returns to the plan";

/// The pane the years are listed in, whose title names a run it shows.
#[derive(Component)]
struct LedgerPane;

/// Names the run the Ledger shows in its title.
fn title_ledger(run: Res<LedgerRun>, mut panes: Query<&mut Framed, With<LedgerPane>>) {
    if !run.is_changed() {
        return;
    }
    let title = run.0.as_ref().map_or_else(
        || TITLE.to_owned(),
        |(label, _)| format!("{TITLE} · {label} · {RETURN}"),
    );
    for mut pane in &mut panes {
        Framed::retitle(&mut pane, &title);
    }
}

/// The `ledger-plan` command: the plan's own projection back in the
/// Ledger, in place of a run opened from a market tool.
pub fn return_to_plan(mut run: ResMut<LedgerRun>) -> super::command::Outcome {
    if run.0.is_some() {
        run.0 = None;
    }
    super::command::Outcome::Done
}

/// A run describes the plan it was drawn from, so a plan that changes
/// leaves it behind.
fn leave_run_on_replan(projected: Res<Projected>, mut run: ResMut<LedgerRun>) {
    if projected.is_changed() && run.0.is_some() && !run.is_changed() {
        run.0 = None;
    }
}

fn spawn_ledger(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let view = nav::spawn_surface(&mut commands, body, Some(Page::Ledger));
    let pane = Pane::new(TITLE).sharing(1.0).spawn(&mut commands, view);
    commands.entity(pane).insert(LedgerPane);
    commands.spawn((
        table([Constraint::Length(5)]),
        LedgerTable,
        TableSelection::Row,
        layout::table_cursor(),
        TableKeys::default(),
        TableStripe(Style::new()),
        ScrollArea::new(Size::default()),
        FocusStop,
        filling(),
        placed(),
        ChildOf(pane),
    ));
    let detail = split::spawn_detail(&mut commands, view);
    flows::spawn_pane(&mut commands, detail);
    detail::spawn_pane(&mut commands, detail);
}

#[derive(SystemParam)]
struct LedgerEntities<'w, 's> {
    tables: Query<'w, 's, Entity, With<LedgerTable>>,
    rows: Query<'w, 's, (Entity, &'static ChildOf), Or<(With<RowYear>, With<LedgerHeaderRow>)>>,
}

/// What the rows are built from, and what makes them stale.
#[derive(SystemParam)]
struct RowInputs<'w, 's> {
    shown: Shown<'w>,
    active: Res<'w, ActivePage>,
    theme: Res<'w, Theme>,
    size: Res<'w, TerminalSize>,
    is_stale: Local<'s, bool>,
    /// How the rows on screen were fitted, so a resize that fits the same
    /// way rebuilds nothing.
    fitted: Local<'s, Option<Fit>>,
}

impl RowInputs<'_, '_> {
    /// Rows are rebuilt while the ledger is on screen, and a change that
    /// lands while it is not waits for it: setting the cursor reveals it,
    /// and a hidden table is placed nowhere, so a reveal resolved against
    /// its empty area scrolls the rows out of view for good.
    fn should_rebuild(&mut self) -> bool {
        let is_refitted = self.size.is_changed() && *self.fitted != Some(self.fit());
        *self.is_stale |= self.shown.projected.is_changed()
            || self.shown.run.is_changed()
            || self.shown.basis.is_changed()
            || self.theme.is_changed()
            || is_refitted;
        let rebuilding = *self.is_stale && self.active.0 == Page::Ledger;
        *self.is_stale &= !rebuilding;
        rebuilding
    }

    fn fit(&self) -> Fit {
        let classes = present_classes(&self.shown.ledger().plan);
        let widest = widest_cell(&self.shown.ledger().projection.years, &classes);
        Fit::of(self.size.cols, classes.len(), widest)
    }
}

/// Respawns the header and year rows whenever the projection or the basis
/// changes, carrying the cursor across by year.
fn rebuild_rows(mut inputs: RowInputs, entities: LedgerEntities, mut commands: Commands) {
    if !inputs.should_rebuild() {
        return;
    }
    let Ok(ledger_table) = entities.tables.single() else {
        return;
    };
    for (row, table) in &entities.rows {
        if table.parent() == ledger_table {
            commands.entity(row).despawn();
        }
    }
    let fit = inputs.fit();
    *inputs.fitted = Some(fit);
    let mut classes = present_classes(&inputs.shown.ledger().plan);
    if !fit.has_classes {
        classes.clear();
    }
    let columns = class_columns(&classes);
    let style = RowStyle {
        plan: &inputs.shown.ledger().plan,
        is_nominal: inputs.shown.basis.nominal,
        is_compact: fit.is_compact,
        columns: &columns,
    };
    commands
        .entity(ledger_table)
        .insert(TableColumns(ledger_constraints(&classes)));
    commands.spawn((
        table_header(header_cells(&classes)),
        LedgerHeaderRow,
        UiStyle(Style::new().add_modifier(Modifier::BOLD)),
        ChildOf(ledger_table),
    ));
    let cursor_row = spawn_year_rows(&mut commands, ledger_table, &inputs, style);
    commands
        .entity(ledger_table)
        .insert(ActiveDescendant(cursor_row));
}

/// Points the table at a year set elsewhere, which, like the rows, waits
/// for the Ledger to be shown.
fn follow_cursor(
    shown: Shown,
    active: Res<ActivePage>,
    mut is_stale: Local<bool>,
    mut tables: Query<(Entity, &mut ActiveDescendant), With<LedgerTable>>,
    rows: Query<(Entity, &RowYear, &ChildOf)>,
) {
    *is_stale |= shown.is_changed();
    if !*is_stale || active.0 != Page::Ledger {
        return;
    }
    *is_stale = false;
    let Ok((table, mut active)) = tables.single_mut() else {
        return;
    };
    let year = shown.year();
    let row = rows
        .iter()
        .find(|(_, row_year, parent)| parent.parent() == table && row_year.0 == year);
    if let Some((row, ..)) = row {
        active.set_if_neq(ActiveDescendant(Some(row)));
    }
}

/// Spawns a row per projected year, answering the one the cursor belongs
/// on.
fn spawn_year_rows(
    commands: &mut Commands,
    ledger_table: Entity,
    inputs: &RowInputs,
    style: RowStyle,
) -> Option<Entity> {
    let cursor_year = inputs.shown.year();
    let mut cursor_row = None;
    for year_row in &inputs.shown.ledger().projection.years {
        let cells = ledger_cells(year_row, style);
        let id = commands
            .spawn((
                table_row(cells),
                RowYear(year_row.year),
                ChildOf(ledger_table),
            ))
            .id();
        if year_row.unfunded > 0 {
            commands.entity(id).insert(UiStyle(inputs.theme.exceeded()));
        }
        if cursor_year == year_row.year {
            cursor_row = Some(id);
        }
    }
    cursor_row
}

const YEAR_COLS: u16 = 5;
const AGE_COLS: u16 = 6;
/// Columns of the terminal the rows never get: the pane's borders, the
/// cursor mark, and the scroll bar.
const LEDGER_CHROME: u16 = 5;
/// Income, spending, tax, withdrawn and net worth, which every width shows.
const CORE_MONEY_COLUMNS: usize = 5;

/// How the ledger fits the width it has: whether the treatment classes get
/// columns of their own, and whether figures are written in full.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Fit {
    has_classes: bool,
    is_compact: bool,
}

impl Fit {
    /// The classes are the first to go, since the detail pane under the
    /// table lists every account; figures are compacted only where the
    /// widest of them would still run into its neighbour.
    /// `widest` is the most cells any money column has to show, header
    /// included.
    fn of(cols: u16, classes: usize, widest: usize) -> Self {
        let room = usize::from(cols.saturating_sub(LEDGER_CHROME + YEAR_COLS + AGE_COLS));
        let per_column = |columns: usize| (room / columns).saturating_sub(1);
        let has_classes = per_column(CORE_MONEY_COLUMNS + classes) >= widest;
        let shown = CORE_MONEY_COLUMNS + if has_classes { classes } else { 0 };
        Self {
            has_classes,
            is_compact: per_column(shown) < widest,
        }
    }
}

/// The cells the widest money column takes: the largest figure written in
/// full, or a header longer than it.
fn widest_cell(years: &[YearRow], classes: &[TreatmentClass]) -> usize {
    let largest = years.iter().map(|row| row.net_worth.abs()).max();
    let figure = present::money(largest.unwrap_or(0)).len();
    let headers = header_cells(classes);
    let header = headers.iter().skip(2).map(Line::width).max();
    figure.max(header.unwrap_or(0))
}

fn ledger_constraints(classes: &[TreatmentClass]) -> Vec<Constraint> {
    let money_columns = CORE_MONEY_COLUMNS + classes.len();
    let mut constraints = vec![Constraint::Length(YEAR_COLS), Constraint::Length(AGE_COLS)];
    constraints.extend(std::iter::repeat_n(Constraint::Fill(1), money_columns));
    constraints
}

fn header_cells(classes: &[TreatmentClass]) -> Vec<Line<'static>> {
    let mut cells = vec![Line::from("Year"), Line::from("Age")];
    for label in ["Income", "Spending", "Tax", "Withdrawn"] {
        cells.push(Line::from(label).right_aligned());
    }
    for &class in classes {
        cells.push(Line::from(present::treatment_class(class)).right_aligned());
    }
    cells.push(Line::from("Net worth").right_aligned());
    cells
}

#[derive(Clone, Copy)]
struct RowStyle<'a> {
    plan: &'a Plan,
    is_nominal: bool,
    is_compact: bool,
    columns: &'a [Column],
}

fn class_columns(classes: &[TreatmentClass]) -> Vec<Column> {
    classes.iter().copied().map(Column::Class).collect()
}

fn ledger_cells(row: &YearRow, style: RowStyle) -> Vec<Line<'static>> {
    let money = |amount: Dollars| {
        let amount = basis_amount(amount, row.deflator, style.is_nominal);
        let text = if style.is_compact {
            present::compact_dollars(amount)
        } else {
            present::money(amount)
        };
        Line::from(text).right_aligned()
    };
    let mut cells = vec![
        Line::from(row.year.to_string()),
        Line::from(ages_text(style.plan, row)),
    ];
    cells.extend(year_figures(row, style.columns).into_iter().map(money));
    cells
}

#[cfg(test)]
mod tests {
    use super::super::support::test_projected;
    use super::*;

    fn style<'a>(plan: &'a Plan, columns: &'a [Column]) -> RowStyle<'a> {
        RowStyle {
            plan,
            is_nominal: true,
            is_compact: false,
            columns,
        }
    }

    #[test]
    fn cells_follow_the_plan_shape() {
        let projected = test_projected();
        let plan = &projected.plan;
        let classes = present_classes(plan);
        // The test plan has taxable (cash) and deferred (401k) accounts.
        assert_eq!(classes, [TreatmentClass::Taxable, TreatmentClass::Deferred]);
        let header = header_cells(&classes);
        let columns = class_columns(&classes);
        let first = ledger_cells(&projected.projection.years[0], style(plan, &columns));
        assert_eq!(header.len(), first.len());
        assert_eq!(header.len(), ledger_constraints(&classes).len());
        assert_eq!(first[0].to_string(), "2026");
        assert_eq!(first[1].to_string(), "46");
        assert!(first[2].to_string().starts_with('$'), "{}", first[2]);
    }

    #[test]
    fn cells_deflate_unless_nominal() {
        let projected = test_projected();
        let plan = &projected.plan;
        let classes = present_classes(plan);
        let columns = class_columns(&classes);
        let later = &projected.projection.years[5];
        let nominal = ledger_cells(later, style(plan, &columns));
        let todays = ledger_cells(
            later,
            RowStyle {
                is_nominal: false,
                ..style(plan, &columns)
            },
        );
        let parse = |line: &Line<'_>| present::parse_money(&line.to_string()).unwrap();
        assert!(parse(&nominal[2]) > parse(&todays[2]), "income deflates");
        assert_eq!(nominal[0], todays[0], "years never scale");
    }

    #[test]
    fn a_narrow_ledger_drops_the_classes_before_it_shortens_a_figure() {
        let seven_digits = "$1,074,850".len();
        let narrow = Fit::of(80, 4, seven_digits);
        assert!(!narrow.has_classes && !narrow.is_compact, "{narrow:?}");
        let wide = Fit::of(200, 4, seven_digits);
        assert!(wide.has_classes && !wide.is_compact, "{wide:?}");
        let classes = [TreatmentClass::Taxable, TreatmentClass::Deferred];
        let small_figures = widest_cell(&[], &classes);
        assert_eq!(
            small_figures,
            "Withdrawn".len(),
            "a header sets the width too"
        );
        assert!(!Fit::of(80, 2, small_figures).has_classes);
        let vast = Fit::of(80, 4, "$1,234,567,890".len());
        assert!(!vast.has_classes && vast.is_compact, "{vast:?}");
    }
}
