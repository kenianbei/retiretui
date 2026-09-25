//! `monte-carlo` and `historical`: a plan run through many markets, and
//! what is shown of the runs - the share that succeeded, the runs singled
//! out, and for Monte Carlo the spread of net worth year by year - every
//! figure in today's dollars by each run's own inflation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Args, ValueEnum};
use retiretui_engine::market::{
    self, BAND_PERCENTILES, Band, History, MonteCarlo, Progress, Run, RunError, RunName, Runs,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Draw, Market, Plan};
use schemars::JsonSchema;
use serde::Serialize;

use crate::commands::project::OutputFormat;
use crate::commands::table::{align, plain_dollars, rate};
use crate::commands::user_config_dir;
use retiretui_engine::project::validate_plan;

/// Where the historical record is read from in place of the embedded one.
const HISTORY_FILE: &str = "history.toml";

/// Arguments of `monte-carlo`.
#[derive(Args)]
pub struct MonteCarloArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// Where the markets are drawn from, in place of the plan's.
    #[arg(long, value_enum)]
    pub draw: Option<DrawArg>,
    /// How many markets are run, in place of the plan's.
    #[arg(long)]
    pub trials: Option<u32>,
    /// The seed the markets are drawn from, in place of the plan's.
    #[arg(long)]
    pub seed: Option<u32>,
    #[command(flatten)]
    pub common: MarketArgs,
}

/// Arguments of `historical`.
#[derive(Args)]
pub struct HistoricalArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// The first start year tried, in place of the plan's.
    #[arg(long)]
    pub from: Option<i16>,
    /// The last start year tried, in place of the plan's.
    #[arg(long)]
    pub to: Option<i16>,
    /// Try only start years with a whole horizon of history.
    #[arg(long)]
    pub no_wrap: bool,
    #[command(flatten)]
    pub common: MarketArgs,
}

/// What both commands take besides their settings.
#[derive(Args)]
pub struct MarketArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// A historical record of the embedded one's shape to run in its place.
    #[arg(long)]
    pub history: Option<PathBuf>,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

/// Where a Monte Carlo search draws from.
#[derive(Clone, Copy, ValueEnum)]
pub enum DrawArg {
    /// Random years from the plan's assumptions.
    Assumptions,
    /// Random historical years.
    History,
}

/// One run as the replies show it.
#[derive(Serialize, JsonSchema)]
pub struct RunEntry {
    /// What the run is: "as planned", a percentile, "worst", or a start
    /// year.
    pub market: String,
    /// The Monte Carlo trial it was.
    pub trial: Option<u32>,
    /// The historical year it started in.
    pub start: Option<i16>,
    /// Net worth at the horizon, today's dollars.
    pub ending: Dollars,
    /// Spending left uncovered over the run, today's dollars.
    pub unfunded: Dollars,
    /// The first year spending went uncovered.
    pub first_short: Option<i16>,
    /// Never short, and leaving at least what the plan asks.
    pub success: bool,
}

/// A Monte Carlo search, as `monte-carlo` and `plan_monte_carlo` reply.
#[derive(Serialize, JsonSchema)]
pub struct MonteCarloReply {
    /// "assumptions" or "history".
    pub draw: String,
    /// The seed the markets were drawn from.
    pub seed: u32,
    /// How many markets ran.
    pub runs: usize,
    /// How many succeeded.
    pub successes: usize,
    /// The share that succeeded.
    pub success_rate: f64,
    /// The plan as planned, then the markets singled out, best first.
    pub markets: Vec<RunEntry>,
    /// Net worth year by year across the markets.
    pub bands: Vec<Band>,
}

