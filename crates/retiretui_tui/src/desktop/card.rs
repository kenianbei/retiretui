//! A card: a frame raised on a tinted surface without a header row, its
//! padding wider and its title, where it has one, on a row of its own;
//! and the tint, which a pill is raised on too.

use bevy_app::{App, Update};
use bevy_color::{Color, Mix};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Query, Res};
use bevy_ui::{BackgroundColor, Node, UiRect, Val};
use plurimus::core::ratatui_core::style::Style;

use super::chrome::FRAME_PADDING;
use crate::theme::{Repainted, Theme};

/// How far toward the ink the ground is mixed for a raised surface.
const RAISE: f32 = 0.05;
/// Rows a card keeps over and under what it holds; a titled card keeps a
/// row more over it, its title on it.
const CARD_ROWS: f32 = 1.0;
const TITLE_ROWS: f32 = 1.0;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, paint_raised.in_set(Repainted));
}

/// A frame drawn as a card: no header row, a raised fill, a dimmed title
/// and wider padding. Spawned beside the frame, so it is there as the
/// frame is dressed.
#[derive(Component, Debug)]
pub(super) struct Card;

/// A node filled a shade off the ground, whatever the theme.
#[derive(Component, Debug)]
pub(super) struct Raised;

/// The ground a shade toward the ink.
pub(super) fn raised(theme: &Theme) -> Color {
    theme.ground().mix(&theme.ink(Style::new()), RAISE)
}

fn paint_raised(
    theme: Res<Theme>,
    raised_nodes: Query<(Entity, Ref<Raised>)>,
    mut commands: Commands,
) {
    for (node, marker) in &raised_nodes {
        if theme.is_changed() || marker.is_added() {
            commands
                .entity(node)
                .insert(BackgroundColor(raised(&theme)));
        }
    }
}

/// A card is raised, keeps its padding clear around what it holds, and its
/// title, where it has one, on a row of its own over that.
pub(super) fn pad_the_card(
    commands: &mut Commands,
    node: &mut Node,
    line: Entity,
    is_untitled: bool,
) {
    commands.entity(line).insert(Raised);
    let head = if is_untitled {
        CARD_ROWS
    } else {
        CARD_ROWS + TITLE_ROWS
    };
    node.padding =
        UiRect::axes(Val::Em(FRAME_PADDING.0), Val::Rem(CARD_ROWS)).with_top(Val::Rem(head));
}
