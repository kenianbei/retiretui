//! The planner in a terminal: the `tui` command's arguments and the
//! launch it makes of them, and the `theme` command over the themes such
//! a session can wear.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use bevy_app::{App, AppExit, ScheduleRunnerPlugin};
use clap::{Args, Subcommand};
use plurimus::core::CorePlugin;
use plurimus::crossterm::CrosstermPlugin;
use retiretui_client::environment::{config_dir, load_history, load_tables, state_dir};
use retiretui_client::store::DiskStore;

use crate::Launch;
use crate::theme::document::{TERMINAL, Variant, terminal_variant};
use crate::theme::library::{DIRECTORY, EMBEDDED, Listed, Origin, SUFFIX, Themes, UNREAD};

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

/// Arguments of the `theme` subcommand.
#[derive(Args, Debug)]
pub struct ThemeArgs {
    /// What to do with the themes.
    #[command(subcommand)]
    pub command: ThemeCommand,
}

/// What the `theme` subcommand does.
#[derive(Subcommand, Debug)]
pub enum ThemeCommand {
    /// List every theme the planner can wear: the built-in ones, and those
    /// in the `themes` directory beside `config.toml`.
    List,
    /// Print a theme's file, to start one of your own from.
    Dump {
        /// A theme, or a family of themes, as `[tui.theme] name` takes it.
        name: String,
    },
}

const LIST_HEADER: [&str; 4] = ["theme", "variant", "family", "from"];
const COLUMN_GAP: &str = "  ";
const TERMINALS_OWN: &str = "the terminal's own colours";
const NO_FILE: &str = "the terminal's own colours have no file; dump a theme to start from";

/// Lists the themes a terminal session can wear, or prints one's file.
///
/// # Errors
///
/// Where the theme to print is not one, or its file cannot be read.
pub fn run_theme(args: &ThemeArgs) -> anyhow::Result<()> {
    // With no home there is no directory, and a path that lists nothing
    // stands in for it.
    let directory = config_dir(DIRECTORY).unwrap_or_default();
    let (themes, complaints) = Themes::load(&DiskStore, &directory);
    match &args.command {
        ThemeCommand::List => {
            for complaint in &complaints {
                eprintln!("{complaint}");
            }
            print!("{}", listing(&themes));
        }
        ThemeCommand::Dump { name } => {
            let text = dumped(&themes, &directory, name, terminal_variant());
            print!("{}", text.map_err(anyhow::Error::msg)?);
        }
    }
    Ok(())
}

/// A row for the header and one for each theme, every column as wide as
/// its widest cell.
fn listing(themes: &Themes) -> String {
    let listed = themes.listed().map(|(slug, listed)| {
        let [variant, family, from] = listed.map_or(["", "", TERMINALS_OWN], about);
        [slug, variant, family, from]
    });
    let rows: Vec<[&str; 4]> = std::iter::once(LIST_HEADER).chain(listed).collect();
    let widths: [usize; 4] =
        std::array::from_fn(|column| rows.iter().map(|row| row[column].len()).max().unwrap_or(0));
    let lines = rows.iter().map(|row| {
        let cells = row
            .iter()
            .zip(widths)
            .map(|(cell, width)| format!("{cell:<width$}"));
        format!(
            "{}\n",
            cells.collect::<Vec<_>>().join(COLUMN_GAP).trim_end()
        )
    });
    lines.collect()
}

/// A theme's variant, its family, and where it comes from.
fn about(listed: &Listed) -> [&str; 3] {
    let Some(painted) = &listed.read else {
        return ["", "", UNREAD];
    };
    let from = match listed.origin {
        Origin::Embedded => "built in",
        Origin::User => "user",
        Origin::UserOverEmbedded => "user, over built in",
    };
    [painted.variant.name(), &painted.family, from]
}

/// The file of the theme `name` names as a session wanting `wanted` would
/// take it: an embedded document, or the user's in `directory`.
fn dumped(
    themes: &Themes,
    directory: &Path,
    name: &str,
    wanted: Variant,
) -> Result<String, String> {
    if name == TERMINAL {
        return Err(NO_FILE.to_owned());
    }
    let listed = themes.find(name, wanted)?;
    let embedded = EMBEDDED.iter().find(|(slug, _)| *slug == listed.slug);
    if let (Origin::Embedded, Some((_, text))) = (listed.origin, embedded) {
        return Ok((*text).to_owned());
    }
    let file = directory.join(format!("{}{SUFFIX}", listed.slug));
    std::fs::read_to_string(&file).map_err(|error| format!("{}: {error}", file.display()))
}
