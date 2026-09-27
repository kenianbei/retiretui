//! The page's `localStorage`, as the planner's store keeps its files in it.

use std::io;

use retiretui_tui::store::Backend;
use web_sys::Storage;

/// The origin's `localStorage`, looked up on each call: a handle to it
/// cannot be held across threads, and the store must be.
#[derive(Debug)]
pub struct LocalStorage;

fn storage() -> io::Result<Storage> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or_else(|| io::Error::other("this browser keeps no local storage for the page"))
}

impl Backend for LocalStorage {
    fn get(&self, key: &str) -> Option<String> {
        storage().ok()?.get_item(key).ok().flatten()
    }

    fn set(&self, key: &str, value: &str) -> io::Result<()> {
        storage()?
            .set_item(key, value)
            .map_err(|error| io::Error::other(format!("the browser refused the write: {error:?}")))
    }

    fn keys(&self) -> Vec<String> {
        let Ok(storage) = storage() else {
            return Vec::new();
        };
        let count = storage.length().unwrap_or_default();
        (0..count)
            .filter_map(|at| storage.key(at).ok().flatten())
            .collect()
    }
}
