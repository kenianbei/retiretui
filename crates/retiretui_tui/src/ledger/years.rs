//! The year table: every year under the column set it is turned to - the
//! year, the ages reached, what marks it, its figures and the net worth it
//! ends on - over the cursor year or as the whole page, its headers held
//! above the years as they scroll. The table's cursor is the year cursor
//! either way.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, Query, Res, With,
};
use bevy_ecs::system::SystemParam;
use bevy_ui::{Node, UiRect, Val};
use plurimus::core::TerminalSize;
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::UiStyle;
use plurimus::widgets::{
    ActiveDescendant, TableColumns, table_header, table_row, table_self_update,
};
use retiretui_client::ledger::{Marks, Table, TableRow};
use retiretui_engine::plan::Dollars;

use super::arrange::Part;
use super::{Columns, LedgerSystems, MILESTONE, TableSaid, WARNING};
use crate::edit::table_bundle;
use crate::hints::CommandHints;
use crate::layout::{self, growing, placed};
use crate::nav::{self, FocusStop, Page};
use crate::pane::{self, Framed, Pane};
use crate::present::{self, MoneyForm};
use crate::session::{RowYear, Shown, track_cursor};
use crate::theme::Theme;

pub(super) fn plugin(app: &mut App) {
    let on_ledger = nav::shows(Page::Ledger);
    app.add_systems(
        Update,
        (
            title_years,
            rebuild_rows.run_if(on_ledger.clone()),
            follow_cursor.run_if(on_ledger),
            track_cursor::<LedgerTable>,
        )
            .chain()
            .in_set(LedgerSystems::Draw),
    );
    app.add_observer(table_self_update);
}

/// The table of years.
#[derive(Component)]
pub struct LedgerTable;

/// The table of one row the headers stand in, over the years' own so that
/// it does not scroll with them.
#[derive(Component)]
struct LedgerHeads;

#[derive(Component)]
struct LedgerHeaderRow;

/// The pane the years are tabled in.
#[derive(Component)]
pub(super) struct YearsPane;

const TITLE: &str = "Ledger";

pub(super) fn spawn_pane(commands: &mut Commands, ledger: Entity) {
    let pane = Pane::new(TITLE).spawn(commands, ledger);
    let parts = (YearsPane, Part::Table, CommandHints(&[]));
    commands.entity(pane).insert(parts);
    // The years' columns start past their cursor and end before their
    // scrollbar, and the headers' are solved over the same cells.
    let heads = Node {
        margin: UiRect {
            left: Val::Px(f32::from(layout::CURSOR_COLS)),
            right: Val::Px(f32::from(SCROLL_BAR_COLS)),
            ..UiRect::default()
        },
        height: Val::Px(HEADER_ROWS),
        flex_shrink: 0.0,
        ..Node::default()
    };
    let headed = (plurimus::widgets::table([]), LedgerHeads, heads, placed());
    commands.spawn((headed, ChildOf(pane)));
    commands.spawn((
        table_bundle(),
        LedgerTable,
        FocusStop,
        growing(),
        placed(),
        ChildOf(pane),
    ));
}

/// Titles the pane by the run it shows, the dollars and the column set.
fn title_years(
    (shown, columns): (Shown, Res<Columns>),
    mut panes: Query<&mut Framed, With<YearsPane>>,
) {
    if !shown.run.is_changed() && !shown.basis.is_changed() && !columns.is_changed() {
        return;
    }
    let mut parts = vec![TITLE.to_owned()];
    parts.extend(shown.run.0.iter().map(|(label, _)| label.clone()));
    parts.push(present::basis_name(shown.basis.nominal).to_owned());
    parts.push(columns.0.title().to_owned());
    let title = parts.join(" · ");
    for mut pane in &mut panes {
        Framed::retitle(&mut pane, &title);
    }
}

#[derive(SystemParam)]
struct LedgerEntities<'w, 's> {
    tables: Query<'w, 's, Entity, With<LedgerTable>>,
    heads: Query<'w, 's, Entity, With<LedgerHeads>>,
    header_rows: Query<'w, 's, Entity, With<LedgerHeaderRow>>,
    rows: Query<'w, 's, (Entity, &'static ChildOf), With<RowYear>>,
}

/// What the rows are built from, and what makes them stale.
#[derive(SystemParam)]
struct RowInputs<'w, 's> {
    said: Res<'w, TableSaid>,
    shown: Shown<'w>,
    theme: Res<'w, Theme>,
    size: Res<'w, TerminalSize>,
    /// How the rows on screen were fitted, so a resize that fits the same
    /// way rebuilds nothing.
    fitted: Local<'s, Option<Fit>>,
}

impl RowInputs<'_, '_> {
    /// Whether what the rows say or how they fit has moved since they were
    /// built. It is asked only while the Ledger is on show, and what moved
    /// while it was not reads as moved on its return: setting the cursor
    /// reveals it, and a hidden table is placed nowhere, so a reveal
    /// resolved against its empty area scrolls the rows out of view for
    /// good.
    fn is_stale(&self) -> bool {
        let is_refitted = self.size.is_changed() && *self.fitted != self.fit();
        self.said.is_changed() || self.theme.is_changed() || is_refitted
    }

