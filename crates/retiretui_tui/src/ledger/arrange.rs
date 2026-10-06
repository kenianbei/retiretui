//! How the Ledger's page is split: the years beside the cursor year in
//! full, or the years alone as the whole table; and whether it is tall
//! enough for the histories.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Query, Res, ResMut, Resource, With, Without,
};
use bevy_ecs::system::SystemParam;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::TerminalSize;

use super::years::{LedgerTable, YearsPane};
use super::{LedgerSystems, say};
use crate::command::{LEDGER_COLUMNS, LEDGER_TABLE};
use crate::focus::PageFocus;
use crate::hints::{CommandHint, CommandHints};
use crate::layout::set_display;
use crate::nav::FocusStop;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Roomy>();
    app.add_systems(
        Update,
        (measure, arrange.after(say)).in_set(LedgerSystems::Say),
    );
}

/// Whether the page is tall enough for the histories under the money
/// panes.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) struct Roomy(pub(super) bool);

/// The terminal at least this tall has the room: the year, a dozen
/// accounts' flows and the money panes fit above the histories' least.
const ROOMY_ROWS: u16 = 44;

fn measure(size: Res<TerminalSize>, mut roomy: ResMut<Roomy>) {
    roomy.set_if_neq(Roomy(size.rows >= ROOMY_ROWS));
}

/// What the Ledger's page is given over to.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum LedgerView {
    /// The cursor year in full, beside the list of years.
    #[default]
    Year,
    /// Every year in one table.
    Table,
}

/// The columns the list of years takes beside the year, borders included:
/// the year, a couple's ages, the marks and an eight-figure net worth.
pub(super) const YEARS_COLS: f32 = 32.0;

/// The column the cursor year is laid out down.
#[derive(Component)]
struct DetailColumn;

/// A pane of the year's detail the keyboard walks while it is on show.
#[derive(Component)]
pub(super) struct DetailStop;

const TO_TABLE: &[CommandHint] = &[CommandHint {
    commands: &[LEDGER_TABLE],
    key: 0,
    word: "table",
}];
const FROM_TABLE: &[CommandHint] = &[
    CommandHint {
        commands: &[LEDGER_COLUMNS],
        key: 0,
        word: "columns",
    },
    CommandHint {
        commands: &[LEDGER_TABLE],
        key: 0,
        word: "the year",
    },
];

/// The column the year's panes are stacked down, beside the years.
pub(super) fn spawn_detail(commands: &mut Commands, across: Entity) -> Entity {
    let detail = Node {
        flex_direction: FlexDirection::Column,
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_width: Val::Px(0.0),
        ..Node::default()
    };
    let hints = CommandHints(TO_TABLE);
    commands
        .spawn((detail, DetailColumn, hints, ChildOf(across)))
        .id()
}

/// What the view shows, hides and hands the keyboard between.
#[derive(SystemParam)]
struct Laid<'w, 's> {
    years: Query<'w, 's, (&'static mut Node, &'static mut CommandHints), With<YearsPane>>,
    details: Query<'w, 's, &'static mut Node, (With<DetailColumn>, Without<YearsPane>)>,
    tables: Query<'w, 's, Entity, With<LedgerTable>>,
    detail_stops: Query<'w, 's, Entity, With<DetailStop>>,
}

/// Lays the page out for the view as it changes: under the table the year's
/// panes take no room and are not walked to, and the keyboard one of them
/// held goes to the table.
fn arrange(view: Res<LedgerView>, mut laid: Laid, mut focus: PageFocus, mut commands: Commands) {
    if !view.is_changed() {
        return;
    }
    let is_table = *view == LedgerView::Table;
    for (mut node, mut hints) in &mut laid.years {
        node.width = if is_table {
            Val::Percent(100.0)
        } else {
            Val::Px(YEARS_COLS)
        };
        hints.0 = if is_table { FROM_TABLE } else { TO_TABLE };
    }
    for mut node in &mut laid.details {
        set_display(&mut node, !is_table);
    }
    let holder = focus.holder();
    let mut is_left = false;
    for stop in &laid.detail_stops {
        if is_table {
            commands.entity(stop).remove::<FocusStop>();
            is_left |= holder == Some(stop);
        } else {
            commands.entity(stop).insert(FocusStop);
        }
    }
    if is_left {
        focus.set(laid.tables.iter().next());
    }
}
