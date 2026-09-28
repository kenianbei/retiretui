//! The market tools: the plan run through many markets, the share of them
//! it survives, the runs singled out, and a chart of their spread - one page
//! drawing random markets, one replaying every historical start year. Both
//! search by themselves whenever the plan changes while they are shown,
//! stopping a search under way.

mod assumptions;
mod chart;
mod historical;
mod monte_carlo;
mod views;

#[cfg(test)]
mod tests;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Local, Query, Res, ResMut, Resource};
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Style;
use retiretui_client::searches::markets::{self, Markets, Zone, zone_of};
use retiretui_engine::market::{History, Progress, Run, RunError};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::Projection;

pub(crate) use assumptions::edit_assumption;
pub(crate) use views::cycle_view;

use super::options::spawn_table;
use super::{Found, HelpLine, ResultPane, Tool, count_text, show_help};
use crate::edit::Draft;
use crate::hints::Hints;
use crate::nav::{self, FocusStop, Page, ShownSurface, Turn};
use crate::overview::Better;
use crate::pane::{Framed, Pane};
use crate::session::Session;
use crate::theme::Theme;

/// The historical record the market tools draw from: the user's own where
/// the launch found one, else the embedded record.
#[derive(Resource)]
pub struct MarketHistory(pub History);

impl Default for MarketHistory {
    fn default() -> Self {
        Self(History::embedded().clone())
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<MarketHistory>();
    app.add_plugins((monte_carlo::plugin, historical::plugin));
}

/// Borders, the header, the plan's own row and the six runs Monte Carlo
/// singles out.
const RUNS_ROWS: f32 = 10.0;
/// The Assumptions pane's width, borders included.
const ASSUMPTIONS_COLS: f32 = 34.0;

/// What a market tool searches, and how it names what it found.
pub(crate) trait MarketTool: Found + Markets + Sized {
    const PAGE: Page;
    const HELP: &'static str;
    const TOOL_PAGE: &'static super::ToolPage = &super::ToolPage {
        surface: Self::PAGE,
        panes: spawn_panes::<Self>,
    };
    /// The command ⏎ on a run runs.
    const OPEN: &'static str;
    /// The command ⏎ on an assumption runs.
    const EDIT: &'static str;
    /// The command `v` runs.
    const VIEW: &'static str;

    fn search(
        plan: &Plan,
        tables: &TaxTables,
        history: &History,
        progress: &Progress,
    ) -> Result<Self, RunError>;

    /// How many runs a search of `plan` makes.
    fn total(plan: &Plan) -> usize;

    /// How the Ledger names a run shown in it, from its first cell.
    fn ledger_label(first: &str) -> String;

    /// What the Overview already found over `plan`, taken in place of a
    /// search.
    fn found_by(_better: &Better, _plan: &Plan) -> Option<Self> {
        None
    }
}

fn install<R: MarketTool>(app: &mut App) {
    super::install::<R>(app, R::TOOL_PAGE);
    super::options::plugin::<R>(app);
    app.add_systems(
        Update,
        (
            (search_by_itself::<R>, say_help::<R>).run_if(nav::shows(R::PAGE)),
            title_runs::<R>,
        )
            .after(super::poll_search::<R>),
    );
    assumptions::install::<R>(app);
    chart::install::<R>(app);
    views::install::<R>(app);
}

/// Searches the plan whenever it changes while the page is shown, stopping
/// a search under way.
fn search_by_itself<R: MarketTool>(
    (draft, session, history): (Res<Draft>, Res<Session>, Res<MarketHistory>),
    (shown, better): (ShownSurface, Res<Better>),
    mut searched: Local<Option<Plan>>,
    mut tool: ResMut<Tool<R>>,
) {
    let is_ready = draft.is_changed() || shown.is_changed();
    if !is_ready || !super::is_due(&draft, searched.as_ref(), |plan| *plan == draft.plan) {
        return;
    }
    *searched = Some(draft.plan.clone());
    if let Some(found) = R::found_by(&better, &draft.plan) {
        tool.take(found);
        return;
    }
    let tables = session.tables.clone();
    let history = history.0.clone();
    let total = R::total(&draft.plan);
    tool.restart(draft.plan.clone(), total, move |plan, progress| {
        R::search(plan, &tables, &history, progress)
    });
}

