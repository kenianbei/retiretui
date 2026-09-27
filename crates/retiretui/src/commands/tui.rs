//! The `tui` subcommand: the planner in the terminal.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::bail;
use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use clap::Args;
use plurimus::core::CorePlugin;
use plurimus::crossterm::CrosstermPlugin;
use retiretui_tui::Launch;

/// Arguments of the `tui` subcommand.
#[derive(Args)]
pub struct TuiArgs {
    /// Path to a plan or scenario TOML file, or to a directory to open
    /// one from; the working directory by default.
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

const FRAME_INTERVAL: Duration = Duration::from_millis(16);

pub fn run(args: &TuiArgs) -> anyhow::Result<()> {
    let launch = Launch {
        path: args.path.clone(),
        tables: super::load_tables(&args.tax_dir)?,
        history: super::markets::load_history(None)?,
    };
    let mut app = App::new();
    app.add_plugins((
        ScheduleRunnerPlugin::run_loop(FRAME_INTERVAL),
        CorePlugin,
        CrosstermPlugin::default(),
    ));
    retiretui_tui::build(&mut app, launch).map_err(anyhow::Error::msg)?;
    retiretui_tui::install_log(&app)?;
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("tui exited with error code {code}"),
    }
}
