//! The Ledger: one year in the context of all of them. The table of every
//! year stands over the cursor year in full - what happens in it and what
//! to do, each account's flows, where its money came from and went, and
//! its tax - and one key gives the table the whole page.

mod arrange;
mod flows;
mod funds;
mod year;
mod years;

#[cfg(test)]
mod tests;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Entity, IntoScheduleConfigs, Query, Res, ResMut, Resource, SystemSet, With,
};
use bevy_ecs::system::SystemParam;
use bevy_ui::{FlexDirection, Node};
use plurimus::widgets::WidgetSystems;
use retiretui_client::ledger::{Asked, ColumnSet, Table, Year};

pub use arrange::LedgerView;
#[cfg(test)]
pub use years::LedgerTable;

use super::command::Outcome;
use super::hints::{CommandHint, CommandHints};
use super::layout::{self, Body, growing};
use super::nav::{self, Page, ShownSurface};
use super::session::{LedgerRun, Projected, Session, Shown, Today, YearCursor, cursor_year};
use super::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<LedgerRun>();
    app.init_resource::<LedgerView>();
    app.init_resource::<Columns>();
    app.init_resource::<TableSaid>();
    app.init_resource::<YearSaid>();
    app.configure_sets(
        Update,
        (LedgerSystems::Say, LedgerSystems::Draw)
            .chain()
            .before(Repainted)
            .before(WidgetSystems::Layout),
    );
    app.add_plugins((
        arrange::plugin,
        years::plugin,
        year::plugin,
        flows::plugin,
        funds::plugin,
    ));
    app.add_systems(Startup, spawn_ledger.after(layout::spawn_frame));
    app.add_systems(
        Update,
        (leave_run_on_replan, open_run_on_its_year, say)
            .chain()
            .in_set(LedgerSystems::Say),
    );
}

/// The Ledger's frame: what it says is settled, and then drawn.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum LedgerSystems {
    Say,
    Draw,
}

/// The column set the year table is under.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Columns(pub ColumnSet);

/// Every year of what the Ledger shows, as the client tables it.
#[derive(Resource, Default)]
struct TableSaid(Option<Table>);

/// The cursor year of what the Ledger shows, as the client says it.
#[derive(Resource, Default)]
struct YearSaid(Option<Year>);

/// What marks a milestone of the plan, on its year and before its line.
const MILESTONE: &str = "◆";
/// What marks something to watch, on its year and before its line.
const WARNING: &str = "!";

/// The year keys' hints, said from any pane.
const PAGE_HINTS: &[CommandHint] = &[
    CommandHint {
        commands: &[
            super::command::LEDGER_YEARS[0],
            super::command::LEDGER_YEARS[1],
        ],
        key: 1,
        word: "year",
    },
    CommandHint {
        commands: &[
            super::command::LEDGER_MARKED[0],
            super::command::LEDGER_MARKED[1],
        ],
        key: 0,
        word: "marked year",
    },
];

fn spawn_ledger(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let view = nav::spawn_surface(&mut commands, body, Some(Page::Ledger));
    commands.entity(view).insert(CommandHints(PAGE_HINTS));
    let page = Node {
        flex_direction: FlexDirection::Column,
        ..growing()
    };
    let page = commands.spawn((page, ChildOf(view))).id();
    years::spawn_pane(&mut commands, page);
    let (year, actions) = arrange::spawn_year(&mut commands, page);
    year::spawn_panes(&mut commands, actions);
    flows::spawn_pane(&mut commands, actions);
    funds::spawn_panes(&mut commands, year);
}

/// The `ledger-plan` command: the plan's own projection back in the
/// Ledger, in place of a run opened from a market tool.
pub fn return_to_plan(mut run: ResMut<LedgerRun>) -> Outcome {
    if run.0.is_some() {
        run.0 = None;
    }
    Outcome::Done
}

/// A run describes the plan it was drawn from, so a plan that changes
/// leaves it behind.
fn leave_run_on_replan(projected: Res<Projected>, mut run: ResMut<LedgerRun>) {
    if projected.is_changed() && run.0.is_some() && !run.is_changed() {
        run.0 = None;
    }
}