/// Keeps the runs table's title saying how the plan fares, in the colour
/// of the zone its share falls in.
fn title_runs<R: MarketTool>(
    (tool, theme): (Res<Tool<R>>, Res<Theme>),
    mut panes: Query<&mut Framed, bevy_ecs::prelude::With<ResultPane<R>>>,
) {
    if !tool.is_changed() && !theme.is_changed() {
        return;
    }
    let title = tool.found().map_or_else(
        || R::RUN_HEADING.to_owned(),
        |found| format!("{} · {} {}", R::RUN_HEADING, R::HEADLINE, found.verdict()),
    );
    let zone = tool
        .found()
        .map(|found| zone_style(found.runs().success_rate(), &theme));
    for mut pane in &mut panes {
        Framed::retitle(&mut pane, &title);
        Framed::restyle(&mut pane, zone);
    }
}

fn say_help<R: MarketTool>(
    shown: ShownSurface,
    theme: Res<Theme>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    if shown.is_changed() || theme.is_changed() {
        show_help(&mut lines, R::PAGE, R::HELP, &theme);
    }
}

/// How a share of runs reads against the zones.
fn zone_style(share: f64, theme: &Theme) -> Style {
    match zone_of(share) {
        Zone::Good => Style::new().fg(theme.good),
        Zone::Caution => Style::new().fg(theme.caution),
        Zone::Short => theme.exceeded(),
    }
}

/// The runs table's rows: the plan's own, then each run listed.
fn laid<R: MarketTool>(found: &R, plan: &Plan) -> super::options::Laid {
    let runs = found.runs();
    super::options::Laid {
        header: markets::run_columns::<R>().map(str::to_owned).to_vec(),
        current: markets::run_cells(plan, markets::PLANNED.to_owned(), &runs.planned),
        options: found
            .listed()
            .into_iter()
            .map(|listed| markets::run_cells(plan, listed.first, listed.run))
            .collect(),
    }
}

/// The page: the Assumptions pane beside a column of the runs over the
/// chart, which has a pane of its own so that its read-out and the
/// search's note each have a title to say it in.
fn spawn_panes<R: MarketTool>(commands: &mut Commands, row: Entity) {
    assumptions::spawn_pane::<R>(commands, row, ASSUMPTIONS_COLS);
    let column = Node {
        flex_direction: FlexDirection::Column,
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        height: Val::Percent(100.0),
        ..Node::default()
    };
    let column = commands.spawn((column, ChildOf(row))).id();
    let framed = Pane::new(R::RUN_HEADING)
        .tall(RUNS_ROWS)
        .spawn(commands, column);
    let hints = Hints(&[("↑↓", "run"), ("⏎", "open in ledger")]);
    let table = spawn_table::<R>(commands, framed, R::OPEN, hints);
    commands.entity(table).insert(FocusStop);
    chart::spawn_pane::<R>(commands, column);
}

/// The run highlighted in the options table, and its first cell.
fn highlighted<R: MarketTool>(tool: &Tool<R>) -> Option<(String, &Run)> {
    let at = tool.highlighted()?;
    let listed = tool.found()?.listed().into_iter().nth(at)?;
    Some((listed.first, listed.run))
}

/// The `open-*-run` commands: the highlighted run, projected whole, in the
/// Ledger - or, on the plan's own row, the plan's own projection, which is
/// the market it states.
pub(crate) fn open_run<R: MarketTool>(
    (tool, draft, session, history): (Res<Tool<R>>, Res<Draft>, Res<Session>, Res<MarketHistory>),
    mut run: ResMut<crate::session::LedgerRun>,
    mut turn: Turn,
) -> crate::command::Outcome {
    use crate::command::Outcome;
    if tool.found().is_none() {
        return Outcome::Refused(super::NOTHING_SEARCHED_YET.to_owned());
    }
    let Some((label, chosen)) = highlighted(&*tool) else {
        run.0 = None;
        turn.to(Page::Ledger);
        return Outcome::Done;
    };
    let replayed: Option<Projection> =
        retiretui_engine::market::replay(&draft.plan, &session.tables, &history.0, chosen.name);
    let Some(projection) = replayed else {
        return Outcome::Refused(format!("{label} cannot be replayed"));
    };
    let projected = crate::session::Projected {
        plan: draft.plan.clone(),
        projection,
    };
    run.0 = Some((R::ledger_label(&label), projected));
    turn.to(Page::Ledger);
    Outcome::Done
}
