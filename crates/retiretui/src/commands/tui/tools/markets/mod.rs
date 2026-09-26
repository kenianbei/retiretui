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
use retiretui_engine::market::{History, Progress, Run, RunError, Runs};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::Projection;

pub(crate) use assumptions::edit_assumption;
pub(crate) use views::cycle_view;

use super::options::spawn_table;
use super::{Found, HelpLine, ResultPane, Tool, count_text, show_help};
use crate::commands::tui::edit::Draft;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::nav::{ActivePage, FocusStop, Page, Turn};
use crate::commands::tui::pane::{Framed, Pane};
use crate::commands::tui::present::{self, compact_dollars};
use crate::commands::tui::session::Session;
use crate::commands::tui::theme::Theme;

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

/// A share this high or above reads as comfortable, the zones' upper edge.
const GOOD_ZONE: f64 = 0.9;
/// A share this high or above, and under [`GOOD_ZONE`], reads as close.
const CAUTION_ZONE: f64 = 0.75;
/// Borders, the header, the plan's own row and the six runs Monte Carlo
/// singles out.
const RUNS_ROWS: f32 = 10.0;
/// The Assumptions pane's width, borders included.
const ASSUMPTIONS_COLS: f32 = 34.0;
const NEVER: &str = "never";
/// What a market tool's runs table says before its first search answers.
const NOTHING_SEARCHED: &str = "Runs by itself while this page is shown.";

/// What a market tool searches, and how it names what it found.
pub(crate) trait MarketTool: Found + Sized {
    const PAGE: Page;
    /// The runs table's first column.
    const RUN_HEADING: &'static str;
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
    /// What the Assumptions pane's first row is called.
    const VERDICT: &'static str;
    /// What the runs table's title says before the verdict.
    const HEADLINE: &'static str;
    /// Whether `v` offers net worth year by year at each percentile.
    const HAS_BY_YEAR: bool;

    fn search(
        plan: &Plan,
        tables: &TaxTables,
        history: &History,
        progress: &Progress,
    ) -> Result<Self, RunError>;

    /// How many runs a search of `plan` makes.
    fn total(plan: &Plan) -> usize;

    fn runs(&self) -> &Runs;

    /// The options table's runs, in its order, each with its first cell.
    fn listed(&self) -> Vec<(String, &Run)>;

    /// How the Ledger names a run shown in it, from its first cell.
    fn ledger_label(first: &str) -> String;

    /// How the plan fared, as the Assumptions pane's first row says it.
    fn verdict(&self) -> String;

    /// What the tool runs under, as rows of the Assumptions pane after
    /// the verdict.
    fn settings(plan: &Plan) -> Vec<assumptions::Assumption>;
}

/// The engine's refusals as the tool says them; a search cancelled for a
/// newer one says nothing.
fn issues_of(error: RunError) -> Vec<Issue> {
    match error {
        RunError::Refused(issues) => issues,
        RunError::Cancelled => Vec::new(),
    }
}

fn install<R: MarketTool>(app: &mut App) {
    super::install::<R>(app, R::TOOL_PAGE);
    super::options::plugin::<R>(app);
    app.add_systems(
        Update,
        (search_by_itself::<R>, title_runs::<R>, say_help::<R>).after(super::poll_search::<R>),
    );
    assumptions::install::<R>(app);
    chart::install::<R>(app);
    views::install::<R>(app);
}

/// Searches the plan whenever it changes while the page is shown, stopping
/// a search under way.
fn search_by_itself<R: MarketTool>(
    (draft, session, history): (Res<Draft>, Res<Session>, Res<MarketHistory>),
    active: Res<ActivePage>,
    mut searched: Local<Option<Plan>>,
    mut tool: ResMut<Tool<R>>,
) {
    let is_ready = (draft.is_changed() || active.is_changed()) && active.page() == R::PAGE;
    if !is_ready || !super::is_due(&draft, searched.as_ref(), |plan| *plan == draft.plan) {
        return;
    }
    *searched = Some(draft.plan.clone());
    let tables = session.tables.clone();
    let history = history.0.clone();
    let total = R::total(&draft.plan);
    tool.restart(draft.plan.clone(), total, move |plan, progress| {
        R::search(plan, &tables, &history, progress).map_err(issues_of)
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
    active: Res<ActivePage>,
    theme: Res<Theme>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    if (active.is_changed() || theme.is_changed()) && active.page() == R::PAGE {
        show_help(&mut lines, R::PAGE, R::HELP, &theme);
    }
}

/// How a share of runs reads against the zones.
fn zone_style(share: f64, theme: &Theme) -> Style {
    if share >= GOOD_ZONE {
        Style::new().fg(theme.good)
    } else if share >= CAUTION_ZONE {
        Style::new().fg(theme.caution)
    } else {
        theme.exceeded()
    }
}

/// A run's row: its first cell, what it ends with, and the year it first
/// falls short, with the eldest's age then.
fn run_cells(plan: &Plan, first: String, run: &Run) -> Vec<String> {
    let short = run.first_short.map_or_else(
        || NEVER.to_owned(),
        |year| match plan.household.people.first() {
            Some(person) => format!("{year} ({})", person.age_in_year(year)),
            None => year.to_string(),
        },
    );
    vec![first, compact_dollars(run.ending), short]
}

/// The runs table's rows: the plan's own, then each run listed.
fn laid<R: MarketTool>(found: &R, plan: &Plan) -> super::options::Laid {
    let runs = found.runs();
    super::options::Laid {
        header: [R::RUN_HEADING, "Ends with", "Short in"]
            .map(str::to_owned)
            .to_vec(),
        current: run_cells(plan, "As planned".to_owned(), &runs.planned),
        options: found
            .listed()
            .into_iter()
            .map(|(first, run)| run_cells(plan, first, run))
            .collect(),
    }
}

/// The share that succeeded, as a percent.
fn share_text(runs: &Runs) -> String {
    present::rate(runs.success_rate())
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
    tool.found()?.listed().into_iter().nth(at)
}

/// The `open-*-run` commands: the highlighted run, projected whole, in the
/// Ledger - or, on the plan's own row, the plan's own projection, which is
/// the market it states.
pub(crate) fn open_run<R: MarketTool>(
    (tool, draft, session, history): (Res<Tool<R>>, Res<Draft>, Res<Session>, Res<MarketHistory>),
    mut run: ResMut<crate::commands::tui::session::LedgerRun>,
    mut turn: Turn,
) -> crate::commands::tui::command::Outcome {
    use crate::commands::tui::command::Outcome;
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
    let projected = crate::commands::tui::session::Projected {
        plan: draft.plan.clone(),
        projection,
    };
    run.0 = Some((R::ledger_label(&label), projected));
    turn.to(Page::Ledger);
    Outcome::Done
}
