//! The planner's command line: each command reads a plan or scenario,
//! checks it, and prints what it asks of the engine, as text or JSON.
//! Internal to the planner's own crates; it makes no promise of a stable
//! API.

mod actions;
mod compare;
mod import;
mod markets;
mod optimize;
mod project;

use std::path::{Path, PathBuf};

use clap::Subcommand;
use retiretui_client::environment::load_tables;
use retiretui_client::files::{Invalid, validated_plan_with_files};
use retiretui_client::store::DiskStore;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;

use actions::ActionsArgs;
use compare::CompareArgs;
use import::ImportEarningsArgs;
use markets::{HistoricalArgs, MonteCarloArgs};
use optimize::OptimizeCommand;
use project::ProjectArgs;

/// The command line's commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Check a plan file for schema and consistency errors.
    Validate {
        /// Path to the plan TOML file.
        plan: PathBuf,
    },
    /// Project a plan year by year and print the ledger.
    Project(ProjectArgs),
    /// Print one year's concrete to-dos: conversions, RMDs, transfers,
    /// contributions, and funding withdrawals, with warnings.
    Actions(ActionsArgs),
    /// Compare two or more plans or scenarios side by side.
    Compare(CompareArgs),
    /// Search a plan: a Roth conversion ladder, or Social Security claim
    /// ages.
    #[command(subcommand)]
    Optimize(OptimizeCommand),
    /// Run a plan through many random markets, drawn from its assumptions
    /// or from history, and report how often the money lasts.
    MonteCarlo(MonteCarloArgs),
    /// Run a plan through history from every start year and report which
    /// the money would have lasted through.
    Historical(HistoricalArgs),
    /// Pull the earnings record out of a Social Security statement (the
    /// XML from ssa.gov) onto a person, rewriting the plan file.
    ImportEarnings(ImportEarningsArgs),
}

/// Runs `command`, printing what it reports to stdout.
///
/// # Errors
///
/// Where the plan cannot be read or is invalid, its tables or history cannot
/// be loaded, or what the command writes cannot be written.
pub fn run(command: &Command) -> anyhow::Result<()> {
    match command {
        Command::Validate { plan } => run_validate(plan),
        Command::Project(args) => project::run(args),
        Command::Actions(args) => actions::run(args),
        Command::Compare(args) => compare::run(args),
        Command::Optimize(command) => optimize::run(command),
        Command::MonteCarlo(args) => markets::run_monte_carlo(args),
        Command::Historical(args) => markets::run_historical(args),
        Command::ImportEarnings(args) => import::run(args),
    }
}

fn run_validate(path: &Path) -> anyhow::Result<()> {
    let tables = load_tables(&[])?;
    load_validated_plan(path, &tables)?;
    println!("{}: ok", path.display());
    Ok(())
}

/// Loads a plan or scenario file and refuses an invalid one, printing its
/// issues to stderr.
fn load_validated_plan(path: &Path, tables: &TaxTables) -> anyhow::Result<Plan> {
    let invalid = match validated_plan_with_files(&DiskStore, path, tables).0 {
        Ok(plan) => return Ok(plan),
        Err(invalid) => invalid,
    };
    match &invalid {
        Invalid::Load(message) => message.lines().skip(1).for_each(|line| eprintln!("{line}")),
        Invalid::Issues { issues, .. } => issues.iter().for_each(|issue| eprintln!("{issue}")),
    }
    Err(anyhow::Error::msg(invalid.headline().to_owned()))
}
