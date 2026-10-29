//! What stands over the page in the window: an overlay above the dim laid
//! over the body beneath it, and a centred box placed in the body's
//! middle, which the terminal places on whole cells.

use bevy_app::{App, Startup, Update};
use bevy_color::Alpha;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Add, Changed, Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, With,
};
use bevy_ui::{BackgroundColor, Display, Node, PositionType, UiRect, Val, ZIndex};
use plurimus::ui::ModalOpen;

use crate::layout::{self, Body, set_display};
use crate::motion::Motion;
use crate::overlay::Centred;
use crate::settings::Settings;
use crate::theme::{Repainted, Theme};

/// How far the body fades under an overlay.
const DIM: f32 = 0.6;
const DIM_LAYER: ZIndex = ZIndex(1);
const OVERLAY_LAYER: ZIndex = ZIndex(2);

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_dim.after(layout::spawn_frame));
    app.add_systems(Update, dim_the_body.in_set(Repainted));
    app.add_observer(raise_the_overlay);
    app.add_systems(Update, centre_the_boxes);
}

/// The translucent node over the body, shown while an overlay stands.
#[derive(Component, Debug)]
struct Dim;

fn spawn_dim(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
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
        DIM_LAYER,
        ChildOf(body),
    ));
}

fn raise_the_overlay(added: On<Add<ModalOpen>>, mut commands: Commands) {
    commands.entity(added.entity).insert(OVERLAY_LAYER);
}

/// The dim stands while an overlay does and the motion is not off, in the
/// ground's colour at the dim's strength.
fn dim_the_body(
    theme: Res<Theme>,
    settings: Res<Settings>,
    overlays: Query<(), With<ModalOpen>>,
    mut dims: Query<(Entity, &mut Node), With<Dim>>,
    mut commands: Commands,
) {
    for (dim, mut node) in &mut dims {
        let is_shown = !overlays.is_empty() && settings.motion != Motion::Off;
        set_display(&mut node, is_shown);
        if theme.is_changed() {
            let ground = theme.ground().with_alpha(DIM);
            commands.entity(dim).insert(BackgroundColor(ground));
        }
    }
}

/// A centred box is laid in the body's middle at the height it holds,
/// which an asker may change after it opens, with the room on every side
/// left to the layout.
fn centre_the_boxes(mut boxes: Query<(&Centred, &mut Node), Changed<Centred>>) {
    for (centred, mut node) in &mut boxes {
        node.left = Val::ZERO;
        node.right = Val::ZERO;
        node.top = Val::ZERO;
        node.bottom = Val::ZERO;
        node.margin = UiRect::all(Val::Auto);
        node.height = Val::Rem(f32::from(centred.rows));
    }
}
