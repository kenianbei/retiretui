//! Files handed across a browser page's edge: taken in from the visitor's
//! machine into the workspace, and handed back out of it. Only a page has
//! an edge to hand them across.

use std::collections::VecDeque;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use bevy_app::{App, Update};
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Local, Res, ResMut, Resource, World};
use bevy_ecs::schedule::common_conditions::resource_exists;

use crate::confirm::Confirm;
use crate::journal;
use crate::session::{self, Session};

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) mod commands;
#[cfg(test)]
mod tests;

const REPLACE: &str = "Replace {} with the one uploaded?";

/// What carries files across the page's edge.
pub trait Exchange: fmt::Debug + Send + Sync + 'static {
    /// Asks the visitor for files to take in; they come back through
    /// [`Exchange::arrived`] once the visitor has chosen them.
    fn ask(&self);

    /// Hands `text` to the visitor as a file named `name`.
    fn hand_out(&self, name: &str, text: &str);

    /// The files that arrived since it was last asked, each by its name
    /// and text.
    fn arrived(&self) -> Vec<(String, String)>;
}

/// The session's edge, where it has one.
#[derive(Resource)]
pub(crate) struct Edge(pub(crate) Arc<dyn Exchange>);

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, land_arrivals.run_if(resource_exists::<Edge>));
}

/// Lands what arrived in the workspace, one file a question: one whose
/// name is taken asks before it replaces it.
fn land_arrivals(
    edge: Res<Edge>,
    session: Res<Session>,
    mut arriving: Local<VecDeque<(String, String)>>,
    mut confirm: ResMut<Confirm>,
) {
    arriving.extend(edge.0.arrived());
    if confirm.is_open() {
        return;
    }
    let Some((name, text)) = arriving.pop_front() else {
        return;
    };
    let path = session.workspace().join(&name);
    if !session.store.exists(&path) {
        land(&session, &path, &text);
        return;
    }
    let question = REPLACE.replacen("{}", &name, 1);
    confirm.ask(question, "Replace", move |commands: &mut Commands| {
        commands.queue(move |world: &mut World| land(world.resource::<Session>(), &path, &text));
    });
}

fn land(session: &Session, path: &Path, text: &str) {
    let name = session::file_name(path);
    match session.store.write(path, text) {
        Ok(()) => journal::say(format!("uploaded {name}")),
        Err(error) => journal::warn(format!("{name} not uploaded: {error}")),
    }
}
