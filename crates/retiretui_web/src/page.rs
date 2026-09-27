//! The page's entry: the planner launched over the browser's storage, drawn
//! on a canvas.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy_app::App;
use plurimus::core::CorePlugin;
use plurimus::web::{GridFit, WebPlugin};
use retiretui_client::store::{KeyStore, Store};
use retiretui_engine::market::History;
use retiretui_engine::params::TaxTables;
use retiretui_tui::Launch;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::edge::PageEdge;
use crate::storage::LocalStorage;

/// Where the visitor's plans are kept, and no picker climbs above.
const WORKSPACE: &str = "/workspace";
const SETTINGS: &str = "/config/config.toml";
/// The family `index.html` loads.
const FONT: &str = "Inconsolata";
/// The grid the first frame is fitted to: the shell's floor.
const COLUMNS: u16 = 128;
const ROWS: u16 = 32;
/// The element the canvas is drawn in.
const PARENT: &str = "planner";
const LIGHT_SCHEME: &str = "(prefers-color-scheme: light)";

#[wasm_bindgen(start)]
pub fn start() {
    if let Err(failure) = run() {
        web_sys::console::error_1(&failure.into());
    }
}

fn run() -> Result<(), String> {
    let store: Arc<dyn Store> = Arc::new(KeyStore::new(LocalStorage));
    store
        .create_dir_all(Path::new(WORKSPACE))
        .map_err(|error| format!("no workspace: {error}"))?;
    let launch = Launch {
        path: PathBuf::from(WORKSPACE),
        tables: TaxTables::embedded(),
        history: History::embedded().clone(),
        store,
        settings: Some(PathBuf::from(SETTINGS)),
        is_light: prefers_light(),
        reopens: true,
        floor: Some(PathBuf::from(WORKSPACE)),
        exchange: Some(Arc::new(PageEdge::default())),
    };
    let mut app = App::new();
    app.add_plugins((
        CorePlugin,
        WebPlugin::new()
            .font(FONT)
            .fit(GridFit::Cells(COLUMNS, ROWS))
            .parent(PARENT),
    ));
    retiretui_tui::build(&mut app, launch)?;
    retiretui_tui::install_journal(&app).map_err(|error| error.to_string())?;
    app.run();
    Ok(())
}

/// Whether the visitor's browser asks for a light page.
fn prefers_light() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media(LIGHT_SCHEME).ok().flatten())
        .is_some_and(|query| query.matches())
}
