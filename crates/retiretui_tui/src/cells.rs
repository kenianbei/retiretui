//! The tree drawn in a terminal's cells: what the terminal launcher, the
//! browser page and the headless tests add over the shared planner, and a
//! window does not.

use bevy_app::{App, PostUpdate};
use bevy_ecs::prelude::{Add, Commands, IntoScheduleConfigs, On};
use bevy_ui::UiSystems;
use plurimus::bui::BuiPlugin;
use plurimus::core::UiWidget;
use plurimus::widgets::EditableText;

use super::edit::TextField;
use super::{layout, motion, theme};

pub fn plugin(app: &mut App) {
    app.add_plugins((BuiPlugin, theme::ground::plugin));
    motion::render::install(app);
    app.add_systems(PostUpdate, layout::sync_areas.after(UiSystems::PostLayout));
    app.add_observer(dress_text_fields);
}

/// A form's text field is edited and drawn by plurimus's field over the
/// state the form keeps.
fn dress_text_fields(added: On<Add<TextField>>, mut commands: Commands) {
    commands
        .entity(added.entity)
        .insert((EditableText, UiWidget::default()));
}
