//! The retirement planner's interface: a plurimus app over a projected plan, above
//! whatever backend draws it - a terminal, or a browser page - and the
//! words every surface says a projection in.

pub mod actions;
pub mod files;
pub mod ladder;
pub mod metric;
pub mod resolve;
pub mod table;

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

use bevy_app::{App, Startup};
use bevy_ecs::prelude::{Res, Resource};
use plurimus::widgets::WidgetsPlugin;
use plurimus_filepicker::FilePickerPlugin;
use retiretui_engine::market::History;
use retiretui_engine::params::TaxTables;

/// What a session opens on, and the tables and history it runs over.
#[derive(Debug)]
pub struct Launch {
    /// A plan or scenario file, or a directory, which opens the shell
    /// empty.
    pub path: PathBuf,
    /// The tax tables every projection reads.
    pub tables: TaxTables,
    /// The historical market record the market tools draw from.
    pub history: History,
}

/// Adds the planner to `app`, above whatever backend draws it: the
/// session `launch` names, the user's settings, and every page.
///
/// # Errors
///
/// Where the document cannot be read or resolved.
pub fn build(app: &mut App, launch: Launch) -> Result<(), String> {
    let session = session::Session::at(launch.path, launch.tables);
    let today = session::Today::now();
    let (projected, files) = watch::load_session(&session, today)?;
    let (settings, complaint) = settings::Settings::load();
    app.insert_resource(session);
    app.insert_resource(settings);
    app.insert_resource(today);
    app.insert_resource(projected);
    app.insert_resource(watch::Watch::new(files));
    app.insert_resource(tools::markets::MarketHistory(launch.history));
    add_tui(app);
    if let Some(complaint) = complaint {
        app.insert_resource(SettingsComplaint(complaint));
        app.add_systems(Startup, say_settings_complaint);
    }
    Ok(())
}

/// Installs the process's log: the journal the shell toasts from, and the
/// log file.
///
/// # Errors
///
/// Where a subscriber is already installed.
pub fn install_log(app: &App) -> anyhow::Result<()> {
    log::install(app.world().resource::<journal::Inbox>())
}

/// Why the settings file was not read, said once the journal listens.
#[derive(Resource)]
struct SettingsComplaint(String);

fn say_settings_complaint(complaint: Res<SettingsComplaint>) {
    journal::warn(complaint.0.clone());
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
