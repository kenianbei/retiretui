//! Retirement planning application for the terminal.

use clap::{CommandFactory as _, FromArgMatches as _, Parser, Subcommand};
use retiretui_mcp::McpArgs;
use retiretui_tui::terminal::{ThemeArgs, TuiArgs};

#[derive(Parser)]
#[command(version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(flatten)]
    Cli(retiretui_cli::Command),
    /// Open the interactive planner for a plan or scenario.
    Tui(TuiArgs),
    /// List the planner's themes, or print one to start your own from.
    Theme(ThemeArgs),
    /// Serve plans to AI agents over the Model Context Protocol on stdio.
    Mcp(McpArgs),
}

/// The order `--help` lists the commands in, which the crates they come from
/// do not decide.
const HELP_ORDER: [&str; 11] = [
    "validate",
    "project",
    "actions",
    "compare",
    "tui",
    "theme",
    "optimize",
    "monte-carlo",
    "historical",
    "import-earnings",
    "mcp",
];

fn command_line() -> clap::Command {
    HELP_ORDER
        .iter()
        .enumerate()
        .fold(Cli::command(), |command, (place, name)| {
            command.mut_subcommand(name, |sub| sub.display_order(place))
        })
}

fn main() -> anyhow::Result<()> {
    let cli =
        Cli::from_arg_matches(&command_line().get_matches()).unwrap_or_else(|error| error.exit());
    match cli.command {
        Command::Cli(command) => retiretui_cli::run(&command),
        Command::Tui(args) => retiretui_tui::terminal::run(&args),
        Command::Theme(args) => retiretui_tui::terminal::run_theme(&args),
        Command::Mcp(args) => retiretui_mcp::run(&args),
    }
}
