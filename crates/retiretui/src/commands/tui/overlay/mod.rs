//! What stands over the shell: who holds the keyboard while it does, the
//! order it is painted in, and the box it is laid out in.

mod band;
mod focus;
mod standing;

use bevy_app::{App, PostUpdate};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Entity};
use bevy_input::keyboard::Key;
use plurimus::core::ratatui_core::layout::Size;
use plurimus::ui::{KeyBinding, ModalOpen, ScrollArea};
use plurimus::widgets::listbox;

use super::hints::Hints;
use super::layout::{filling, list_cursor, placed};
use super::pane::Framed;
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::prelude::{Component, IntoScheduleConfigs, Query, With};
use bevy_ui::{FlexDirection, Node, Overflow, PositionType, UiRect, UiSystems, Val};
use plurimus::bui::ComputedNodeRect;

pub use focus::Focus;
pub use standing::Standing;

use super::layout::Body;

pub fn plugin(app: &mut App) {
    app.init_resource::<Focus>();
    app.add_plugins(band::plugin);
    app.add_systems(PostUpdate, centre_boxes.before(UiSystems::Layout));
}

/// The border and the cell kept clear inside it, on each side.
pub const CHROME: u16 = 2;

/// The share of the body a panel drawn up from its bottom covers, per
/// cent.
const BOTTOM_SHARE: f32 = 40.0;

pub const CLOSE_KEYS: &[(KeyBinding, ())] = &[(KeyBinding::new(Key::Escape), ())];

/// A box centred over the body, by its outer size in cells.
#[derive(Component, Clone, Copy, Debug)]
pub struct Centred {
    cols: u16,
    rows: u16,
}

/// The box a centred overlay is laid out in: `cols` cells across and as
/// tall as the `rows` it holds inside its frame, clipped where the body
/// gives it less.
#[must_use]
pub fn centred(cols: u16, rows: u16) -> (Node, Centred) {
    let rows = rows.saturating_add(CHROME);
    let node = Node {
        position_type: PositionType::Absolute,
        width: Val::Px(f32::from(cols)),
        height: Val::Px(f32::from(rows)),
        max_height: Val::Percent(100.0),
        padding: UiRect::all(Val::Px(1.0)),
        flex_direction: FlexDirection::Column,
        overflow: Overflow::clip(),
        ..Node::default()
    };
    (node, Centred { cols, rows })
}

/// Lays a standing `root` out as a panel up from the bottom of the body:
/// framed and titled, holding the keys, hinting `hints`, and answering
/// with the list it holds.
pub fn bottom_panel(commands: &mut Commands, root: Entity, title: &str, hints: Hints) -> Entity {
    commands.entity(root).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            height: Val::Percent(BOTTOM_SHARE),
            padding: UiRect::all(Val::Px(1.0)),
            flex_direction: FlexDirection::Column,
            ..Node::default()
        },
        Framed::over(title),
        ModalOpen,
        hints,
    ));
    commands
        .spawn((
            listbox(),
            list_cursor(),
            ScrollArea::new(Size::default()),
            filling(),
            placed(),
            ChildOf(root),
        ))
        .id()
}

/// Places each centred box on whole cells. Left to the layout, a box an
/// odd number of cells narrower than the body starts on half a cell, and
/// what it holds is then rounded a cell past its border.
fn centre_boxes(
    bodies: Query<Ref<ComputedNodeRect>, With<Body>>,
    mut boxes: Query<(Ref<Centred>, &mut Node)>,
) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let room = body.visible;
    for (centred, mut node) in &mut boxes {
        if !body.is_changed() && !centred.is_added() {
            continue;
        }
        let rows = centred.rows.min(room.height);
        node.left = Val::Px(f32::from(room.width.saturating_sub(centred.cols) / 2));
        node.top = Val::Px(f32::from(room.height.saturating_sub(rows).div_ceil(2)));
    }
}
