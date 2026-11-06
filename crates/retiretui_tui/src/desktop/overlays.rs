//! What stands over the page in the window: each overlay raised by how
//! deep it stands, the dim laid over the whole frame just under the
//! topmost, a menu just over it, and a centred box placed in the body's
//! middle, which the terminal places on whole cells.

use bevy_app::{App, Startup, Update};
use bevy_color::Alpha;
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, IntoScheduleConfigs, Query, Res, With,
};
use bevy_ui::{BackgroundColor, Display, GlobalZIndex, Node, PositionType, Val};
use plurimus::ui::ModalOpen;

use crate::layout::{self, Root, set_display};
use crate::motion::Motion;
use crate::overlay::Band;
use crate::settings::Settings;
use crate::theme::{Repainted, Theme};

/// How far the frame fades under an overlay.
const DIM: f32 = 0.6;
/// Overlays are layered three apart: the dim just under each, and a menu
/// opened from it just over.
const LAYERS_PER_OVERLAY: i32 = 3;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_dim.after(layout::spawn_frame));
    app.add_systems(
        Update,
        (raise_the_overlays, dim_the_frame)
            .chain()
            .in_set(Repainted),
    );
}

/// The translucent node over the frame, shown while an overlay stands.
#[derive(Component, Debug)]
struct Dim;

fn spawn_dim(roots: Query<Entity, With<Root>>, mut commands: Commands) {
    let Ok(root) = roots.single() else {
        return;
    };
    commands.spawn((
        Dim,
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::ZERO,
            top: Val::ZERO,
            bottom: Val::ZERO,
            display: Display::None,
            ..Node::default()
        },
        GlobalZIndex::default(),
        ChildOf(root),
    ));
}

/// The layer of an overlay with `beneath` overlays under it.
const fn overlay_layer(beneath: i32) -> GlobalZIndex {
    GlobalZIndex(beneath.saturating_add(1).saturating_mul(LAYERS_PER_OVERLAY))
}

/// The layer of a menu opened over the topmost overlay, or over the page
/// where none stands.
pub(super) fn menu_layer(overlays: &Query<&Band, With<ModalOpen>>) -> GlobalZIndex {
    let top = overlays
        .iter()
        .max()
        .map_or(0, |band| overlay_layer(band.beneath()).0);
    GlobalZIndex(top.saturating_add(1))
}

/// Each overlay drawn over the ones under it, as its band lands on it.
fn raise_the_overlays(
    overlays: Query<(Entity, &Band), (With<ModalOpen>, Changed<Band>)>,
    mut commands: Commands,
) {
    for (overlay, band) in &overlays {
        commands
            .entity(overlay)
            .insert(overlay_layer(band.beneath()));
    }
}

/// The dim stands just under the topmost overlay while one does and the
/// motion is not off, in the ground's colour at the dim's strength.
fn dim_the_frame(
    theme: Res<Theme>,
    settings: Res<Settings>,
    overlays: Query<&Band, With<ModalOpen>>,
    mut dims: Query<(Entity, &mut Node, &mut GlobalZIndex), With<Dim>>,
    mut commands: Commands,
) {
    let topmost = overlays.iter().max();
    for (dim, mut node, mut layer) in &mut dims {
        let is_shown = topmost.is_some() && settings.motion != Motion::Off;
        set_display(&mut node, is_shown);
        if let Some(band) = topmost {
            let GlobalZIndex(above) = overlay_layer(band.beneath());
            layer.set_if_neq(GlobalZIndex(above - 1));
        }
        if theme.is_changed() {
            let ground = theme.ground().with_alpha(DIM);
            commands.entity(dim).insert(BackgroundColor(ground));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::RunSystemOnce;
    use plurimus::term::KeyCode;

    use bevy_ecs::prelude::Has;
    use plurimus::core::UiHidden;
    use plurimus::widgets::MenuPopup;

    use super::*;
    use crate::desktop::menus::Shadow;
    use crate::edit::tests::{open, retype_balance, tab_to_field};
    use crate::nav::Page;
    use crate::settings::Settings;
    use crate::support::{SIZE, answer_back, headless_app_set, press_key, scratch_full_plan};

    /// The dialog's third button, which goes back to the form.
    const CANCEL: usize = 2;

    /// The dim's layer, and every open overlay's.
    fn layers(app: &mut App) -> (i32, Vec<i32>) {
        let world = app.world_mut();
        let dim = world
            .query_filtered::<(&GlobalZIndex, &Node), With<Dim>>()
            .single(world)
            .map(|(layer, node)| {
                assert_eq!(node.display, Display::Flex, "the dim is shown");
                layer.0
            })
            .unwrap();
        let mut overlays: Vec<i32> = world
            .query_filtered::<&GlobalZIndex, With<ModalOpen>>()
            .iter(world)
            .map(|layer| layer.0)
            .collect();
        overlays.sort_unstable();
        (dim, overlays)
    }

    /// Stops along an account's form to its kind, a select.
    const KIND_FIELD: usize = 2;

    #[test]
    fn a_menu_over_a_form_stands_over_the_dim() {
        let mut app = headless_app_set(scratch_full_plan(), SIZE, Settings::default());
        app.add_plugins((plugin, crate::desktop::menus::plugin));
        app.world_mut().run_system_once(spawn_dim).unwrap();
        open(&mut app, Page::Accounts);
        tab_to_field(&mut app, KIND_FIELD);
        press_key(&mut app, KeyCode::Char(' '));
        app.update();
        let (dim, overlays) = layers(&mut app);
        let world = app.world_mut();
        let shadow = world
            .query_filtered::<(&Shadow, Has<UiHidden>), With<MenuPopup>>()
            .iter(world)
            .find_map(|(shadow, is_hidden)| (!is_hidden).then_some(shadow.0))
            .expect("the kind's menu is open and shadowed");
        let menu = world.get::<GlobalZIndex>(shadow).expect("layered").0;
        assert!(menu > overlays[0], "over the form");
        assert!(overlays[0] > dim, "which is over the dim");
    }

    #[test]
    fn the_dim_stands_just_under_the_topmost_overlay() {
        let mut app = headless_app_set(scratch_full_plan(), SIZE, Settings::default());
        app.add_plugins(plugin);
        app.world_mut().run_system_once(spawn_dim).unwrap();
        open(&mut app, Page::Accounts);
        app.update();
        let (dim, overlays) = layers(&mut app);
        assert_eq!(overlays, vec![dim + 1], "the form alone, over the dim");
        retype_balance(&mut app);
        press_key(&mut app, KeyCode::Esc);
        app.update();
        let (dim, overlays) = layers(&mut app);
        assert_eq!(overlays.len(), 2, "the dialog asks over the form");
        assert_eq!(overlays[1], dim + 1, "the dialog over the dim");
        assert!(overlays[0] < dim, "the form under it");
        answer_back(&mut app, CANCEL);
        app.update();
        let (dim, overlays) = layers(&mut app);
        assert_eq!(overlays, vec![dim + 1], "the form over the dim again");
    }
}
