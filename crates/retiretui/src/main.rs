//! Retirement planning application for the terminal.

mod commands;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use commands::actions::ActionsArgs;
use commands::compare::CompareArgs;
use commands::import::ImportEarningsArgs;
use commands::markets::{HistoricalArgs, MonteCarloArgs};
use commands::mcp::McpArgs;
use commands::optimize::OptimizeCommand;
use commands::project::ProjectArgs;
use commands::tui::TuiArgs;

#[derive(Parser)]
#[command(version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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
    /// Open the interactive dashboard for a plan or scenario.
    Tui(TuiArgs),
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
    /// Serve plans to AI agents over the Model Context Protocol on stdio.
    Mcp(McpArgs),
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Validate { plan } => commands::run_validate(&plan),
        Command::Project(args) => commands::project::run(&args),
        Command::Actions(args) => commands::actions::run(&args),
        Command::Compare(args) => commands::compare::run(&args),
        Command::Tui(args) => commands::tui::run(&args),
        Command::Optimize(command) => commands::optimize::run(&command),
        Command::MonteCarlo(args) => commands::markets::run_monte_carlo(&args),
        Command::Historical(args) => commands::markets::run_historical(&args),
        Command::ImportEarnings(args) => commands::import::run(&args),
        Command::Mcp(args) => commands::mcp::run(&args),
    }
}