    fn fit(&self) -> Option<Fit> {
        let table = self.said.0.as_ref()?;
        Some(Fit::of(self.size.cols, table))
    }
}

/// Respawns the header and year rows whenever what they say or the fit
/// changes, carrying the cursor across by year.
fn rebuild_rows(mut inputs: RowInputs, entities: LedgerEntities, mut commands: Commands) {
    if !inputs.is_stale() {
        return;
    }
    let (Ok(ledger_table), Ok(heads)) = (entities.tables.single(), entities.heads.single()) else {
        return;
    };
    let (Some(table), Some(fit)) = (inputs.said.0.as_ref(), inputs.fit()) else {
        return;
    };
    for row in &entities.header_rows {
        commands.entity(row).despawn();
    }
    for (row, parent) in &entities.rows {
        if parent.parent() == ledger_table {
            commands.entity(row).despawn();
        }
    }
    *inputs.fitted = Some(fit);
    let figures = fit.figures(table.figure_headers.len());
    for tabled in [heads, ledger_table] {
        let columns = TableColumns(constraints(figures.len()));
        commands.entity(tabled).insert(columns);
    }
    commands.spawn((
        table_header(header_cells(table, &figures, fit.given)),
        LedgerHeaderRow,
        UiStyle(Style::new().add_modifier(Modifier::BOLD)),
        ChildOf(heads),
    ));
    let cursor_year = inputs.shown.year();
    let mut cursor_row = None;
    for row in &table.rows {
        let cells = year_cells(row, &figures, fit.form, &inputs.theme);
        let mut spawned = commands.spawn((table_row(cells), RowYear(row.year)));
        spawned.insert(ChildOf(ledger_table));
        if row.is_exceeded {
            spawned.insert(UiStyle(inputs.theme.exceeded()));
        }
        if cursor_year == row.year {
            cursor_row = Some(spawned.id());
        }
    }
    commands
        .entity(ledger_table)
        .insert(ActiveDescendant(cursor_row));
}

/// Points the table at a year set elsewhere, which, like the rows, waits
/// for the Ledger to be shown.
fn follow_cursor(
    shown: Shown,
    mut tables: Query<(Entity, &mut ActiveDescendant), With<LedgerTable>>,
    rows: Query<(Entity, &RowYear, &ChildOf)>,
) {
    if !shown.is_changed() {
        return;
    }
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

/// The row the headers stand in.
const HEADER_ROWS: f32 = 1.0;
const YEAR_COLS: u16 = 5;
const AGE_COLS: u16 = 6;
const MARK_COLS: u16 = 2;
const SCROLL_BAR_COLS: u16 = 1;
/// The year, the ages and the marks, which lead every row.
const LEADING: [u16; 3] = [YEAR_COLS, AGE_COLS, MARK_COLS];
/// The cells the leading columns take, each with the one after it.
const LEADING_COLS: u16 = YEAR_COLS + AGE_COLS + MARK_COLS + LEADING.len() as u16;
/// Columns of the pane the figures never get.
const LEDGER_CHROME: u16 = pane::BORDERS + layout::CURSOR_COLS + SCROLL_BAR_COLS + LEADING_COLS;
/// The cells a figure written short takes: `-$1.23M`.
const COMPACT_COLS: usize = 7;

/// How the years fit the width they have: how many figures are drawn
/// ahead of the net worth, the cells each gets, and whether they are
/// written in full.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Fit {
    /// The figures drawn ahead of the net worth, from the first.
    before_net_worth: usize,
    form: MoneyForm,
    /// The cells each figure's column gets.
    given: usize,
}

impl Fit {
    /// The table across `cols`. Figures are shortened before the set's own
    /// columns go, since the set is what the table was turned to, and they
    /// go before the figures every set leads with are shortened.
    fn of(cols: u16, table: &Table) -> Self {
        let figures = table.figure_headers.len();
        let widest = widest_figure(table);
        let room = usize::from(cols.saturating_sub(LEDGER_CHROME));
        let per_column = |columns: usize| (room / columns.max(1)).saturating_sub(1);
        let has_set = per_column(figures) >= widest.min(COMPACT_COLS);
        let drawn = if has_set {
            figures
        } else {
            figures.min(Table::LEADING + 1)
        };
        let given = per_column(drawn);
        Self {
            before_net_worth: drawn.saturating_sub(1),
            form: form_for(given, widest),
            given,
        }
    }

