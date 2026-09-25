//! What the user has set: read from the `[tui]` table of `config.toml`,
//! and written back a key at a time so the file stays the user's own.

use std::path::{Path, PathBuf};

use bevy_ecs::prelude::Resource;
use etcetera::BaseStrategy as _;
use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Table, Value};

use super::motion::Motion;
use super::theme::document::Choice;

const CONFIG_DIRECTORY: &str = "retiretui";
const CONFIG_FILE: &str = "config.toml";
const TUI_TABLE: &str = "tui";

#[derive(Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    tui: Settings,
}

/// The settings in force, and the file they are kept in.
#[derive(Resource, Deserialize, Default, Debug)]
#[serde(default)]
pub struct Settings {
    /// Where a changed key is written; nowhere for a session that keeps
    /// nothing.
    #[serde(skip)]
    path: Option<PathBuf>,
    pub theme: Choice,
    pub motion: Motion,
}

impl Settings {
    /// The user's settings from their config directory. A file that is
    /// absent is the defaults; one that does not read is the defaults and a
    /// complaint, and is left as it is.
    pub fn load() -> (Self, Option<String>) {
        let Ok(platform) = etcetera::choose_base_strategy() else {
            return (Self::default(), None);
        };
        Self::at(
            platform
                .config_dir()
                .join(CONFIG_DIRECTORY)
                .join(CONFIG_FILE),
        )
    }

    /// Settings that keep nothing and move nothing, which is what a frame
    /// is compared under.
    #[cfg(test)]
    pub fn still() -> Self {
        Self {
            motion: Motion::Off,
            ..Self::default()
        }
    }

    /// The settings the file at `path` holds.
    pub fn at(path: PathBuf) -> (Self, Option<String>) {
        let (mut settings, complaint) = match std::fs::read_to_string(&path) {
            Err(_) => (Self::default(), None),
            Ok(text) => match toml::from_str::<ConfigFile>(&text) {
                Ok(config) => (config.tui, None),
                Err(error) => (
                    Self::default(),
                    Some(format!("{}: {}", path.display(), error.message())),
                ),
            },
        };
        settings.path = Some(path);
        (settings, complaint)
    }

    /// Writes one key under `[tui]`, leaving every other key, comment, and
    /// blank line of the file as it was. `key` is the path beneath the
    /// table: `["theme", "name"]`.
    ///
    /// # Errors
    ///
    /// Where the file holds something that does not read as TOML - it is
    /// not overwritten - or cannot be written.
    pub fn keep(&self, key: &[&str], value: impl Into<Value>) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let kept = with_key(&text, key, value.into())
            .map_err(|error| format!("{}: {error}", path.display()))?;
        write(path, &kept).map_err(|error| format!("{}: {error}", path.display()))
    }
}

/// `text` with `key` under `[tui]` set to `value`.
fn with_key(text: &str, key: &[&str], value: Value) -> Result<String, String> {
    let mut document: DocumentMut = text.parse().map_err(|error| format!("{error}"))?;
    let Some((last, tables)) = key.split_last() else {
        return Ok(text.to_owned());
    };
    let mut table = document.as_table_mut();
    for name in std::iter::once(&TUI_TABLE).chain(tables) {
        let entry = table
            .entry(name)
            .or_insert_with(|| Item::Table(Table::new()));
        table = entry
            .as_table_mut()
            .ok_or_else(|| format!("{name} is not a table"))?;
    }
    let mut value = value;
    if let Some(replaced) = table.get(last).and_then(Item::as_value) {
        // The comment trailing a value on its line is the value's own.
        *value.decor_mut() = replaced.decor().clone();
    }
    table[last] = Item::Value(value);
    Ok(document.to_string())
}

fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    crate::commands::write_atomic(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAND_WRITTEN: &str = "\
# my settings
[tui]
motion = \"reduced\"   # the charts still move

[tui.theme]
name = \"gruvbox\"
accent = \"#fabd2f\"

[other]
kept = true
";

    #[test]
    fn a_kept_key_leaves_the_rest_of_the_file_as_it_was() {
        let kept = with_key(HAND_WRITTEN, &["theme", "name"], "nord".into()).unwrap();
        assert_eq!(kept, HAND_WRITTEN.replace("gruvbox", "nord"));
        let kept = with_key(HAND_WRITTEN, &["motion"], "off".into()).unwrap();
        assert!(kept.contains("motion = \"off\"   # the charts"), "{kept}");
    }

    #[test]
    fn a_kept_key_makes_the_tables_it_needs() {
        let kept = with_key("", &["theme", "name"], "nord".into()).unwrap();
        let read: ConfigFile = toml::from_str(&kept).unwrap();
        assert_eq!(read.tui.theme.name.as_deref(), Some("nord"));
    }

    #[test]
    fn a_file_that_does_not_read_is_not_written_over() {
        assert!(with_key("[tui", &["motion"], "off".into()).is_err());
        let in_the_way = with_key("tui = 3", &["motion"], "off".into());
        assert_eq!(in_the_way.unwrap_err(), "tui is not a table");
    }

    #[test]
    fn settings_are_read_from_the_file_and_kept_back_into_it() {
        let path = std::env::temp_dir().join(format!(
            "retiretui-settings-{}/config.toml",
            std::process::id()
        ));
        let (absent, complaint) = Settings::at(path.clone());
        assert!(complaint.is_none() && absent.theme == Choice::default());
        absent.keep(&["theme", "name"], "nord").unwrap();
        absent.keep(&["motion"], "off").unwrap();
        let (read, complaint) = Settings::at(path.clone());
        assert!(complaint.is_none());
        assert_eq!(read.theme.name.as_deref(), Some("nord"));
        assert_eq!(read.motion, Motion::Off);

        std::fs::write(&path, "[tui\n").unwrap();
        let (broken, complaint) = Settings::at(path.clone());
        assert!(complaint.is_some_and(|said| said.contains("config.toml")));
        assert!(broken.keep(&["motion"], "full").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[tui\n");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_session_that_keeps_nothing_writes_nothing() {
        assert!(Settings::default().keep(&["motion"], "off").is_ok());
    }
}
