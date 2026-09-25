//! The Claim Options pane: every set of claim ages the search tried, as
//! the shared options table lists them.

use retiretui_engine::optimize::ClaimSearch;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Commands;

use super::super::options::spawn_table;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::nav::FocusStop;
use crate::commands::tui::pane::Pane;

pub const TITLE: &str = "Claim Options";

/// The pane, sharing `row` with the people.
pub fn spawn_pane(commands: &mut Commands, row: Entity) {
    let framed = Pane::new(TITLE).sharing(1.0).spawn(commands, row);
    let hints = Hints(&[("↑↓", "option"), ("⏎", "take")]);
    let table = spawn_table::<ClaimSearch>(commands, framed, "take-claims", hints);
    commands.entity(table).insert(FocusStop);
}
