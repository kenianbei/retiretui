//! Editing: the draft plan every edit lands in, and the screens that edit
//! it.

mod build;
use retiretui_client::codec;
use retiretui_client::forms::{applies, cells, offers};
mod commands;
mod details;
mod domain;
mod draft;
mod editing;
mod field;
mod form;
mod group;
mod screen;
mod scroll;
mod search;
mod select;
mod sort;
mod table;
#[cfg(test)]
pub(crate) mod tests;
mod trigger;
mod widths;

use bevy_app::{App, Startup, Update};
use bevy_ecs::prelude::{Commands, Entity, IntoScheduleConfigs, Query, With, World};
use retiretui_engine::plan::Plan;

use super::hints::Hints;
use super::layout::{self, Body};
use super::nav::Page;
use retiretui_client::forms::DOMAINS;

#[cfg(test)]
pub use build::EditForm;
pub use build::{FormButton, SHORTEST_FORM_ROWS, help_fits};
pub use commands::{Importing, add, delete, import_earnings, record_statement};
pub use domain::{Ops, page_of};
pub use draft::{Draft, DraftEditor, redo, save, undo, write_draft};
pub use editing::{EditSession, Slot, open_item};
pub use retiretui_client::codec::from_table;
pub use retiretui_client::forms::changes::change_words;
pub use retiretui_client::forms::offers::{RefSource, ref_offers};
pub use retiretui_client::forms::{FieldSpec, ToolAnswers};
pub use retiretui_client::issues::issue_words;
pub use sort::sort;
#[cfg(test)]
pub use table::DomainTable;
pub use table::{Row, Turn, table_bundle};

use super::session::Projected;
use super::watch::Watch;

const SCREENS: [Ops; DOMAINS.len()] = {
    let mut screens = [Ops::domain(DOMAINS[0]); DOMAINS.len()];
    let mut at = 1;
    while at < DOMAINS.len() {
        screens[at] = Ops::domain(DOMAINS[at]);
        at += 1;
    }
    screens
};

/// The session settles on its item and fills the form before anything
/// reads what that wrote: a trigger's slots follow the kind filled in, and
/// a table's rows the cursor row the session asked for.
#[derive(bevy_ecs::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditSystems {
    Seed,
    Place,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Importing>();
    app.configure_sets(Update, (EditSystems::Seed, EditSystems::Place).chain());
    app.add_plugins((screen::plugin, form::plugin, scroll::plugin));
    app.add_systems(
        Startup,
        (seed_draft, spawn_screens.after(layout::spawn_frame)),
    );
}

/// The draft over the projected plan, read-only for a scenario.
pub fn seed_draft(world: &mut World) {
    let plan = world.resource::<Projected>().plan.clone();
    let is_scenario = world.resource::<Watch>().is_scenario();
    world.insert_resource(Draft::new(plan, is_scenario));
}

fn spawn_screens(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    for &ops in &SCREENS {
        screen::spawn_screen(&mut commands, body, ops);
    }
}

/// Whether `page` is a table of items, which is what add and delete act
/// on.
pub fn lists_items(page: Page) -> bool {
    SCREENS
        .iter()
        .any(|ops| ops.surface == Some(page) && ops.list.is_some())
}

/// How many items `page`'s domain holds; `None` for a page that lists
/// none.
pub fn item_count(page: Page, plan: &Plan) -> Option<usize> {
    let list = SCREENS.iter().find(|ops| ops.surface == Some(page))?.list?;
    Some((list.count)(plan))
}

const _: () = {
    let mut at = 0;
    while at < SCREENS.len() {
        assert!(
            build::help_fits(SCREENS[at]),
            "a field's help is missing or too long"
        );
        at += 1;
    }
};

/// Fills `pane` with a tool's answers to read, which ⏎ opens the form
/// over, as a single-item domain's page is.
pub fn spawn_details(commands: &mut Commands, pane: Entity, ops: Ops) {
    details::spawn_into(commands, pane, ops, None, TOOL_HINTS);
}

const TOOL_HINTS: Hints = Hints(&[("⏎", "edit")]);

/// The page that shows what an issue at `path` is about, and the item's
/// row where the path indexes one.
pub fn issue_target(path: &str) -> Option<(Page, Option<usize>)> {
    let (domain, index) = retiretui_client::issues::issue_place(path)?;
    Some((page_of(domain), index))
}
