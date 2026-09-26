//! The Assumptions pane: what a market tool's runs were made under, read
//! from the plan, headed by how the plan fared; ⏎ on a row turns to the
//! page it is edited on.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, Res, ResMut};
use plurimus::ui::ScrollArea;
use plurimus::widgets::{ActiveDescendant, WidgetSystems};
use retiretui_engine::plan::{Account, AssetClass, Plan};

use super::MarketTool;
use crate::commands::tui::command::Outcome;
use crate::commands::tui::edit::{Draft, table_bundle};
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, filling, placed};
use crate::commands::tui::nav::{ActivePage, FocusStop, Page};
use crate::commands::tui::pane::Pane;
use crate::commands::tui::present::{money, rate};
use crate::commands::tui::tabulate;
use crate::commands::tui::tools::{EnterRuns, Tool, handle_enter};

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
pub(crate) type Assumption = (&'static str, String, Page);

/// What success means under the plan's `[market]`.
fn success(plan: &Plan) -> String {
    plan.market().leave_at_least().map_or_else(
        || "Never short".to_owned(),
        |floor| format!("Leaves {}", money(floor)),
    )
}

/// Each asset class's return and inflation's, as the plan assumes them.
pub(super) fn assumed(plan: &Plan) -> Vec<Assumption> {
    let market = plan.market();
    let mut rows: Vec<Assumption> = [
        ("Stocks", AssetClass::Stocks),
        ("Bonds", AssetClass::Bonds),
        ("Cash", AssetClass::Cash),
    ]
    .into_iter()
    .map(|(label, class)| {
        let spread = format!(
            "{} ± {}",
            rate(market.mean(class)),
            rate(market.volatility(class))
        );
        (label, spread, Page::Market)
    })
    .collect();
    let inflation = format!(
        "{} ± {}",
        rate(plan.plan.inflation),
        rate(market.inflation_volatility())
    );
    rows.push(("Inflation", inflation, Page::Market));
    rows
}

/// The accounts no mix is held in, which every market leaves alone.
fn unmixed(plan: &Plan) -> Option<Assumption> {
    let names: Vec<&str> = plan
        .accounts
        .iter()
        .filter(|account| account.allocation.is_none())
        .map(Account::display_name)
        .collect();
    (!names.is_empty()).then(|| ("No mix", names.join(", "), Page::Accounts))
}

fn rows<R: MarketTool>(plan: &Plan, found: Option<&R>) -> Vec<Assumption> {
    let verdict = found.map_or_else(|| NOT_YET.to_owned(), R::verdict);
    let mut rows = vec![
        (R::VERDICT, verdict, Page::Market),
        ("Success", success(plan), Page::Market),
    ];
    rows.extend(R::settings(plan));
    rows.extend(unmixed(plan));
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
    mut active: ResMut<ActivePage>,
) -> Outcome {
    let row = tables
        .iter()
        .find(|(table, _)| table.0 == R::PAGE)
        .and_then(|(_, on)| on.0);
    let Some(&EditedOn(page)) = row.and_then(|row| rows.get(row).ok()) else {
        return Outcome::Done;
    };
    active.0 = page;
    Outcome::Done
}
