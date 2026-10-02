//! The planner in a terminal: the `tui` command's arguments and the
//! launch it makes of them.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use clap::Args;
use plurimus::core::CorePlugin;
use plurimus::crossterm::CrosstermPlugin;
use retiretui_client::environment::{config_dir, load_history, load_tables, state_dir};
use retiretui_client::store::DiskStore;

use crate::Launch;

/// Arguments of the `tui` subcommand.
#[derive(Args, Debug)]
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
const CONFIG_FILE: &str = "config.toml";
const LOG_FILE: &str = "tui.log";

/// Runs the planner in the terminal until it is quit.
///
/// # Errors
///
/// Where the tables, the history or the log cannot be set up, the session
/// cannot open, or the app exits with an error.
pub fn run(args: &TuiArgs) -> anyhow::Result<()> {
    let launch = Launch {
        path: args.path.clone(),
        tables: load_tables(&args.tax_dir)?,
        history: load_history(None)?,
        store: Arc::new(DiskStore),
        settings: config_dir(CONFIG_FILE),
        is_light: crate::terminal_is_light(),
        reopens: false,
        floor: None,
        exchange: None,
    };
    let mut app = App::new();
    app.add_plugins((ScheduleRunnerPlugin::run_loop(FRAME_INTERVAL), CorePlugin));
    // The terminal is taken by adding its plugin, so a refused plan comes first.
    crate::build(&mut app, launch).map_err(anyhow::Error::msg)?;
    app.add_plugins(CrosstermPlugin::default().clipboard(true));
    crate::install_log(&app, state_dir().map(|dir| dir.join(LOG_FILE)))?;
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("tui exited with error code {code}"),
    }
}
