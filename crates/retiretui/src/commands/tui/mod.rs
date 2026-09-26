//! The interactive terminal dashboard: a plurimus app over a projected
//! plan.

mod chart;
mod command;
mod compare;
mod confirm;
mod documents;
mod drawer;
mod edit;
mod focus;
mod guard;
mod hints;
mod issues;
mod journal;
mod layout;
mod ledger;
mod log;
mod motion;
mod nav;
mod overlay;
mod overview;
mod pane;
mod picker;
mod present;
mod scope;
mod session;
mod settings;
mod setup;
mod sidebar;
mod success;
mod tabbar;
mod tabulate;
mod theme;
mod toast;
mod tools;
mod watch;

#[cfg(test)]
mod cursor_tests;
#[cfg(test)]
mod frames;
#[cfg(test)]
mod messages_tests;
#[cfg(test)]
mod picker_tests;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod support;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::bail;
use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use clap::Args;
use plurimus::core::CorePlugin;
use plurimus::crossterm::CrosstermPlugin;
use plurimus::widgets::WidgetsPlugin;
use plurimus_filepicker::FilePickerPlugin;

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
    let tables = super::load_tables(&args.tax_dir)?;
    let session = session::Session::at(args.path.clone(), tables);
    let today = session::Today::now();
    let (projected, files) = watch::load_session(&session, today).map_err(anyhow::Error::msg)?;
    let mut app = App::new();
    app.add_plugins((
        ScheduleRunnerPlugin::run_loop(FRAME_INTERVAL),
        CorePlugin,
        CrosstermPlugin::default(),
    ));
    let (settings, complaint) = settings::Settings::load();
    app.insert_resource(session);
    app.insert_resource(settings);
    app.insert_resource(today);
    app.insert_resource(projected);
    app.insert_resource(watch::Watch::new(files));
    app.insert_resource(tools::markets::MarketHistory(super::markets::load_history(
        None,
    )?));
    add_tui(&mut app);
    let inbox = app.world().resource::<journal::Inbox>().clone();
    log::install(&inbox)?;
    if let Some(complaint) = complaint {
        journal::warn(complaint);
    }
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => bail!("tui exited with error code {code}"),
    }
}

/// Everything above the terminal backend and the session resources, shared
/// by the real run and the headless tests.
fn add_tui(app: &mut App) {
    app.init_resource::<session::Basis>();
    app.init_resource::<session::YearCursor>();
    app.add_plugins((WidgetsPlugin, FilePickerPlugin));
    app.add_plugins((
        layout::plugin,
        guard::plugin,
        theme::plugin,
        pane::plugin,
        nav::plugin,
        focus::plugin,
        overlay::plugin,
        picker::plugin,
        confirm::plugin,
    ));
    app.add_plugins((
        tabbar::plugin,
        sidebar::plugin,
        setup::plugin,
        command::plugin,
        edit::plugin,
        hints::plugin,
        journal::plugin,
        toast::plugin,
        drawer::plugin,
        motion::plugin,
        overview::plugin,
        ledger::plugin,
        watch::plugin,
        documents::plugin,
    ));
    app.add_plugins((
        issues::plugin,
        chart::plugin,
        compare::plugin,
        success::plugin,
        tools::plugin,
    ));
}
