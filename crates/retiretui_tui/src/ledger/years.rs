//! The years: a narrow list beside the cursor year - the year, the ages
//! reached, what marks it and the net worth it ends on - or, as the whole
//! page, the year table under the column set it is turned to. The table's
//! cursor is the year cursor either way.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, Or, Query, Res, With,
};
use bevy_ecs::system::SystemParam;
use plurimus::core::TerminalSize;
use plurimus::core::ratatui_core::layout::{Constraint, Size};
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::{ScrollArea, UiStyle};
use plurimus::widgets::{
    ActiveDescendant, TableColumns, TableSelection, TableStripe, table, table_header, table_row,
    table_self_update,
};
use retiretui_client::ledger::{Marks, Table, TableRow, YEARS};
use retiretui_engine::plan::Dollars;

use super::arrange::{LedgerView, YEARS_COLS};
use super::{Columns, LedgerSystems, RETURN, TableSaid};
use crate::command::{self, Keymap};
use crate::edit::table_keys;
use crate::hints::CommandHints;
use crate::layout::{self, filling, placed};
use crate::nav::{self, ActivePage, FocusStop, Page};
use crate::pane::{self, Framed, Pane};
use crate::present::{self, MoneyForm};
use crate::session::{LedgerRun, RowYear, Shown, track_cursor};
use crate::theme::Theme;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            title_years,
            rebuild_rows,
            follow_cursor.run_if(nav::shows(Page::Ledger)),
            track_cursor::<LedgerTable>,
        )
            .chain()
            .in_set(LedgerSystems::Draw),
    );
    app.add_observer(table_self_update);
}

/// The table of years, as the list or as the whole page.
#[derive(Component)]
pub struct LedgerTable;

#[derive(Component)]
struct LedgerHeaderRow;

/// The pane the years are listed in.
#[derive(Component)]
pub(super) struct YearsPane;

const TITLE: &str = "Ledger";
const MILESTONE: &str = "◆";
const WARNING: &str = "!";

pub(super) fn spawn_pane(commands: &mut Commands, across: Entity) {
    let pane = Pane::new(YEARS).wide(YEARS_COLS).spawn(commands, across);
    commands.entity(pane).insert((YearsPane, CommandHints(&[])));
    commands.spawn((
        table([Constraint::Length(YEAR_COLS)]),
        LedgerTable,
        TableSelection::Row,
        layout::table_cursor(),
        table_keys(),
        TableStripe(Style::new()),
        ScrollArea::new(Size::default()),
        FocusStop,
        filling(),
        placed(),
        ChildOf(pane),
    ));
}

/// Titles the pane for the view: the list by what it lists, the table by
/// the run it shows, the dollars and the column set.
fn title_years(
    (run, shown, columns): (Res<LedgerRun>, Shown, Res<Columns>),
    view: Res<LedgerView>,
    keymap: Res<Keymap>,
    mut panes: Query<&mut Framed, With<YearsPane>>,
) {
    let is_moved = run.is_changed() || shown.basis.is_changed() || columns.is_changed();
    if !is_moved && !view.is_changed() {
        return;
    }
    let title = if *view == LedgerView::Year {
        YEARS.to_owned()
    } else {
        let mut parts = vec![TITLE.to_owned()];
        parts.extend(run_named(&run, &keymap));
        parts.push(present::basis_name(shown.basis.nominal).to_owned());
        parts.push(columns.0.title().to_owned());
        parts.join(" · ")
    };
    for mut pane in &mut panes {
        Framed::retitle(&mut pane, &title);
    }
}

/// The run the Ledger shows and the key that leaves it, as a title says
/// them; nothing of the plan's own projection.
pub(super) fn run_named(run: &LedgerRun, keymap: &Keymap) -> Option<String> {
    let (label, _) = run.0.as_ref()?;
    Some(match keymap.label_named(command::LEDGER_PLAN, 0) {
        "" => label.clone(),
        back => format!("{label} · {back} {RETURN}"),
    })
}

