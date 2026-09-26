//! The documents a session opens: the switch from one to the next, and the
//! pickers that name them.

mod browse;
mod pickers;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use bevy_app::{App, PostStartup, Startup};
use bevy_ecs::prelude::{Commands, In, Res, ResMut, World};

use super::command::Outcome;
use super::compare::Compared;
use super::confirm::{Answer, Confirm};
use super::edit::{self, Draft};
use super::journal;
use super::nav::{self, ActivePage, Page};
use super::session::{Projected, Session, YearCursor};
use super::tools::{Claims, Ladders};
use super::watch::{self, Watch};

pub use browse::{Browsing, FilePick};
pub use pickers::{Pickers, compare_with, name_new_plan, open as open_picker, save_as};

pub fn plugin(app: &mut App) {
    app.add_plugins(browse::plugin);
    app.add_systems(Startup, pickers::register);
    app.add_systems(PostStartup, offer_a_document);
}

/// A shell launched on a directory offers the files it holds; one with
/// none of them is left on the new plan's form, which is the answer.
fn offer_a_document(session: Res<Session>, pickers: Res<Pickers>, mut browsing: ResMut<Browsing>) {
    if session.plan_path.is_none() && pickers::workspace_holds_a_plan(&session) {
        browsing.open(pickers.open);
    }
}

/// A document to open, and whether the compared set - and the Compare
/// page - stays with it: only when a compared plan opens in the
/// document's place.
#[derive(Clone)]
pub struct Opening {
    path: PathBuf,
    is_swap: bool,
}

impl Opening {
    /// The compared plan at `path` in the document's place, the document
    /// taking its place among the compared.
    pub fn swapping(path: PathBuf) -> Self {
        Self {
            path,
            is_swap: true,
        }
    }
}

impl From<PathBuf> for Opening {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            is_swap: false,
        }
    }
}

/// Opens the plan or scenario at `path` in place of the document the shell
/// holds. A draft the file does not hold is asked about first: Save,
/// Discard, or Cancel.
pub fn open(
    In(opening): In<Opening>,
    draft: Res<Draft>,
    session: Res<Session>,
    mut confirm: ResMut<Confirm>,
    mut commands: Commands,
) {
    if !draft.is_dirty() {
        commands.run_system_cached_with(switch, opening);
        return;
    }
    let discarding = opening.clone();
    confirm.ask_among(
        format!("Save the changes to {} first?", session.file_name()),
        vec![
            Answer::closing("Cancel"),
            Answer::running("Discard", move |commands| {
                commands.run_system_cached_with(switch, discarding);
            })
            .destructive(),
            Answer::running("Save", move |commands| {
                commands.run_system_cached_with(save_then_go, opening);
            })
            .primary(),
        ],
    );
}

fn save_then_go(In(wanted): In<Opening>, world: &mut World) {
    if let Ok(Outcome::Refused(reason)) = world.run_system_cached(edit::save) {
        journal::warn(reason);
        return;
    }
    switch(In(wanted), world);
}

/// A document that fails to load leaves the shell as it was.
pub fn switch(In(opening): In<Opening>, world: &mut World) {
    land(opening, world);
}

/// Makes `opening` the document, saying whether it did.
pub fn land(opening: Opening, world: &mut World) -> bool {
    let tables = &world.resource::<Session>().tables;
    let (loaded, files) = watch::load_projected(&opening.path, tables);
    let projected = match loaded {
        Ok(projected) => projected,
        Err(invalid) => {
            journal::warn(format!("not opened: {}", invalid.headline()));
            return false;
        }
    };
    let left = world
        .resource_mut::<Session>()
        .plan_path
        .replace(opening.path.clone());
    let held = opening
        .is_swap
        .then(|| world.remove_resource::<Compared>().unwrap_or_default());
    reset_session(world, projected, Watch::new(files));
    if let Some(held) = held {
        let swapped = held.swapped(&opening.path, left, &world.resource::<Session>().tables);
        world.insert_resource(swapped);
        nav::turn_in(world, Page::Compare);
    }
    journal::say(format!(
        "opened {}",
        world.resource::<Session>().file_name()
    ));
    true
}

/// Everything a document takes with it when it goes: the draft it is
/// seeded into, what is watched for it, and what ran over it.
fn reset_session(world: &mut World, projected: Projected, watch: Watch) {
    world.insert_resource(projected);
    world.insert_resource(watch);
    edit::seed_draft(world);
    world.insert_resource(YearCursor::default());
    world.insert_resource(ActivePage::default());
    world.insert_resource(Compared::default());
    world.insert_resource(Ladders::default());
    world.insert_resource(Claims::default());
}
