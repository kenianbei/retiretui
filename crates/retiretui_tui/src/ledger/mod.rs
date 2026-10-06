//! The Ledger: one year in the context of all of them. A list of every
//! year stands beside the cursor year in full - what happens in it and
//! what to do, each account's flows, where its money came from and went,
//! and its tax - and one key swaps the page for the whole year table.

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
    Commands, Entity, IntoScheduleConfigs, Local, Query, Res, ResMut, Resource, SystemSet, With,
};
use bevy_ecs::system::SystemParam;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::widgets::WidgetSystems;
use retiretui_client::ledger::{Asked, ColumnSet, Table, Year};

pub use arrange::LedgerView;
#[cfg(test)]
pub use years::LedgerTable;

use super::command::Outcome;
use super::hints::{CommandHint, CommandHints};
use super::layout::{self, Body, growing};
use super::nav::{self, ActivePage, Page};
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

/// The key that returns a market run to the plan, said where a run is
/// named.
pub(super) const RETURN: &str = "returns to the plan";

/// The year keys' and the table key's hints, said from any pane.
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

/// The rows the three panes under the flows keep, borders included,
/// however many accounts the year touches.
const FUNDS_LEAST: f32 = 7.0;

fn spawn_ledger(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let view = nav::spawn_surface(&mut commands, body, Some(Page::Ledger));
    commands.entity(view).insert(CommandHints(PAGE_HINTS));
    let across = Node {
        flex_direction: FlexDirection::Row,
        ..growing()
    };
    let across = commands.spawn((across, ChildOf(view))).id();
    years::spawn_pane(&mut commands, across);
    let detail = arrange::spawn_detail(&mut commands, across);
    year::spawn_pane(&mut commands, detail);
    flows::spawn_pane(&mut commands, detail);
    let funds = Node {
        flex_direction: FlexDirection::Row,
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_height: Val::Px(FUNDS_LEAST),
        ..Node::default()
    };
    let funds = commands.spawn((funds, ChildOf(detail))).id();
    funds::spawn_panes(&mut commands, funds);
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
        let asked = Asked {
            year: shown.year(),
            is_nominal,
            is_run: shown.run.0.is_some(),
        };
        year.0 = Year::new(ledger, &session.tables, asked);
    }
}

/// The year's detail, for a pane that draws it: drawn while it is on show,
/// and one that moves while it is not waits for it, since a table given
/// its cursor while hidden scrolls that row into no area.
#[derive(SystemParam)]
struct Detail<'w, 's> {
    year: Res<'w, YearSaid>,
    view: Res<'w, LedgerView>,
    active: Res<'w, ActivePage>,
    theme: Res<'w, Theme>,
    is_stale: Local<'s, bool>,
}

impl Detail<'_, '_> {
    /// Whether the detail is on the page on show.
    fn is_on_show(&self) -> bool {
        self.active.page() == Page::Ledger && *self.view == LedgerView::Year
    }

    /// The year to draw, where it or the theme has moved since the pane
    /// last drew it, or `is_resized` says the pane has.
    fn due(&mut self, is_resized: bool) -> Option<&Year> {
        *self.is_stale |= self.year.is_changed() || self.theme.is_changed() || is_resized;
        let drawing = *self.is_stale && self.is_on_show();
        *self.is_stale &= !drawing;
        self.year.0.as_ref().filter(|_| drawing)
    }
}

/// The `ledger-table` command: the whole year table in the page's place,
/// or the year it left.
pub fn swap_table(mut view: ResMut<LedgerView>) -> Outcome {
    *view = match *view {
        LedgerView::Year => LedgerView::Table,
        LedgerView::Table => LedgerView::Year,
    };
    Outcome::Done
}

/// The `ledger-year` command: the cursor's year in full, from the table.
pub fn show_year(mut view: ResMut<LedgerView>) -> Outcome {
    view.set_if_neq(LedgerView::Year);
    Outcome::Done
}

/// The `ledger-columns` command: the table under its next column set.
pub fn turn_columns(mut columns: ResMut<Columns>, mut view: ResMut<LedgerView>) -> Outcome {
    columns.0 = columns.0.neighbor(1);
    view.set_if_neq(LedgerView::Table);
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
