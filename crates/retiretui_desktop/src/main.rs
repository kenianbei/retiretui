//! Retirement planning application in a window.
#![windows_subsystem = "windows"]

use clap::Parser;
use retiretui_tui::desktop::DesktopArgs;

/// Open the interactive planner for a plan or scenario in a window.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(flatten)]
    args: DesktopArgs,
}

fn main() -> anyhow::Result<()> {
    retiretui_tui::desktop::run(&Cli::parse().args)
}