/// A run is opened to be read a year at a time.
fn open_run_on_its_year(run: Res<LedgerRun>, mut view: ResMut<LedgerView>) {
    if run.is_changed() && run.0.is_some() {
        view.set_if_neq(LedgerView::Year);
    }
}

/// Says the table again whenever the plan shown, the basis or the column
/// set moves, and the year whenever any of those or the cursor does.
fn say(
    (shown, session, columns): (Shown, Res<Session>, Res<Columns>),
    mut table: ResMut<TableSaid>,
    mut year: ResMut<YearSaid>,
) {
    let ledger = shown.ledger();
    let is_nominal = shown.basis.nominal;
    let is_replanned =
        shown.projected.is_changed() || shown.run.is_changed() || shown.basis.is_changed();
    if is_replanned || columns.is_changed() || table.0.is_none() {
        let said = Table::new(ledger, &session.tables, columns.0, is_nominal);
        table.0 = Some(said);
    }
    if shown.is_changed() || year.is_added() {
        let run = shown.run.0.as_ref();
        let asked = Asked {
            year: shown.year(),
            is_nominal,
            run: run.map(|(label, _)| label.as_str()),
        };
        year.0 = Year::new(ledger, &session.tables, asked);
    }
}

/// A run condition: whether the year in full is on show. A pane it skips
/// keeps its last run, so what moved while the pane was hidden reads as
/// moved on its return - and is drawn then, since a table given its cursor
/// while hidden scrolls that row into no area.
fn shows_the_year(shown: ShownSurface, view: Res<LedgerView>) -> bool {
    shown.surface() == Some(Page::Ledger) && *view == LedgerView::Year
}

/// The year's detail, for a pane that draws it.
#[derive(SystemParam)]
struct Detail<'w> {
    year: Res<'w, YearSaid>,
    theme: Res<'w, Theme>,
}

impl Detail<'_> {
    /// The year to draw and the theme to draw it in, where either has
    /// moved since the pane last drew it, or `is_resized` says the pane
    /// has.
    fn due(&self, is_resized: bool) -> Option<(&Year, &Theme)> {
        let is_due = self.year.is_changed() || self.theme.is_changed() || is_resized;
        let year = self.year.0.as_ref().filter(|_| is_due)?;
        Some((year, &self.theme))
    }
}

/// The `ledger-table` command: the page given to the year table alone, or
/// the year back under it.
pub fn swap_table(mut view: ResMut<LedgerView>) -> Outcome {
    *view = match *view {
        LedgerView::Year => LedgerView::Table,
        LedgerView::Table => LedgerView::Year,
    };
    Outcome::Done
}

/// The `ledger-year` command: the cursor's year in full, under the table.
pub fn show_year(mut view: ResMut<LedgerView>) -> Outcome {
    view.set_if_neq(LedgerView::Year);
    Outcome::Done
}

/// The `ledger-columns` command: the table under its next column set.
pub fn turn_columns(mut columns: ResMut<Columns>) -> Outcome {
    columns.0 = columns.0.neighbor(1);
    Outcome::Done
}

/// What the year cursor is read from and moved through.
#[derive(SystemParam)]
pub struct Stepped<'w> {
    table: Res<'w, TableSaid>,
    projected: Res<'w, Projected>,
    today: Res<'w, Today>,
    cursor: ResMut<'w, YearCursor>,
}

impl Stepped<'_> {
    /// Moves the cursor to the nearest marked year `step` says to look
    /// towards, refusing where there is none.
    fn step_to_marked(&mut self, step: i16) -> Outcome {
        let from = cursor_year(&self.projected, *self.today, *self.cursor);
        let table = self.table.0.as_ref();
        match table.and_then(|table| table.marked_year(from, step)) {
            Some(year) => {
                self.cursor.set_if_neq(YearCursor(Some(year)));
                Outcome::Done
            }
            None if step > 0 => Outcome::Refused(format!("No marked year after {from}")),
            None => Outcome::Refused(format!("No marked year before {from}")),
        }
    }
}

/// The `ledger-marked-next` command.
pub fn next_marked(mut stepped: Stepped) -> Outcome {
    stepped.step_to_marked(1)
}

/// The `ledger-marked-previous` command.
pub fn previous_marked(mut stepped: Stepped) -> Outcome {
    stepped.step_to_marked(-1)
}