/// A Historical search, as `historical` and `plan_historical` reply.
#[derive(Serialize, JsonSchema)]
pub struct HistoricalReply {
    /// The first start year tried.
    pub from: i16,
    /// The last start year tried.
    pub to: i16,
    /// Whether histories went on from the record's start past its end.
    pub wrap: bool,
    /// How many start years ran.
    pub runs: usize,
    /// How many succeeded.
    pub successes: usize,
    /// The share that succeeded.
    pub success_rate: f64,
    /// The plan as planned, then every start year, worst first.
    pub start_years: Vec<RunEntry>,
}

fn entry(market: String, run: &Run) -> RunEntry {
    let (trial, start) = match run.name {
        RunName::Planned => (None, None),
        RunName::Trial(trial) => (Some(trial), None),
        RunName::Start(start) => (None, Some(start)),
    };
    RunEntry {
        market,
        trial,
        start,
        ending: run.ending,
        unfunded: run.unfunded,
        first_short: run.first_short,
        success: run.is_success,
    }
}

/// What the plan's own market is called among the runs.
const PLANNED: &str = "as planned";

impl MonteCarloReply {
    pub fn new(plan: &Plan, found: &MonteCarlo) -> Self {
        let runs = &found.runs;
        let labels = BAND_PERCENTILES
            .iter()
            .rev()
            .map(|&percentile| percentile_label(percentile))
            .chain(std::iter::once("worst".to_owned()));
        let mut markets = vec![entry(PLANNED.to_owned(), &runs.planned)];
        markets.extend(
            labels
                .zip(&found.singled_out)
                .map(|(label, &at)| entry(label, &runs.runs[at])),
        );
        Self {
            draw: plan.market().draw().as_str().to_owned(),
            seed: plan.market().seed(),
            runs: runs.runs.len(),
            successes: runs.successes,
            success_rate: runs.success_rate(),
            markets,
            bands: runs.bands.clone(),
        }
    }
}

impl HistoricalReply {
    pub fn new(plan: &Plan, runs: &Runs) -> Self {
        let mut start_years = vec![entry(PLANNED.to_owned(), &runs.planned)];
        start_years.extend(runs.worst_first().into_iter().map(|at| {
            let run = &runs.runs[at];
            let label = match run.name {
                RunName::Start(start) => start.to_string(),
                RunName::Planned | RunName::Trial(_) => String::new(),
            };
            entry(label, run)
        }));
        Self {
            from: plan.market().from(),
            to: plan.market().to(),
            wrap: plan.market().wrap(),
            runs: runs.runs.len(),
            successes: runs.successes,
            success_rate: runs.success_rate(),
            start_years,
        }
    }
}

/// How a market singled out at `percentile` is named.
pub(crate) fn percentile_label(percentile: u8) -> String {
    format!("{percentile}th percentile")
}

/// The historical record: `explicit`, else the user's own under the config
/// directory, else the embedded one.
pub(crate) fn load_history(explicit: Option<&Path>) -> anyhow::Result<History> {
    let user = user_config_dir("market").map(|dir| dir.join(HISTORY_FILE));
    let Some(path) = explicit
        .map(Path::to_path_buf)
        .or_else(|| user.filter(|path| path.is_file()))
    else {
        return Ok(History::embedded().clone());
    };
    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    History::from_toml_str(&text).with_context(|| format!("in {}", path.display()))
}

/// The plan's `[market]`, created where it states none, for a flag to set.
fn market_of(plan: &mut Plan) -> &mut Market {
    plan.market.get_or_insert_with(Market::default)
}

/// Why a search answered nothing, as the CLI and MCP say it.
pub(crate) fn run_refusal(error: RunError) -> String {
    match error {
        RunError::Cancelled => "the search was cancelled".to_owned(),
        RunError::Refused(issues) => crate::commands::issue_listing(&issues),
    }
}

