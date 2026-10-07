//! How the Ledger's page is split: the year table over the cursor year in
//! full, or the table alone; and how the year is laid out under it, by the
//! shape of the terminal.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, Query, Res, ResMut, Resource, With,
};
use bevy_ecs::system::SystemParam;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::TerminalSize;

use super::years::{LedgerTable, YearsPane};
use super::{LedgerSystems, say};
use crate::command::{LEDGER_COLUMNS, LEDGER_TABLE};
use crate::focus::PageFocus;
use crate::hints::{CommandHint, CommandHints};
use crate::layout::{self, growing, set_display};
use crate::nav::FocusStop;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Shape>();
    app.add_systems(
        Update,
        (measure, arrange.after(say))
            .chain()
            .in_set(LedgerSystems::Say),
    );
}

/// How the year is laid out under the table, by which the terminal has
/// more of.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Shape {
    /// Wider than tall: the money panes stand in a column beside the rest.
    #[default]
    Landscape,
    /// Taller than wide: the money panes are a row under the rest.
    Portrait,
}

/// A cell is about this many times as tall as it is wide.
const CELL_ASPECT: u16 = 2;
/// The part of the page the table takes over the year: one row in this
/// many.
const LANDSCAPE_TABLE: u16 = 3;
const PORTRAIT_TABLE: u16 = 2;
/// The share of a landscape page's width the year takes beside the money.
const ACTIONS_SHARE: f32 = 2.0;

impl Shape {
    fn of(size: TerminalSize) -> Self {
        if size.cols < size.rows.saturating_mul(CELL_ASPECT) {
            Self::Portrait
        } else {
            Self::Landscape
        }
    }

    pub(super) fn is_landscape(self) -> bool {
        self == Self::Landscape
    }

    /// The rows the table takes over the year on a terminal of `size`:
    /// whole rows, since a share that splits one leaves the panes under it
    /// a row out.
    fn table_rows(self, size: TerminalSize) -> f32 {
        let page = size.rows.saturating_sub(layout::CHROME_ROWS);
        let part = match self {
            Self::Landscape => LANDSCAPE_TABLE,
            Self::Portrait => PORTRAIT_TABLE,
        };
        f32::from(page / part)
    }
}

/// The terminal at least this tall opens on the year under the table:
/// past the tab and key rows, the two thirds a landscape table leaves hold
/// the three money panes at their least. A shorter one opens on the table
/// alone.
const SPLIT_ROWS: u16 = 36;

/// Settles the shape as the terminal is resized, and the view a terminal
/// of its height opens on whenever it comes to have, or to lack, the room
/// for the year.
fn measure(
    size: Res<TerminalSize>,
    mut shape: ResMut<Shape>,
    mut view: ResMut<LedgerView>,
    mut was_split: Local<Option<bool>>,
) {
    if !size.is_changed() {
        return;
    }
    shape.set_if_neq(Shape::of(*size));
    let is_split = size.rows >= SPLIT_ROWS;
    if was_split.replace(is_split) != Some(is_split) {
        view.set_if_neq(if is_split {
            LedgerView::Year
        } else {
            LedgerView::Table
        });
    }
}

/// What the Ledger's page is given over to.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum LedgerView {
    /// Every year in a table, over the cursor year in full.
    #[default]
    Year,
    /// The table alone.
    Table,
}

/// A node of the page the view and the shape lay out.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Part {
    /// The pane of the year table.
    Table,
    /// The cursor year in full, under the table.
    Year,
    /// What the year does: what to do in it, how far the plan has come
    /// and its flows.
    Actions,
    /// The year's money and its tax.
    Money,
}

/// A pane of the year's detail the keyboard walks while it is on show.
#[derive(Component)]
pub(super) struct DetailStop;

const TO_TABLE: &[CommandHint] = &[
    CommandHint {
        commands: &[LEDGER_TABLE],
        key: 0,
        word: "table",
    },
    COLUMNS,
];
const FROM_TABLE: &[CommandHint] = &[
    COLUMNS,
    CommandHint {
        commands: &[LEDGER_TABLE],
        key: 0,
        word: "the year",
    },
];
const COLUMNS: CommandHint = CommandHint {
    commands: &[LEDGER_COLUMNS],
    key: 0,
    word: "columns",
};

/// The year under the table, and within it the node what the year does is
/// stacked down; the money is spawned into the year after it.
pub(super) fn spawn_year(commands: &mut Commands, page: Entity) -> (Entity, Entity) {
    let hints = CommandHints(TO_TABLE);
    let year = Node {
        flex_basis: Val::Px(0.0),
        ..growing()
    };
    let year = commands
        .spawn((year, Part::Year, hints, ChildOf(page)))
        .id();
    let actions = Node {
        flex_direction: FlexDirection::Column,
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        ..Node::default()
    };
    let actions = commands.spawn((actions, Part::Actions, ChildOf(year))).id();
    (year, actions)
}

/// What the view shows, hides and hands the keyboard between.
#[derive(SystemParam)]
struct Laid<'w, 's> {
    parts: Query<'w, 's, (&'static Part, &'static mut Node)>,
    hints: Query<'w, 's, &'static mut CommandHints, With<YearsPane>>,
    tables: Query<'w, 's, Entity, With<LedgerTable>>,
    detail_stops: Query<'w, 's, Entity, With<DetailStop>>,
}

impl Laid<'_, '_> {
    /// Sizes and turns each part of the page: the table to its share over
    /// the year or to the whole page, and the year's two sides beside each
    /// other on a landscape page and one over the other on a portrait one.
    fn lay(&mut self, is_table: bool, shape: Shape, size: TerminalSize) {
        let (across, down) = (FlexDirection::Row, FlexDirection::Column);
        for (part, mut node) in &mut self.parts {
            match (part, shape) {
                (Part::Table, _) if is_table => node.flex_basis = Val::Percent(100.0),
                (Part::Table, _) => node.flex_basis = Val::Px(shape.table_rows(size)),
                (Part::Year, _) => {
                    set_display(&mut node, !is_table);
                    node.flex_direction = if shape.is_landscape() { across } else { down };
                }
                (Part::Actions, Shape::Landscape) => {
                    (node.flex_grow, node.flex_basis) = (ACTIONS_SHARE, Val::Px(0.0));
                }
                (Part::Actions, Shape::Portrait) => {
                    (node.flex_grow, node.flex_basis) = (0.0, Val::Auto);
                }
                (Part::Money, _) => {
                    node.flex_direction = if shape.is_landscape() { down } else { across };
                }
            }
        }
    }
}

/// Lays the page out for the view and the terminal as either changes:
/// under the table alone the year's panes take no room and are not walked
/// to, and the keyboard one of them held goes to the table.
fn arrange(
    (view, shape, size): (Res<LedgerView>, Res<Shape>, Res<TerminalSize>),
    mut laid: Laid,
    mut focus: PageFocus,
    mut commands: Commands,
) {
    if !view.is_changed() && !size.is_changed() {
        return;
    }
    let is_table = *view == LedgerView::Table;
    laid.lay(is_table, *shape, *size);
    for mut hints in &mut laid.hints {
        hints.0 = if is_table { FROM_TABLE } else { TO_TABLE };
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
