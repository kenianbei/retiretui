//! The retirement planner's interface: a plurimus app over a projected plan, above
//! whatever backend draws it - a terminal, a browser page, or a window.

pub mod exchange;

use retiretui_client::{files, metric, present, store, table};

mod cells;
mod chart;
mod command;
mod compare;
mod confirm;
#[cfg(feature = "desktop")]
pub mod desktop;
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
mod scope;
mod session;
mod settings;
mod setup;
mod sidebar;
mod success;
mod tabbar;
mod tabulate;
#[cfg(feature = "terminal")]
pub mod terminal;
mod theme;
mod toast;
mod tools;
mod watch;

#[cfg(test)]
mod cursor_tests;
#[cfg(test)]
mod frames;
#[cfg(test)]
mod keys_tests;
#[cfg(test)]
mod messages_tests;
#[cfg(test)]
mod picker_tests;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod support;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

use bevy_app::{App, Startup, Update};
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
    /// The user's patterns file a payee named on that page is remembered
    /// in; none where there is no machine to keep one.
    pub patterns_file: Option<PathBuf>,
    /// Where the plan files are kept.
    pub store: Arc<dyn retiretui_client::store::Store>,
    /// The settings file in `store`; none for a session that keeps
    /// nothing.
    pub settings: Option<PathBuf>,
    /// Whether the screen it is drawn on is light, which the variant of a
    /// theme family follows.
    pub is_light: bool,
    /// Whether the document last open is opened again in place of `path`
    /// where it is still there, each document opened being remembered.
    pub reopens: bool,
    /// The directory no picker climbs above; none where any may be
    /// reached.
    pub floor: Option<PathBuf>,
    /// What hands files across the page's edge; none where there is no
    /// page.
    pub exchange: Option<Arc<dyn exchange::Exchange>>,
}

/// Adds the planner to `app`, drawn in a terminal's cells: the session
/// `launch` names, the user's settings, and every page.
///
/// # Errors
///
/// Where the document cannot be read or resolved.
pub fn build(app: &mut App, launch: Launch) -> Result<(), String> {
    build_shared(app, launch, cells::plugin)
}

/// Everything above whatever draws the tree, with `skin` adding what
/// does, in the place plurimus's layout had among the plugins.
fn build_shared(app: &mut App, launch: Launch, skin: fn(&mut App)) -> Result<(), String> {
    let (settings, complaints) = launch.settings.map_or_else(Default::default, |path| {
        settings::Settings::at(Arc::clone(&launch.store), path)
    });
    let path = match &settings.document {
        Some(document) if launch.reopens && launch.store.exists(document) => document.clone(),
        _ => launch.path,
    };
    let mut session = session::Session::at(Arc::clone(&launch.store), path, launch.tables);
    session.floor = launch.floor;
    let today = session::Today::now();
    let (projected, files) = watch::load_session(&session, today)?;
    app.insert_resource(session);
    app.insert_resource(settings);
    app.insert_resource(today);
    app.insert_resource(projected);
    app.insert_resource(watch::Watch::new(launch.store, files));
    app.insert_resource(tools::markets::MarketHistory(launch.history));
    app.insert_resource(tools::estimate::PatternsFile(launch.patterns_file));
    if let Some(exchange) = launch.exchange {
        app.insert_resource(exchange::Edge(exchange));
    }
    add_tui(app, skin);
    let variant = if launch.is_light {
        theme::document::Variant::Light
    } else {
        theme::document::Variant::Dark
    };
    app.insert_resource(theme::WantedVariant(variant));
    if launch.reopens {
        app.add_systems(Update, settings::remember_document);
    }
    // Said once the journal listens.
    app.add_systems(Startup, move || {
        complaints.iter().for_each(journal::warn);
    });
    Ok(())
}

/// Arguments every launcher on a machine takes: what to open, and where
/// else the tax tables are.
#[cfg(any(feature = "terminal", feature = "desktop"))]
#[derive(clap::Args, Debug)]
pub struct TuiArgs {
    /// Path to a plan or scenario TOML file, or to a directory to open
    /// one from; the working directory by default.
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

/// The launch `args` describe on this machine's disk and config.
///
/// # Errors
///
/// Where the tables or the history cannot be read.
#[cfg(any(feature = "terminal", feature = "desktop"))]
fn native_launch(args: &TuiArgs, is_light: bool) -> anyhow::Result<Launch> {
    use retiretui_client::environment::{config_dir, load_history, load_tables, patterns_path};
    Ok(Launch {
        path: args.path.clone(),
        tables: load_tables(&args.tax_dir)?,
        history: load_history(None)?,
        patterns_file: patterns_path(),
        store: Arc::new(retiretui_client::store::DiskStore),
        settings: config_dir(settings::CONFIG_FILE),
        is_light,
        reopens: false,
        floor: None,
        exchange: None,
    })
}

/// Whether the terminal says, through `COLORFGBG`, that its ground is
/// light.
#[must_use]
pub fn terminal_is_light() -> bool {
    theme::document::terminal_variant() == theme::document::Variant::Light
}

/// Installs the process's journal, which the shell toasts from, and no log
/// file.
///
/// # Errors
///
/// Where a subscriber is already installed.
pub fn install_journal(app: &App) -> anyhow::Result<()> {
    log::install_journal(app.world().resource::<journal::Inbox>())
}

/// Installs the process's log: the journal the shell toasts from, and the
/// log file at `file`, or the reason there is none, which is said.
///
/// # Errors
///
/// Where a subscriber is already installed.
pub fn install_log(app: &App, file: anyhow::Result<PathBuf>) -> anyhow::Result<()> {
    log::install(app.world().resource::<journal::Inbox>(), file)
}

/// Everything above the backend and the session resources, shared by the
/// real runs and the headless tests; `skin` is what draws the tree.
fn add_tui(app: &mut App, skin: fn(&mut App)) {
    app.init_resource::<session::Basis>();
    app.init_resource::<session::YearCursor>();
    app.add_plugins((WidgetsPlugin, FilePickerPlugin));
    skin(app);
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
        exchange::plugin,
    ));
}
