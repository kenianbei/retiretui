//! What the user has set: read from the `[tui]` table of `config.toml`,
//! and written back a key at a time so the file stays the user's own.

use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Res, ResMut, Resource};
use serde::de::DeserializeOwned;
use toml_edit::{DocumentMut, Item, Table, Value};

use super::motion::Motion;
use super::theme::document::Choice;
use crate::journal;
use crate::session::Session;
use crate::store::Store;

const TUI_TABLE: &str = "tui";
/// The keys of `[tui]` that others write back.
pub const THEME_KEY: &str = "theme";
pub const MOTION_KEY: &str = "motion";
const DOCUMENT_KEY: &str = "document";
const KEYS_KEY: &str = "keys";

/// The file settings are read from and written back to.
#[derive(Debug)]
struct Kept {
    store: Arc<dyn Store>,
    path: PathBuf,
}

/// The settings in force, and the file they are kept in.
#[derive(Resource, Default, Debug)]
pub struct Settings {
    /// Where a changed key is written; nowhere for a session that keeps
    /// nothing.
    kept: Option<Kept>,
    pub theme: Choice,
    pub motion: Motion,
    /// The document last open, where a session reopens it.
    pub document: Option<PathBuf>,
    /// The keys each command named answers to, as the file states them:
    /// the command table judges each entry.
    pub keys: toml::Table,
}

/// `key` of `tui` read into `setting`. Where the value does not read,
/// `setting` keeps what it held and the answer is why.
fn take<T: DeserializeOwned>(tui: &mut toml::Table, key: &str, setting: &mut T) -> Option<String> {
    match tui.remove(key)?.try_into() {
        Ok(read) => {
            *setting = read;
            None
        }
        Err(error) => Some(format!("[{TUI_TABLE}] {key}: {}", error.message())),
    }
}

impl Settings {
    /// Settings that keep nothing and move nothing, which is what a frame
    /// is compared under.
    #[cfg(test)]
    pub fn still() -> Self {
        Self {
            motion: Motion::Off,
            ..Self::default()
        }
    }

    /// The settings the file at `path` in `store` holds, and a complaint
    /// for each that does not read and so keeps its default. An absent
    /// file is the defaults, as is one that is not TOML; it is left as it is.
    pub fn at(store: Arc<dyn Store>, path: PathBuf) -> (Self, Vec<String>) {
        let mut settings = Self::default();
        let unread = store
            .read(&path)
            .map_or_else(|_| Vec::new(), |text| settings.read(&text));
        let complaints = unread
            .into_iter()
            .map(|complaint| format!("{}: {complaint}", path.display()))
            .collect();
        settings.kept = Some(Kept { store, path });
        (settings, complaints)
    }

    /// Takes each setting `text` states under `[tui]`, and says which did
    /// not read.
    fn read(&mut self, text: &str) -> Vec<String> {
        let mut file = match text.parse::<toml::Table>() {
            Ok(file) => file,
            Err(error) => return vec![error.message().to_owned()],
        };
        let mut tui = match file.remove(TUI_TABLE) {
            Some(toml::Value::Table(tui)) => tui,
            Some(_) => return vec![format!("{TUI_TABLE} is not a table")],
            None => return Vec::new(),
        };
        let unread = [
            take(&mut tui, THEME_KEY, &mut self.theme),
            take(&mut tui, MOTION_KEY, &mut self.motion),
            take(&mut tui, DOCUMENT_KEY, &mut self.document),
            take(&mut tui, KEYS_KEY, &mut self.keys),
        ];
        unread.into_iter().flatten().collect()
    }

    /// The store the settings file is in and `name` beside that file;
    /// nothing for a session that keeps nothing.
    pub fn beside(&self, name: &str) -> Option<(&dyn Store, PathBuf)> {
        let Kept { store, path } = self.kept.as_ref()?;
        Some((store.as_ref(), path.parent()?.join(name)))
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
        let Some(Kept { store, path }) = &self.kept else {
            return Ok(());
        };
        let text = store.read(path).unwrap_or_default();
        let kept = with_key(&text, key, value.into())
            .map_err(|error| format!("{}: {error}", path.display()))?;
        write(store.as_ref(), path, &kept).map_err(|error| format!("{}: {error}", path.display()))
    }
}

/// Keeps the open document as the one to reopen, whenever another is.
pub fn remember_document(session: Res<Session>, mut settings: ResMut<Settings>) {
    if !session.is_changed() || session.plan_path == settings.document {
        return;
    }
    settings.document.clone_from(&session.plan_path);
    let Some(path) = session.plan_path.as_deref() else {
        return;
    };
    if let Err(error) = settings.keep(&[DOCUMENT_KEY], path.display().to_string()) {
        journal::warn(format!("not remembered: {error}"));
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

fn write(store: &dyn Store, path: &std::path::Path, text: &str) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        store.create_dir_all(directory)?;
    }
    store.write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::DiskStore;

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
        let mut read = Settings::default();
        assert_eq!(read.read(&kept), [""; 0]);
        assert_eq!(read.theme.name.as_deref(), Some("nord"));
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
        let (absent, complaints) = Settings::at(Arc::new(DiskStore), path.clone());
        assert!(complaints.is_empty() && absent.theme == Choice::default());
        absent.keep(&["theme", "name"], "nord").unwrap();
        absent.keep(&["motion"], "off").unwrap();
        let (read, complaints) = Settings::at(Arc::new(DiskStore), path.clone());
        assert_eq!(complaints, [""; 0]);
        assert_eq!(read.theme.name.as_deref(), Some("nord"));
        assert_eq!(read.motion, Motion::Off);

        std::fs::write(&path, "[tui\n").unwrap();
        let (broken, complaints) = Settings::at(Arc::new(DiskStore), path.clone());
        assert!(matches!(&complaints[..], [said] if said.contains("config.toml")));
        assert!(broken.keep(&["motion"], "full").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[tui\n");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_setting_that_does_not_read_spoils_only_itself() {
        let mut settings = Settings::default();
        let unread = settings.read(
            "[tui]\nmotion = \"slow\"\ndocument = \"/plans/ours.toml\"\n\
             [tui.theme]\nname = \"nord\"\n[tui.keys]\nsave = \"ctrl-w\"\n",
        );
        assert!(
            matches!(&unread[..], [said] if said.starts_with("[tui] motion: ")),
            "{unread:?}"
        );
        assert_eq!(settings.motion, Motion::default());
        assert_eq!(settings.theme.name.as_deref(), Some("nord"));
        assert_eq!(settings.document, Some(PathBuf::from("/plans/ours.toml")));
        assert_eq!(settings.keys["save"].as_str(), Some("ctrl-w"));
    }

    #[test]
    fn a_keys_table_of_any_shape_reads_and_one_that_is_no_table_is_said() {
        let mut settings = Settings::default();
        assert_eq!(settings.read("[tui.keys]\nsave = 3\nquit = []\n"), [""; 0]);
        assert_eq!(settings.keys.len(), 2);
        let mut settings = Settings::default();
        let unread = settings.read("[tui]\nkeys = \"ctrl-w\"\nmotion = \"off\"\n");
        assert!(matches!(&unread[..], [said] if said.starts_with("[tui] keys: ")));
        assert_eq!(settings.motion, Motion::Off);
        assert_eq!(Settings::default().read("tui = 3"), ["tui is not a table"]);
    }

    #[test]
    fn a_session_that_keeps_nothing_writes_nothing() {
        assert!(Settings::default().keep(&["motion"], "off").is_ok());
    }
}