    /// Which of a row's `count` figures are drawn, by their place.
    fn figures(self, count: usize) -> Vec<usize> {
        let last = count.saturating_sub(1);
        let before = 0..self.before_net_worth.min(last);
        before.chain([last]).collect()
    }
}

fn form_for(given: usize, widest: usize) -> MoneyForm {
    if given < widest {
        MoneyForm::Compact
    } else {
        MoneyForm::Full
    }
}

/// The cells the widest figure takes written in full, or the header of
/// one every set shows that is longer.
fn widest_figure(table: &Table) -> usize {
    let largest = table.rows.iter().flat_map(|row| &row.figures);
    let largest = largest.map(|figure| figure.abs()).max();
    let leading = table.figure_headers.iter().take(Table::LEADING);
    let headers = leading.chain(table.figure_headers.last());
    let header = headers.map(|header| header.chars().count()).max();
    present::money(largest.unwrap_or(0))
        .len()
        .max(header.unwrap_or(0))
}

fn constraints(figures: usize) -> Vec<Constraint> {
    let mut constraints: Vec<Constraint> = LEADING.into_iter().map(Constraint::Length).collect();
    constraints.extend(std::iter::repeat_n(Constraint::Fill(1), figures));
    constraints
}

/// The year, the ages and nothing over the marks on the left, the figures
/// on the right; one too long for the `given` cells of its column stands
/// on the left, clipped at its end rather than its start.
fn header_cells(table: &Table, figures: &[usize], given: usize) -> Vec<Line<'static>> {
    let text = table.text_headers.into_iter().map(Line::from);
    let figures = figures.iter().map(|&at| {
        let cell = Line::from(table.figure_headers[at].clone());
        if cell.width() > given {
            cell
        } else {
            cell.right_aligned()
        }
    });
    text.chain([Line::default()]).chain(figures).collect()
}

fn marks(marks: Marks, theme: &Theme) -> Line<'static> {
    let mut spans = Vec::new();
    if marks.is_milestone {
        spans.push(Span::styled(MILESTONE, theme.accented()));
    }
    if marks.has_warning {
        spans.push(Span::styled(WARNING, Style::new().fg(theme.caution)));
    }
    Line::from(spans)
}

fn year_cells(
    row: &TableRow,
    figures: &[usize],
    form: MoneyForm,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let money = |amount: Dollars| Line::from(form.money(amount)).right_aligned();
    let mut cells = vec![
        Line::from(row.year.to_string()),
        Line::from(row.ages.clone()),
        marks(row.marks, theme),
    ];
    cells.extend(figures.iter().map(|&at| money(row.figures[at])));
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of `figures` figures a year, the largest `largest`.
    fn table_of(figures: usize, largest: Dollars) -> Table {
        let row = TableRow {
            year: 2026,
            ages: "51".to_owned(),
            marks: Marks::default(),
            figures: vec![largest; figures],
            is_exceeded: false,
        };
        Table {
            text_headers: ["Year", "Age"],
            figure_headers: vec!["Figure".to_owned(); figures],
            rows: vec![row],
        }
    }

    const SEVEN_FIGURES: Dollars = 1_074_850;

    #[test]
    fn the_table_shortens_its_figures_before_it_drops_the_set_s_columns() {
        let roomy = Fit::of(200, &table_of(9, SEVEN_FIGURES));
        assert_eq!((roomy.before_net_worth, roomy.form), (8, MoneyForm::Full));
        let by_account = Fit::of(128, &table_of(12, SEVEN_FIGURES));
        assert_eq!(by_account.before_net_worth, 11, "{by_account:?}");
        assert_eq!(by_account.form, MoneyForm::Compact, "{by_account:?}");
        let cramped = Fit::of(128, &table_of(25, SEVEN_FIGURES));
        assert_eq!(cramped.before_net_worth, Table::LEADING, "{cramped:?}");
        assert_eq!(cramped.form, MoneyForm::Full, "what leads has the room");
        let vast = Fit::of(128, &table_of(25, 1_234_567_890_123_456));
        assert_eq!(vast.form, MoneyForm::Compact, "{vast:?}");
    }

    #[test]
    fn a_fit_draws_what_leads_the_set_s_own_and_always_the_net_worth() {
        let all = Fit::of(200, &table_of(9, SEVEN_FIGURES));
        assert_eq!(all.figures(9), [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        let cramped = Fit::of(128, &table_of(25, SEVEN_FIGURES));
        assert_eq!(cramped.figures(25), [0, 1, 2, 3, 24]);
        assert_eq!(constraints(3).len(), LEADING.len() + 3);
    }

    #[test]
    fn the_widest_figure_is_a_header_where_every_figure_is_shorter() {
        let mut table = table_of(6, 5);
        table.figure_headers[3] = "Withdrawn".to_owned();
        table.figure_headers[4] = "A set's own long header".to_owned();
        assert_eq!(
            widest_figure(&table),
            "Withdrawn".len(),
            "a set's own may clip"
        );
    }
}
