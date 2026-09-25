use std::path::{Path, PathBuf};

use clap::Args;
use retiretui_engine::optimize::LadderStep;
use retiretui_engine::optimize::{
    BracketSweep, OptimizeOptions, OptimizedLadder, ladder_overlay, optimize_conversions,
    sweep_brackets,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{Projection, Summary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::project::OutputFormat;
use crate::commands::table::{align, display_dollars, summary_table};

/// Arguments of the `optimize` subcommand.
#[derive(Args)]
pub struct OptimizeArgs {
    /// Path to the plan or scenario TOML file.
    pub plan: PathBuf,
    /// Deferred source account id, drained in the given order (repeatable).
    #[arg(long, required = true)]
    pub from: Vec<String>,
    /// Roth destination account id; every source must share its owner.
    #[arg(long)]
    pub to: String,
    /// Bracket to fill, as a percent (e.g. 22); absent sweeps every
    /// fillable bracket and prints the comparison.
    #[arg(long)]
    pub bracket: Option<f64>,
    #[command(flatten)]
    pub constraints: LadderConstraints,
    /// Write the ladder as a scenario overlay file (requires --bracket).
    #[arg(long, requires = "bracket")]
    pub write: Option<PathBuf>,
    /// Show future (nominal) dollars instead of today's dollars.
    #[arg(long)]
    pub nominal: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

/// What a ladder is held to, as `optimize conversions` and the MCP
/// optimizer tools take it.
#[derive(Args, Deserialize, JsonSchema)]
pub struct LadderConstraints {
    /// First conversion year; defaults to plan start.
    #[arg(long)]
    pub start_year: Option<i16>,
    /// Last conversion year; defaults to the year before the owner's RMDs.
    #[arg(long)]
    pub end_year: Option<i16>,
    /// Cap on any single year's conversion.
    #[arg(long)]
    pub annual_max: Option<Dollars>,
    /// Cap on total conversions across the ladder.
    #[arg(long)]
    pub total_max: Option<Dollars>,
    /// Dollars left unfilled below the bracket top.
    #[arg(long, default_value_t = 0)]
    #[serde(default)]
    pub headroom: Dollars,
    /// Highest IRMAA tier the ladder may buy (0 = under every surcharge);
    /// requires a `[medicare]` section in the plan.
    #[arg(long)]
    pub irmaa_tier: Option<u8>,
    /// Explicit MAGI ceiling in today's dollars.
    #[arg(long)]
    pub max_magi: Option<Dollars>,
}

impl LadderConstraints {
    /// The options a ladder from `sources` into `destination` is searched
    /// under.
    pub fn options(&self, sources: &[String], destination: &str) -> OptimizeOptions {
        OptimizeOptions {
            sources: sources.to_vec(),
            destination: destination.to_owned(),
            start_year: self.start_year,
            end_year: self.end_year,
            annual_max: self.annual_max,
            total_max: self.total_max,
            headroom: self.headroom,
            irmaa_tier: self.irmaa_tier,
            max_magi: self.max_magi,
        }
    }
}

/// A bracket sweep, as `optimize conversions` and
/// `sweep_conversion_brackets` reply.
#[derive(Serialize, JsonSchema)]
pub struct SweepReply {
    /// The plan without any ladder.
    pub baseline: Summary,
    /// One entry per fillable bracket, ascending by rate.
    pub brackets: Vec<SweepEntry>,
}

/// One bracket of a sweep.
#[derive(Serialize, JsonSchema)]
pub struct SweepEntry {
    /// The bracket's rate (e.g. 0.22).
    pub bracket_rate: f64,
    /// The ladder's total, on the reply's dollar basis.
    pub total_converted: Dollars,
    /// Headline figures with the ladder applied.
    pub optimized: Summary,
}

impl SweepReply {
    pub fn new(sweep: &BracketSweep, deflated: bool) -> Self {
        Self {
            baseline: sweep.baseline.summary(deflated),
            brackets: sweep
                .brackets
                .iter()
                .map(|bracket| SweepEntry {
                    bracket_rate: bracket.rate,
                    total_converted: bracket.converted(deflated),
                    optimized: bracket.optimized.summary(deflated),
                })
                .collect(),
        }
    }
}

/// A searched ladder, as `optimize conversions --bracket` and
/// `optimize_conversions` reply.
#[derive(Serialize, JsonSchema)]
pub struct LadderReply {
    /// The per-year conversions, in year order.
    pub steps: Vec<LadderStep>,
    /// The ladder's total, on the reply's dollar basis.
    pub total_converted: Dollars,
    /// Headline figures without the ladder.
    pub baseline: Summary,
    /// Headline figures with the ladder applied.
    pub optimized: Summary,
}

impl LadderReply {
    pub fn new(ladder: &OptimizedLadder, deflated: bool) -> Self {
        Self {
            steps: ladder.ladder.steps.clone(),
            total_converted: ladder.ladder.converted(deflated),
            baseline: ladder.baseline.summary(deflated),
            optimized: ladder.ladder.optimized.summary(deflated),
        }
    }
}

pub fn run(args: &OptimizeArgs) -> anyhow::Result<()> {
    let tables = crate::commands::load_tables(&args.tax_dir)?;
    let plan = crate::commands::load_validated_plan(&args.plan, &tables)?;
    match args.bracket {
        Some(percent) => run_single(&plan, &tables, percent / 100.0, args),
        None => run_sweep(&plan, &tables, args),
    }
}

fn run_sweep(plan: &Plan, tables: &TaxTables, args: &OptimizeArgs) -> anyhow::Result<()> {
    let sweep = sweep_brackets(
        plan,
        tables,
        &args.constraints.options(&args.from, &args.to),
    )
    .map_err(|issues| anyhow::Error::msg(crate::commands::issue_listing(&issues)))?;
    let deflated = !args.nominal;
    match args.format {
        OutputFormat::Json => {
            let reply = SweepReply::new(&sweep, deflated);
            println!("{}", serde_json::to_string_pretty(&reply)?);
        }
        OutputFormat::Table => print!("{}", sweep_table(&sweep, deflated)),
    }
    Ok(())
}

fn sweep_table(sweep: &BracketSweep, deflated: bool) -> String {
    let mut rows = vec![(
        vec!["baseline".to_owned(), "0".to_owned()],
        sweep.baseline.summary(deflated),
    )];
    rows.extend(sweep.brackets.iter().map(|bracket| {
        let cells = vec![
            format!("{:.0}%", bracket.rate * 100.0),
            bracket.converted(deflated).to_string(),
        ];
        (cells, bracket.optimized.summary(deflated))
    }));
    summary_table(&["bracket", "converted"], rows)
}

fn run_single(
    plan: &Plan,
    tables: &TaxTables,
    rate: f64,
    args: &OptimizeArgs,
) -> anyhow::Result<()> {
    let options = args.constraints.options(&args.from, &args.to);
    let ladder = optimize_conversions(plan, tables, &options, rate)
        .map_err(|issues| anyhow::Error::msg(crate::commands::issue_listing(&issues)))?;
    let deflated = !args.nominal;
    match args.format {
        OutputFormat::Json => {
            let reply = LadderReply::new(&ladder, deflated);
            println!("{}", serde_json::to_string_pretty(&reply)?);
        }
        OutputFormat::Table => {
            print!(
                "{}",
                ladder_table(&ladder.ladder.steps, &ladder.ladder.optimized, args.nominal)
            );
            println!();
            print!("{}", comparison_table(&ladder, deflated));
        }
    }
    if let Some(out) = &args.write {
        write_overlay(out, &args.plan, plan, &options, &ladder)?;
        println!("wrote {}", out.display());
    }
    Ok(())
}

/// Each step beside the taxable income it leaves that year with.
fn ladder_table(steps: &[LadderStep], optimized: &Projection, nominal: bool) -> String {
    let header: Vec<String> = ["year", "source", "amount", "taxable"]
        .map(str::to_owned)
        .into();
    let rows: Vec<Vec<String>> = steps
        .iter()
        .map(|step| {
            let row = optimized.years.iter().find(|row| row.year == step.year);
            let deflator = row.map_or(1.0, |row| row.deflator);
            let taxable = row.map_or(0, |row| row.taxes.ordinary_taxable);
            vec![
                step.year.to_string(),
                step.source.clone(),
                display_dollars(step.amount, deflator, nominal),
                display_dollars(taxable, deflator, nominal),
            ]
        })
        .collect();
    align(&header, &rows)
}

fn comparison_table(ladder: &OptimizedLadder, deflated: bool) -> String {
    let baseline = vec!["baseline".to_owned(), "0".to_owned()];
    let optimized = vec![
        "optimized".to_owned(),
        ladder.ladder.converted(deflated).to_string(),
    ];
    summary_table(
        &["plan", "converted"],
        vec![
            (baseline, ladder.baseline.summary(deflated)),
            (optimized, ladder.ladder.optimized.summary(deflated)),
        ],
    )
}

fn write_overlay(
    out: &Path,
    plan_path: &Path,
    plan: &Plan,
    options: &OptimizeOptions,
    ladder: &OptimizedLadder,
) -> anyhow::Result<()> {
    let base = crate::commands::overlay_base(out, plan_path)?;
    let overlay = ladder_overlay(&base, plan, options, &ladder.ladder.steps)?;
    crate::commands::write_atomic(out, &overlay)?;
    Ok(())
}
