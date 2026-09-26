//! The theme's ground under everything: a cell a widget left at the
//! terminal's default is drawn in the theme's own foreground and ground.
//!
//! A pass over the composed frame rather than a style at every widget, so
//! that the rule holds for a widget built with no theme in reach, a stock
//! widget, and one not yet written alike. Under a theme that leaves its
//! grounds to the terminal it does nothing.

use bevy_app::App;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::{IntoScheduleConfigs, Res, ResMut, Resource};
use bevy_ecs::schedule::SystemSet;
use plurimus::core::ratatui_core::style::Color;
use plurimus::core::{
    CompositeSystems, FrameBuffer, MainWorld, TerminalRenderApp, TerminalRenderAppExt,
    TerminalRenderSystems,
};

use super::Theme;

pub fn plugin(app: &mut App) {
    app.add_extract_systems(extract);
    app.sub_app_mut(TerminalRenderApp).init_resource::<Ground>();
    app.add_terminal_systems(
        TerminalRenderSystems::Composite,
        paint_ground
            .in_set(CompositeSystems::PostProcess)
            .in_set(Grounded),
    );
}

/// The ground being painted. What eases between a cell's colours runs
/// after it, so it starts from the colours the cell is drawn in.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Grounded;

/// What the theme draws an unstyled cell in, as the render world holds it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
struct Ground {
    fg: Color,
    bg: Option<Color>,
}

fn extract(main_world: Res<MainWorld>, mut ground: ResMut<Ground>) {
    let theme = main_world.resource::<Theme>();
    ground.set_if_neq(Ground {
        fg: theme.fg,
        bg: theme.bg,
    });
}

fn paint_ground(ground: Res<Ground>, mut frame: ResMut<FrameBuffer>) {
    let paints_fg = ground.fg != Color::Reset;
    if !paints_fg && ground.bg.is_none() {
        return;
    }
    for cell in &mut frame.0.content {
        if paints_fg && cell.fg == Color::Reset {
            cell.fg = ground.fg;
        }
        if let Some(bg) = ground.bg
            && cell.bg == Color::Reset
        {
            cell.bg = bg;
        }
    }
}