#[derive(SystemParam)]
struct LedgerEntities<'w, 's> {
    tables: Query<'w, 's, Entity, With<LedgerTable>>,
    rows: Query<'w, 's, (Entity, &'static ChildOf), Or<(With<RowYear>, With<LedgerHeaderRow>)>>,
}

/// What the rows are built from, and what makes them stale.
#[derive(SystemParam)]
struct RowInputs<'w, 's> {
    said: Res<'w, TableSaid>,
    shown: Shown<'w>,
    view: Res<'w, LedgerView>,
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
        let is_refitted = self.size.is_changed() && *self.fitted != self.fit();
        *self.is_stale |= self.said.is_changed()
            || self.view.is_changed()
            || self.theme.is_changed()
            || is_refitted;
        let rebuilding = *self.is_stale && self.active.page() == Page::Ledger;
        *self.is_stale &= !rebuilding;
        rebuilding
    }

    fn fit(&self) -> Option<Fit> {
        let table = self.said.0.as_ref()?;
        let widest = widest_figure(&table.rows);
        Some(match *self.view {
            LedgerView::Year => Fit::beside(widest),
            LedgerView::Table => Fit::of(self.size.cols, set_columns(table), widest),
        })
    }
}

/// Respawns the header and year rows whenever what they say, the view or
/// the fit changes, carrying the cursor across by year.
fn rebuild_rows(mut inputs: RowInputs, entities: LedgerEntities, mut commands: Commands) {
    if !inputs.should_rebuild() {
        return;
    }
    let (Ok(ledger_table), Some(table)) = (entities.tables.single(), inputs.said.0.as_ref()) else {
        return;
    };
    let Some(fit) = inputs.fit() else {
        return;
    };
    for (row, parent) in &entities.rows {
        if parent.parent() == ledger_table {
            commands.entity(row).despawn();
        }
    }
    *inputs.fitted = Some(fit);
    let figures = fit.figures(table.headers.len() - TEXT_COLUMNS);
    commands
        .entity(ledger_table)
        .insert(TableColumns(constraints(figures.len())));
    commands.spawn((
        table_header(header_cells(table, &figures, fit.given)),
        LedgerHeaderRow,
        UiStyle(Style::new().add_modifier(Modifier::BOLD)),
        ChildOf(ledger_table),
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

const YEAR_COLS: u16 = 5;
const AGE_COLS: u16 = 6;
const MARK_COLS: u16 = 2;
const SCROLL_BAR_COLS: u16 = 1;
/// The year, the ages and the marks, which lead every row.
const LEADING: [u16; 3] = [YEAR_COLS, AGE_COLS, MARK_COLS];
/// The year's and the ages' headers, which the client's table leads with.
const TEXT_COLUMNS: usize = 2;
/// Columns of the terminal the rows never get.
const LEDGER_CHROME: u16 = pane::BORDERS + layout::CURSOR_COLS + SCROLL_BAR_COLS;
/// Income, spending, tax and withdrawn, which lead every column set.
const LEADING_FIGURES: usize = 4;
/// The leading figures and net worth, which every width shows.
const CORE_FIGURES: usize = LEADING_FIGURES + 1;
/// The cells a figure written short takes: `-$1.23M`.
const COMPACT_COLS: usize = 7;

/// How the years fit the width they have: whether the column set's own
/// figures get columns, whether any but net worth does, and whether
/// figures are written in full.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Fit {
    has_set: bool,
    has_figures: bool,
    form: MoneyForm,
    /// The cells each figure's column gets.
    given: usize,
}

impl Fit {
    /// The table across `cols`. Figures are shortened before the set's own
    /// columns go, since the set is what the table was turned to, and they
    /// go before the figures every set leads with are shortened.
    /// `widest` is the most cells a figure written in full takes.
    fn of(cols: u16, set_columns: usize, widest: usize) -> Self {
        let leading: u16 = LEADING.iter().map(|cols| cols + 1).sum();
        let room = usize::from(cols.saturating_sub(LEDGER_CHROME + leading));
        let per_column = |columns: usize| (room / columns).saturating_sub(1);
        let with_set = per_column(CORE_FIGURES + set_columns);
        let has_set = with_set >= widest.min(COMPACT_COLS);
        let given = if has_set {
            with_set
        } else {
            per_column(CORE_FIGURES)
        };
        Self {
            has_set,
            has_figures: true,
            form: form_for(given, widest),
            given,
        }
    }

    /// The list beside the year: net worth alone, in what the pane leaves
    /// it.
    fn beside(widest: usize) -> Self {
        let leading: u16 = LEADING.iter().map(|cols| cols + 1).sum();
        let room = (YEARS_COLS as u16).saturating_sub(LEDGER_CHROME + leading);
        Self {
            has_set: false,
            has_figures: false,
            form: form_for(usize::from(room), widest),
            given: usize::from(room),
        }
    }

    /// Which of a row's `count` figures are drawn, by their place.
    fn figures(self, count: usize) -> Vec<usize> {
        let last = count.saturating_sub(1);
        let leading = 0..LEADING_FIGURES.min(last);
        let own = LEADING_FIGURES.min(last)..last;
        let leading = leading.filter(|_| self.has_figures);
        let own = own.filter(|_| self.has_figures && self.has_set);
        leading.chain(own).chain([last]).collect()
    }
}

fn form_for(given: usize, widest: usize) -> MoneyForm {
    if given < widest {
        MoneyForm::Compact
    } else {
        MoneyForm::Full
    }
}

/// How many columns are the set's own.
fn set_columns(table: &Table) -> usize {
    (table.headers.len() - TEXT_COLUMNS).saturating_sub(CORE_FIGURES)
}

/// The cells the widest figure takes written in full, or a header every
/// set shows that is longer.
fn widest_figure(rows: &[TableRow]) -> usize {
    let largest = rows.iter().flat_map(|row| &row.figures);
    let largest = largest.map(|figure| figure.abs()).max();
    present::money(largest.unwrap_or(0))
        .len()
        .max("Withdrawn".len())
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
    let text = table.headers.iter().take(TEXT_COLUMNS);
    let text = text.map(|(header, _)| Line::from(header.clone()));
    let figures = figures.iter().map(|&at| {
        let (header, _) = &table.headers[TEXT_COLUMNS + at];
        let cell = Line::from(header.clone());
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

    const SEVEN_DIGITS: usize = "$1,074,850".len();

    #[test]
    fn the_table_shortens_its_figures_before_it_drops_the_set_s_columns() {
        let roomy = Fit::of(200, 4, SEVEN_DIGITS);
        assert!(roomy.has_set && roomy.form == MoneyForm::Full, "{roomy:?}");
        let by_account = Fit::of(128, 7, SEVEN_DIGITS);
        assert!(by_account.has_set, "{by_account:?}");
        assert_eq!(by_account.form, MoneyForm::Compact, "{by_account:?}");
        let cramped = Fit::of(128, 20, SEVEN_DIGITS);
        assert!(!cramped.has_set, "{cramped:?}");
        assert_eq!(cramped.form, MoneyForm::Full, "what leads has the room");
        let vast = Fit::of(128, 20, "$1,234,567,890,123,456".len());
        assert_eq!(vast.form, MoneyForm::Compact, "{vast:?}");
    }

    #[test]
    fn a_fit_draws_what_leads_the_set_s_own_and_always_the_net_worth() {
        let all = Fit::of(200, 4, SEVEN_DIGITS);
        assert_eq!(all.figures(9), [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        let cramped = Fit::of(128, 20, SEVEN_DIGITS);
        assert_eq!(cramped.figures(25), [0, 1, 2, 3, 24]);
        let beside = Fit::beside(SEVEN_DIGITS);
        assert_eq!(beside.figures(9), [8], "the list is net worth alone");
        assert_eq!(beside.form, MoneyForm::Full, "{beside:?}");
        assert_eq!(
            Fit::beside("$123,456,789,012".len()).form,
            MoneyForm::Compact
        );
        assert_eq!(constraints(3).len(), LEADING.len() + 3);
    }
}
