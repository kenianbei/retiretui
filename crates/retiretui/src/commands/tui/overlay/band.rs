//! The band an overlay is painted in, by how deep it stands.

use bevy_app::{App, HierarchyPropagatePlugin, PostUpdate, PropagateSet};
use bevy_ecs::prelude::{
    Added, Changed, Commands, Component, Entity, Has, IntoScheduleConfigs, Or, Query, With,
};
use plurimus::core::{UiOrder, UiWidget};
use plurimus::widgets::MenuItem;

use crate::commands::tui::pane::Framed;

/// The orders one overlay takes: the frame it stands in, what it holds
/// within it, and the rows of a menu opened from one of those.
const BAND_STEP: i32 = 3;

/// A menu's rows are drawn over the popup that clears the ground for them.
const MENU_ROW_ORDER: UiOrder = UiOrder(UiOrder::OVERLAY.0 + 1);

/// What an overlay's frame is drawn beneath: its own widgets.
const FRAME_ORDER: UiOrder = UiOrder(UiOrder::OVERLAY.0 - 1);

/// How many overlays stand under the one this marks. Widgets are painted
/// in one flat order, so two overlays sharing a band would draw the lower
/// one's text through the upper one's frame.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Band(i32);

impl Band {
    #[must_use]
    pub fn at(beneath: usize) -> Self {
        Self(i32::try_from(beneath).unwrap_or(i32::MAX))
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins(HierarchyPropagatePlugin::<Band>::new(PostUpdate));
    app.add_systems(
        PostUpdate,
        give_order.after(PropagateSet::<Band>::default()),
    );
}

/// Gives everything an overlay draws its order: the frame beneath what it
/// holds, and both above whatever overlay stands under this one.
fn give_order(
    painted: Query<
        (Entity, &Band, Has<Framed>, Has<MenuItem>),
        (With<UiWidget>, Or<(Changed<Band>, Added<UiWidget>)>),
    >,
    mut commands: Commands,
) {
    for (entity, band, is_framed, is_menu_row) in &painted {
        let base = match (is_framed, is_menu_row) {
            (true, _) => FRAME_ORDER,
            (_, true) => MENU_ROW_ORDER,
            _ => UiOrder::OVERLAY,
        };
        let lifted = band.0.saturating_mul(BAND_STEP);
        commands
            .entity(entity)
            .insert(UiOrder(base.0.saturating_add(lifted)));
    }
}
