//! The planner in a terminal: the `tui` command's arguments and the
//! launch it makes of them.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, bail};
use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use clap::Args;
use etcetera::BaseStrategy as _;
use plurimus::core::CorePlugin;
use plurimus::crossterm::CrosstermPlugin;
use retiretui_client::environment::{config_dir, load_history, load_tables};
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
const LOG_DIRECTORY: &str = "retiretui";
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
    app.add_plugins((
        ScheduleRunnerPlugin::run_loop(FRAME_INTERVAL),
        CorePlugin,
        CrosstermPlugin::default(),
    ));
    crate::build(&mut app, launch).map_err(anyhow::Error::msg)?;
    crate::install_log(&app, log_path())?;
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("tui exited with error code {code}"),
    }
}

/// Where the log file goes: the platform's state directory, else its cache.
fn log_path() -> anyhow::Result<PathBuf> {
    let platform = etcetera::choose_base_strategy().context("finding where state is filed")?;
    let state = platform.state_dir().unwrap_or_else(|| platform.cache_dir());
    Ok(state.join(LOG_DIRECTORY).join(LOG_FILE))
}
