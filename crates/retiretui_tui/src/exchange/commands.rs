//! The commands that hand files across the edge, which only a browser
//! page's command table holds.

use bevy_ecs::prelude::Res;

use super::Edge;
use crate::command::Outcome;
use crate::edit::Draft;
use crate::journal;
use crate::session::Session;

pub(super) const NO_EDGE: &str = "only the browser page hands files in and out";
pub(super) const SAVE_FIRST: &str = "save the draft first: a download is the file as saved";

pub(crate) fn upload(edge: Option<Res<Edge>>) -> Outcome {
    let Some(edge) = edge else {
        return Outcome::Refused(NO_EDGE.to_owned());
    };
    edge.0.ask();
    Outcome::Done
}

pub(crate) fn download(
    edge: Option<Res<Edge>>,
    session: Res<Session>,
    draft: Res<Draft>,
) -> Outcome {
    let Some(edge) = edge else {
        return Outcome::Refused(NO_EDGE.to_owned());
    };
    let document = match session.document() {
        Ok(document) => document,
        Err(refusal) => return Outcome::Refused(refusal),
    };
    if draft.is_dirty() {
        return Outcome::Refused(SAVE_FIRST.to_owned());
    }
    match session.store.read(document) {
        Ok(text) => {
            let name = session.file_name();
            edge.0.hand_out(&name, &text);
            journal::say(format!("downloaded {name}"));
            Outcome::Done
        }
        Err(error) => Outcome::Refused(format!("not downloaded: {error}")),
    }
}
