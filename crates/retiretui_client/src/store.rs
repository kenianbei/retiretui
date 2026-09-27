//! Where plan files are kept: the disk, or keys in a browser's storage.
//! Every read, write, listing and change stamp the planner makes goes
//! through one [`Store`].

use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

/// One entry of a directory: a file or a directory beneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its name within the directory.
    pub name: String,
    /// Whether it is a directory.
    pub is_dir: bool,
}

impl Entry {
    fn file(name: String) -> Self {
        Self {
            name,
            is_dir: false,
        }
    }

    fn directory(name: String) -> Self {
        Self { name, is_dir: true }
    }
}

/// What changes whenever a file does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stamp {
    /// Its modification time on disk.
    Modified(SystemTime),
    /// How many times it was written under its key.
    Writes(u64),
}

/// Files by path, as the planner reads and writes them.
pub trait Store: fmt::Debug + Send + Sync + 'static {
    /// The file's text.
    ///
    /// # Errors
    ///
    /// Where there is no such file, or it cannot be read.
    fn read(&self, path: &Path) -> io::Result<String>;

    /// Replaces the file's text whole, or leaves it as it was.
    ///
    /// # Errors
    ///
    /// Where it cannot be written, or its directory does not exist.
    fn write(&self, path: &Path, text: &str) -> io::Result<()>;

    /// Makes `directory` and every directory above it that is missing.
    ///
    /// # Errors
    ///
    /// Where one cannot be made.
    fn create_dir_all(&self, directory: &Path) -> io::Result<()>;

    /// The entries of `directory`, in no particular order.
    ///
    /// # Errors
    ///
    /// Where there is no such directory, or it cannot be read.
    fn list(&self, directory: &Path) -> io::Result<Vec<Entry>>;

    /// Whether `path` is a directory.
    fn is_dir(&self, path: &Path) -> bool;

    /// Whether anything is at `path`.
    fn exists(&self, path: &Path) -> bool;

    /// The file's change stamp; none where there is no file.
    fn stamp(&self, path: &Path) -> Option<Stamp>;

    /// The one path the file at `path` is known by, whatever path named
    /// it, as scenario chains are followed and compared by.
    ///
    /// # Errors
    ///
    /// Where nothing is at `path`.
    fn canonical(&self, path: &Path) -> io::Result<PathBuf>;
}

/// The files on the machine's own disk.
#[derive(Debug, Default, Clone, Copy)]
pub struct DiskStore;

impl Store for DiskStore {
    fn read(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        crate::files::write_atomic(path, text)
    }

    fn create_dir_all(&self, directory: &Path) -> io::Result<()> {
        std::fs::create_dir_all(directory)
    }

    // `file_type` is free on most filesystems; only a symlink costs a stat,
    // so a linked directory can be entered.
    fn list(&self, directory: &Path) -> io::Result<Vec<Entry>> {
        let entries = std::fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.file_type().is_ok_and(|kind| {
                    kind.is_dir() || (kind.is_symlink() && entry.path().is_dir())
                });
                if is_dir {
                    Entry::directory(name)
                } else {
                    Entry::file(name)
                }
            })
            .collect();
        Ok(entries)
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn stamp(&self, path: &Path) -> Option<Stamp> {
        let modified = std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()?;
        Some(Stamp::Modified(modified))
    }

    fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
        path.canonicalize()
    }
}

/// Text under string keys, as a browser's `localStorage` keeps it.
pub trait Backend: fmt::Debug + Send + Sync + 'static {
    /// The text under `key`.
    fn get(&self, key: &str) -> Option<String>;

    /// Sets the text under `key`.
    ///
    /// # Errors
    ///
    /// Where the backend is full or refuses the write.
    fn set(&self, key: &str, value: &str) -> io::Result<()>;

    /// Every key held.
    fn keys(&self) -> Vec<String>;
}

const FILE_KEY: &str = "retiretui:file:";
const STAMP_KEY: &str = "retiretui:stamp:";
const DIRECTORY_KEY: &str = "retiretui:dir:";

