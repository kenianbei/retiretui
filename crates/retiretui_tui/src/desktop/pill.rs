//! A pill: a raised box of segments, the chosen one lifted off it and
//! said in the ink, the rest dimmed on it.

use bevy_color::{Color, Mix};
use bevy_ecs::prelude::{Commands, Entity};
use bevy_ui::{BackgroundColor, BorderRadius, Node, UiRect, Val};
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;

use super::card::Raised;
use super::chrome::{HALF_CELL_WIDE, Ruled, box_in};
use crate::theme::{Theme, say};

/// How far toward the ink the ground is mixed under the chosen segment.
const LIFT: f32 = 0.12;

/// The ground lifted further, for the chosen segment of a pill.
pub(super) fn lifted(theme: &Theme) -> Color {
    theme.ground().mix(&theme.ink(Style::new()), LIFT)
}

/// Cells a pill's segment keeps either side of its name.
pub(super) const SEGMENT_PADDING: f32 = 1.0;

/// A pill: `node` boxed and raised, its segments spawned under it.
pub(super) fn pill(mut node: Node) -> (Node, Ruled, Raised) {
    box_in(&mut node, 0.0);
    (node, Ruled, Raised)
}

/// A pill's segment, rounded so the chosen one lifts off the pill.
pub(super) fn segment() -> Node {
    Node {
        padding: UiRect::horizontal(Val::Em(SEGMENT_PADDING)),
        border_radius: BorderRadius::all(HALF_CELL_WIDE),
        ..Node::default()
    }
}

/// Says a segment by `name`: the chosen one lifted off its pill and in
/// the ink, any other dimmed on it.
pub(super) fn lift(
    commands: &mut Commands,
    segment: Entity,
    name: &'static str,
    is_chosen: bool,
    theme: &Theme,
) {
    let (fill, style) = if is_chosen {
        (lifted(theme), Style::new())
    } else {
        (Color::NONE, theme.dimmed())
    };
    commands.entity(segment).insert(BackgroundColor(fill));
    say(commands, segment, Line::styled(name, style), theme);
}
