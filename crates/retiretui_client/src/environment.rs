//! What the user's machine supplies around a plan: the application's config
//! directory, the tax tables it adds to the embedded ones, and the market
//! history it may put in place of the embedded record.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use etcetera::BaseStrategy as _;
use retiretui_engine::market::History;
use retiretui_engine::params::TaxTables;

/// Where the historical record is read from in place of the embedded one.
const HISTORY_FILE: &str = "history.toml";

/// The application's own directory in each of the platform's.
const APPLICATION: &str = "retiretui";

/// The user's own directory `name` in the application's config directory.
#[must_use]
pub fn config_dir(name: &str) -> Option<PathBuf> {
    let strategy = etcetera::choose_base_strategy().ok()?;
    Some(strategy.config_dir().join(APPLICATION).join(name))
}

/// The application's directory in the platform's state directory, else in
/// its cache.
///
/// # Errors
///
/// Where the platform names no home to put either in.
pub fn state_dir() -> anyhow::Result<PathBuf> {
    let platform = etcetera::choose_base_strategy().context("finding where state is filed")?;
    let state = platform.state_dir().unwrap_or_else(|| platform.cache_dir());
    Ok(state.join(APPLICATION))
}

/// The embedded tax tables, overridden by the user's `tax` directory and then
/// by each of `extra_dirs` in order.
///
/// # Errors
///
/// Where a directory's tables cannot be read or parsed.
pub fn load_tables(extra_dirs: &[PathBuf]) -> anyhow::Result<TaxTables> {
    let mut tables = TaxTables::embedded();
    if let Some(dir) = config_dir("tax") {
        tables.add_dir(&dir)?;
    }
    for dir in extra_dirs {
        tables.add_dir(dir)?;
    }
    Ok(tables)
}

/// The market history at `explicit`, else the user's own where they keep
/// one, else the embedded record.
///
/// # Errors
///
/// Where the file cannot be read or parsed.
pub fn load_history(explicit: Option<&Path>) -> anyhow::Result<History> {
    let user = config_dir("market").map(|dir| dir.join(HISTORY_FILE));
    let Some(path) = explicit
        .map(Path::to_path_buf)
        .or_else(|| user.filter(|path| path.is_file()))
    else {
        return Ok(History::embedded().clone());
    };
    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    History::from_toml_str(&text).with_context(|| format!("in {}", path.display()))
}
