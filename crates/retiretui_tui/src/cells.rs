//! The tree drawn in a terminal's cells: what the terminal launcher, the
//! browser page and the headless tests add over the shared planner, and a
//! window does not.

use super::{layout, motion, theme};
use bevy_app::{App, PostUpdate};
use bevy_ecs::prelude::IntoScheduleConfigs;
use bevy_ui::UiSystems;
use plurimus::bui::BuiPlugin;

pub fn plugin(app: &mut App) {
    app.add_plugins((BuiPlugin, theme::ground::plugin));
    motion::render::install(app);
    app.add_systems(PostUpdate, layout::sync_areas.after(UiSystems::PostLayout));
}
