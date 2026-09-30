//! The Assumptions pane: what a market tool's runs were made under, read
//! from the plan, headed by how the plan fared; ⏎ on a row turns to the
//! page it is edited on.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, Res};
use plurimus::ui::ScrollArea;
use plurimus::widgets::{ActiveDescendant, WidgetSystems};
use retiretui_client::present;
use retiretui_client::searches::markets;
use retiretui_engine::plan::Plan;

use super::MarketTool;
use crate::command::Outcome;
use crate::edit::{Draft, page_of, table_bundle};
use crate::hints::Hints;
use crate::layout::{self, filling, placed};
use crate::nav::{FocusStop, Page, Turn};
use crate::pane::Pane;
use crate::tabulate;
use crate::tools::{EnterRuns, Tool, handle_enter};

const TITLE: &str = "Assumptions";
const HEADER: [&str; 2] = ["", ""];
const GAP: u16 = 1;
const NOT_YET: &str = "running…";

/// The table a tool's assumptions are rows of.
#[derive(Component)]
pub(crate) struct AssumptionsTable(Page);

/// The page a row's assumption is edited on.
#[derive(Component, Clone, Copy)]
pub(crate) struct EditedOn(Page);

pub(super) fn install<R: MarketTool>(app: &mut App) {
    app.add_systems(Update, refresh::<R>.before(WidgetSystems::Layout));
}

pub(super) fn spawn_pane<R: MarketTool>(commands: &mut Commands, row: Entity, cols: f32) {
    let pane = Pane::new(TITLE).wide(cols).spawn(commands, row);
    commands
        .spawn((
            table_bundle(),
            AssumptionsTable(R::PAGE),
            EnterRuns(R::EDIT),
            FocusStop,
            Hints(&[("↑↓", "assumption"), ("⏎", "edit")]),
            layout::Rests,
            filling(),
            placed(),
            ChildOf(pane),
        ))
        .observe(handle_enter);
}

/// A row: what it says, and the page it is edited on.
type Row = (&'static str, String, Page);

fn rows<R: MarketTool>(plan: &Plan, found: Option<&R>) -> Vec<Row> {
    let verdict = found.map_or_else(|| NOT_YET.to_owned(), R::verdict);
    let mut rows = vec![(present::MONEY_LASTS, verdict, Page::Market)];
    rows.extend(
        markets::assumptions::<R>(plan)
            .into_iter()
            .map(|row| (row.label, row.value, page_of(row.domain))),
    );
    rows
}

/// Rewrites the table whenever the plan or what the search found moves.
fn refresh<R: MarketTool>(
    (tool, draft): (Res<Tool<R>>, Res<Draft>),
    mut tables: Query<(Entity, &AssumptionsTable, &mut ScrollArea)>,
    mut commands: Commands,
) {
    if !(tool.is_changed() || draft.is_changed()) {
        return;
    }
    let assumptions = rows::<R>(&draft.plan, tool.found());
    let header = HEADER.map(str::to_owned);
    let cells: Vec<Vec<String>> = assumptions
        .iter()
        .map(|(label, value, _)| vec![(*label).to_owned(), value.clone()])
        .collect();
    let columns = tabulate::labelled(&cells, GAP);
    for (table, shown, mut scroll) in &mut tables {
        if shown.0 != R::PAGE {
            continue;
        }
        commands.entity(table).insert(columns.clone());
        let spawned = tabulate::refill(
            &mut commands,
            (table, &mut scroll),
            (&header, &cells),
            &[0, 1],
        );
        for (&row, (_, _, page)) in spawned.iter().zip(&assumptions) {
            commands.entity(row).insert(EditedOn(*page));
        }
    }
}

/// The `*-assumption` commands: turns to the page the highlighted
/// assumption is edited on.
pub(crate) fn edit_assumption<R: MarketTool>(
    tables: Query<(&AssumptionsTable, &ActiveDescendant)>,
    rows: Query<&EditedOn>,
    mut turn: Turn,
) -> Outcome {
    let row = tables
        .iter()
        .find(|(table, _)| table.0 == R::PAGE)
        .and_then(|(_, on)| on.0);
    let Some(&EditedOn(page)) = row.and_then(|row| rows.get(row).ok()) else {
        return Outcome::Done;
    };
    turn.to(page);
    Outcome::Done
}
