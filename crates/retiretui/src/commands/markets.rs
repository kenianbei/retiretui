//! `monte-carlo` and `historical`: a plan run through many markets, and
//! what is shown of the runs - the share that succeeded, the runs singled
//! out, and for Monte Carlo the spread of net worth year by year - every
//! figure in today's dollars by each run's own inflation.

use retiretui_client::replies::{HistoricalReply, MonteCarloReply, RunEntry};
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use retiretui_engine::market::{self, BAND_PERCENTILES, History, Progress};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Draw, Market, Plan};

use crate::commands::project::OutputFormat;
use retiretui_client::environment::{load_history, load_tables};
use retiretui_client::searches::run_refusal;
use retiretui_client::table::{align, plain_dollars, rate};
use retiretui_engine::project::validate_plan;

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

/// The historical record: `explicit`, else the user's own under the config
/// directory, else the embedded one.
/// The plan's `[market]`, created where it states none, for a flag to set.
fn market_of(plan: &mut Plan) -> &mut Market {
    plan.market.get_or_insert_with(Market::default)
}

/// Loads the plan and its surroundings, lets `settle` apply the flags, and
/// checks the plan again with them.
fn prepare(
    path: &Path,
    common: &MarketArgs,
    settle: impl FnOnce(&mut Plan),
) -> anyhow::Result<(Plan, TaxTables, History)> {
    let tables = load_tables(&common.tax_dir)?;
    let mut plan = crate::commands::load_validated_plan(path, &tables)?;
    settle(&mut plan);
    let issues = validate_plan(&plan, &tables);
    if !issues.is_empty() {
        anyhow::bail!(retiretui_client::issues::issue_listing(&issues));
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
