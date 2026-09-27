//! The log file: the session's `tracing` events, written where the
//! platform files state.

use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer as _, fmt, registry};

use super::journal::{self, Inbox};

const LEVEL_VARIABLE: &str = "RETIRETUI_LOG";
const DEFAULT_LEVEL: &str = "warn";
const USER_LEVEL: &str = "trace";

/// Installs the process's subscriber: the journal layer feeding `inbox`,
/// and the log file, started over each session so it always holds exactly
/// the last one. A log file that cannot be opened is said to the user and
/// done without: it is a record of the session, not a condition of it.
///
/// # Errors
///
/// Where a subscriber is already installed.
pub fn install(inbox: &Inbox, path: anyhow::Result<PathBuf>) -> anyhow::Result<()> {
    let (file, failure) = match path.and_then(|path| open_log(&path)) {
        Ok(file) => (Some(file), None),
        Err(failure) => (None, Some(failure)),
    };
    let filed = file.map(|file| {
        fmt::layer()
            .with_writer(file)
            .with_ansi(false)
            .with_filter(level_filter())
    });
    registry()
        .with(filed)
        .with(journal::layer(inbox.clone()))
        .try_init()
        .map_err(|failure| anyhow::anyhow!("installing the log: {failure}"))?;
    if let Some(failure) = failure {
        journal::warn(format!("no log file this session: {failure:#}"));
    }
    Ok(())
}

fn open_log(path: &Path) -> anyhow::Result<File> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("creating {}", directory.display()))?;
    }
    File::create(path).with_context(|| format!("opening {}", path.display()))
}

/// Installs the process's subscriber with the journal layer alone, for a
/// session that keeps no log file.
///
/// # Errors
///
/// Where a subscriber is already installed.
pub fn install_journal(inbox: &Inbox) -> anyhow::Result<()> {
    registry()
        .with(journal::layer(inbox.clone()))
        .try_init()
        .map_err(|failure| anyhow::anyhow!("installing the log: {failure}"))
}

/// What reaches the file: warnings and worse, or what `RETIRETUI_LOG`
/// asks for, and always everything said to the user.
fn level_filter() -> EnvFilter {
    let filter =
        EnvFilter::try_from_env(LEVEL_VARIABLE).unwrap_or_else(|_| EnvFilter::new(DEFAULT_LEVEL));
    match format!("{}={USER_LEVEL}", journal::USER_TARGET).parse() {
        Ok(directive) => filter.add_directive(directive),
        Err(_) => filter,
    }
}
