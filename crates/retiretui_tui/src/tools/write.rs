//! Writing a tool's highlighted result into the workspace as a scenario
//! overlay, which is then compared with the document.

use std::path::{Path, PathBuf};

use bevy_ecs::change_detection::Mut;
use bevy_ecs::prelude::World;
use retiretui_client::files::OVERLAY_SAVE_FIRST;

use crate::command::Outcome;
use crate::compare::Compared;
use crate::documents::{Browsing, FilePick};
use crate::edit::Draft;
use crate::journal;
use crate::session::{self, Session};

pub const OVERLAY_OVER: &str = "Overwrite {}?";

/// Asks where to write, once the tool has found it has something to.
pub(super) fn open_picker(pick: FilePick, draft: &Draft, browsing: &mut Browsing) -> Outcome {
    if draft.is_dirty() {
        return Outcome::Refused(OVERLAY_SAVE_FIRST.to_owned());
    }
    browsing.open(pick);
    Outcome::Done
}

/// The document relative to the directory the overlay is written into.
pub(super) fn base_of(world: &World, overlay: &Path) -> Result<String, String> {
    let document = world.resource::<Session>().document()?;
    let store = world.resource::<Session>().store.as_ref();
    crate::files::overlay_base(store, overlay, document)
        .map_err(|error| format!("not written: {error}"))
}

/// Writes `text` at `path` and compares the file written; a reason it
/// could not be built is said instead.
pub(super) fn write(world: &mut World, path: PathBuf, text: Result<String, String>) {
    let text = match text {
        Ok(text) => text,
        Err(reason) => {
            journal::warn(reason);
            return;
        }
    };
    if let Err(error) = world.resource::<Session>().store.write(&path, &text) {
        journal::warn(format!("not written: {}: {error}", path.display()));
        return;
    }
    let name = session::file_name(&path).into_owned();
    let taken = world.resource_scope(|world, mut compared: Mut<Compared>| {
        compared.take_in(path, world.resource::<Session>())
    });
    match taken {
        Ok(()) => journal::say(format!("wrote {name}, compared")),
        Err(reason) => journal::warn(format!("wrote {name}, not compared: {reason}")),
    }
}