/// Loads the plan and its surroundings, lets `settle` apply the flags, and
/// checks the plan again with them.
fn prepare(
    path: &Path,
    common: &MarketArgs,
    settle: impl FnOnce(&mut Plan),
) -> anyhow::Result<(Plan, TaxTables, History)> {
    let tables = crate::commands::load_tables(&common.tax_dir)?;
    let mut plan = crate::commands::load_validated_plan(path, &tables)?;
    settle(&mut plan);
    let issues = validate_plan(&plan, &tables);
    if !issues.is_empty() {
        anyhow::bail!(crate::commands::issue_listing(&issues));
    }
    Ok((plan, tables, load_history(common.history.as_deref())?))
}

pub fn run_monte_carlo(args: &MonteCarloArgs) -> anyhow::Result<()> {
    let (plan, tables, history) = prepare(&args.plan, &args.common, |plan| {
        let settings = market_of(plan).monte_carlo.get_or_insert_default();
        if let Some(draw) = args.draw {
            settings.draw = Some(match draw {
                DrawArg::Assumptions => Draw::Assumptions,
                DrawArg::History => Draw::History,
            });
        }
        settings.trials = args.trials.or(settings.trials);
        settings.seed = args.seed.or(settings.seed);
    })?;
    let found = market::monte_carlo(&plan, &tables, &history, &Progress::default())
        .map_err(|error| anyhow::Error::msg(run_refusal(error)))?;
    let reply = MonteCarloReply::new(&plan, &found);
    match args.common.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&reply)?),
        OutputFormat::Table => print!("{}", monte_carlo_text(&reply)),
    }
    Ok(())
}

pub fn run_historical(args: &HistoricalArgs) -> anyhow::Result<()> {
    let (plan, tables, history) = prepare(&args.plan, &args.common, |plan| {
        let settings = market_of(plan).historical.get_or_insert_default();
        settings.from = args.from.or(settings.from);
        settings.to = args.to.or(settings.to);
        if args.no_wrap {
            settings.wrap = Some(false);
        }
    })?;
    let found = market::historical(&plan, &tables, &history, &Progress::default())
        .map_err(|error| anyhow::Error::msg(run_refusal(error)))?;
    let reply = HistoricalReply::new(&plan, &found);
    match args.common.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&reply)?),
        OutputFormat::Table => print!("{}", historical_text(&reply)),
    }
    Ok(())
}

fn short_text(first_short: Option<i16>) -> String {
    first_short.map_or_else(|| "never".to_owned(), |year| year.to_string())
}

fn runs_table(first: &str, entries: &[RunEntry]) -> String {
    let header = [first, "ends with", "short in"].map(str::to_owned);
    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|entry| {
            vec![
                entry.market.clone(),
                plain_dollars(entry.ending),
                short_text(entry.first_short),
            ]
        })
        .collect();
    align(&header, &rows)
}

fn monte_carlo_text(reply: &MonteCarloReply) -> String {
    let mut header = vec!["year".to_owned()];
    header.extend(
        BAND_PERCENTILES
            .iter()
            .map(|percentile| format!("{percentile}th")),
    );
    header.push("funded".to_owned());
    let rows: Vec<Vec<String>> = reply
        .bands
        .iter()
        .map(|band| {
            let mut cells = vec![band.year.to_string()];
            cells.extend(band.net_worth.iter().map(|&worth| plain_dollars(worth)));
            cells.push(rate(band.funded));
            cells
        })
        .collect();
    format!(
        "Money lasts in {} of {} markets drawn from {} (seed {}), in today's dollars\n\n{}\n{}",
        rate(reply.success_rate),
        reply.runs,
        reply.draw,
        reply.seed,
        runs_table("market", &reply.markets),
        align(&header, &rows),
    )
}

fn historical_text(reply: &HistoricalReply) -> String {
    let wrapped = if reply.wrap { ", wrapped" } else { "" };
    format!(
        "Survived {} of {} start years ({}-{}{wrapped}), {}, in today's dollars\n\n{}",
        reply.successes,
        reply.runs,
        reply.from,
        reply.to,
        rate(reply.success_rate),
        runs_table("started", &reply.start_years),
    )
}