/// Files kept as keys of a [`Backend`], under absolute paths from `/`: a
/// file's text under its path, a count of its writes beside it, and a
/// marker for each directory made, so an empty one still lists.
#[derive(Debug)]
pub struct KeyStore<B> {
    backend: B,
}

impl<B: Backend> KeyStore<B> {
    /// The files `backend` holds.
    pub const fn new(backend: B) -> Self {
        Self { backend }
    }

    fn file(&self, path: &Path) -> Option<String> {
        self.backend.get(&key(FILE_KEY, path))
    }

    /// Every path held, files and made directories alike, each beside
    /// whether it is a file.
    fn paths(&self) -> impl Iterator<Item = (PathBuf, bool)> {
        self.backend.keys().into_iter().filter_map(|key| {
            let (path, is_file) = match key.strip_prefix(FILE_KEY) {
                Some(file) => (file, true),
                None => (key.strip_prefix(DIRECTORY_KEY)?, false),
            };
            Some((PathBuf::from(path), is_file))
        })
    }

    fn writes(&self, path: &Path) -> Option<u64> {
        self.backend.get(&key(STAMP_KEY, path))?.parse().ok()
    }
}

impl<B: Backend> Store for KeyStore<B> {
    fn read(&self, path: &Path) -> io::Result<String> {
        self.file(path).ok_or_else(|| not_found(path))
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        let directory = normal(path).parent().map(Path::to_path_buf);
        if !directory.is_some_and(|directory| self.is_dir(&directory)) {
            return Err(not_found(path));
        }
        let writes = self.writes(path).unwrap_or_default();
        self.backend.set(&key(FILE_KEY, path), text)?;
        self.backend
            .set(&key(STAMP_KEY, path), &(writes + 1).to_string())
    }

    fn create_dir_all(&self, directory: &Path) -> io::Result<()> {
        let directory = normal(directory);
        for made in directory.ancestors().filter(|made| *made != Path::new("/")) {
            if self.file(made).is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} is a file", made.display()),
                ));
            }
            self.backend.set(&key(DIRECTORY_KEY, made), "")?;
        }
        Ok(())
    }

    fn list(&self, directory: &Path) -> io::Result<Vec<Entry>> {
        let directory = normal(directory);
        let mut is_held = directory == Path::new("/");
        let mut entries: Vec<Entry> = Vec::new();
        for (path, is_file) in self.paths() {
            let Ok(below) = path.strip_prefix(&directory) else {
                continue;
            };
            is_held = true;
            let mut parts = below.components();
            let Some(Component::Normal(name)) = parts.next() else {
                continue;
            };
            let name = name.to_string_lossy().into_owned();
            let is_dir = parts.next().is_some() || !is_file;
            if entries.iter().any(|entry| entry.name == name) {
                continue;
            }
            entries.push(if is_dir {
                Entry::directory(name)
            } else {
                Entry::file(name)
            });
        }
        if is_held {
            Ok(entries)
        } else {
            Err(not_found(&directory))
        }
    }

    fn is_dir(&self, path: &Path) -> bool {
        let path = normal(path);
        path == Path::new("/")
            || self
                .paths()
                .any(|(held, _)| held != path && held.starts_with(&path))
            || self.backend.get(&key(DIRECTORY_KEY, &path)).is_some()
    }

    fn exists(&self, path: &Path) -> bool {
        self.writes(path).is_some() || self.is_dir(path)
    }

    // A file's count is set beside every write of it and nothing is ever
    // deleted, so the count is there exactly when the file is.
    fn stamp(&self, path: &Path) -> Option<Stamp> {
        self.writes(path).map(Stamp::Writes)
    }

    fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
        if self.exists(path) {
            Ok(normal(path))
        } else {
            Err(not_found(path))
        }
    }
}

fn key(kind: &str, path: &Path) -> String {
    format!("{kind}{}", normal(path).display())
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("no such file: {}", path.display()),
    )
}

/// `path` from the root with `.` dropped and `..` folded, a `..` at the
/// root staying there.
#[must_use]
pub fn normal(path: &Path) -> PathBuf {
    let mut normal = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::Normal(part) => normal.push(part),
            Component::ParentDir => {
                normal.pop();
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        }
    }
    normal
}

/// A [`Backend`] in memory, for tests of what runs over a [`KeyStore`].
#[cfg(any(test, feature = "testing"))]
pub mod memory {
    use std::collections::BTreeMap;
    use std::io;
    use std::sync::{Arc, Mutex};

    use super::Backend;

    /// A backend in memory, shared between the stores made over it as a
    /// browser's storage is between its tabs.
    #[derive(Debug, Default, Clone)]
    pub struct Memory(Arc<Mutex<BTreeMap<String, String>>>);

    impl Backend for Memory {
        fn get(&self, key: &str) -> Option<String> {
            self.0.lock().unwrap().get(key).cloned()
        }

        fn set(&self, key: &str, value: &str) -> io::Result<()> {
            self.0
                .lock()
                .unwrap()
                .insert(key.to_owned(), value.to_owned());
            Ok(())
        }

        fn keys(&self) -> Vec<String> {
            self.0.lock().unwrap().keys().cloned().collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::memory::Memory;
    use super::*;

    fn workspace() -> KeyStore<Memory> {
        let store = KeyStore::new(Memory::default());
        store.create_dir_all(Path::new("/workspace")).unwrap();
        store
    }

    fn names(store: &impl Store, directory: &str) -> Vec<(String, bool)> {
        let mut names: Vec<_> = store
            .list(Path::new(directory))
            .unwrap()
            .into_iter()
            .map(|entry| (entry.name, entry.is_dir))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_written_file_reads_back_and_its_stamp_moves_only_on_a_write() {
        let store = workspace();
        let path = Path::new("/workspace/plan.toml");
        assert!(store.stamp(path).is_none());
        store.write(path, "one").unwrap();
        let first = store.stamp(path).unwrap();
        assert_eq!(store.read(path).unwrap(), "one");
        assert_eq!(store.stamp(path), Some(first));
        store.write(path, "two").unwrap();
        assert_ne!(store.stamp(path), Some(first));
        assert_eq!(
            store
                .read(Path::new("/workspace/./x/../plan.toml"))
                .unwrap(),
            "two"
        );
    }

    #[test]
    fn a_directory_lists_its_files_and_the_directories_beneath_it() {
        let store = workspace();
        store.create_dir_all(Path::new("/workspace/old")).unwrap();
        store.write(Path::new("/workspace/a.toml"), "").unwrap();
        store.write(Path::new("/workspace/old/b.toml"), "").unwrap();
        assert_eq!(
            names(&store, "/workspace"),
            [("a.toml".to_owned(), false), ("old".to_owned(), true)]
        );
        assert_eq!(names(&store, "/"), [("workspace".to_owned(), true)]);
        assert!(store.list(Path::new("/missing")).is_err());
    }

    #[test]
    fn a_file_is_written_only_into_a_directory_there_is() {
        let store = workspace();
        assert!(store.write(Path::new("/nowhere/plan.toml"), "").is_err());
        assert!(store.stamp(Path::new("/nowhere/plan.toml")).is_none());
        store.write(Path::new("/workspace/plan.toml"), "").unwrap();
        assert!(
            store
                .create_dir_all(Path::new("/workspace/plan.toml"))
                .is_err()
        );
    }

    #[test]
    fn a_path_is_canonical_only_where_something_is() {
        let store = workspace();
        store.write(Path::new("/workspace/plan.toml"), "").unwrap();
        assert_eq!(
            store
                .canonical(Path::new("/workspace/../workspace/plan.toml"))
                .unwrap(),
            PathBuf::from("/workspace/plan.toml")
        );
        assert!(store.canonical(Path::new("/workspace/gone.toml")).is_err());
        assert!(store.exists(Path::new("/workspace")));
    }

    #[test]
    fn another_store_over_the_same_backend_sees_a_write() {
        let backend = Memory::default();
        let mine = KeyStore::new(backend.clone());
        mine.create_dir_all(Path::new("/workspace")).unwrap();
        let theirs = KeyStore::new(backend);
        let path = Path::new("/workspace/plan.toml");
        mine.write(path, "one").unwrap();
        let seen = theirs.stamp(path);
        mine.write(path, "two").unwrap();
        assert_ne!(theirs.stamp(path), seen);
    }
}
