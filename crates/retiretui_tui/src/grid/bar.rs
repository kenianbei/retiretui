//! The bar beside a grid's rows: ratatui's scrollbar drawn into a scratch
//! buffer as tall as the body and said a glyph a row, so a terminal shows
//! what it showed of a scrolled table and a window the same glyphs.

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res};
use bevy_ui::{ComputedNode, FlexDirection, Node, Val};
use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::text::Line;
use plurimus::core::ratatui_core::widgets::StatefulWidget;
use plurimus::widgets::ratatui_widgets::scrollbar::{
    Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::Grid;
use crate::layout::{self, SCROLL_BAR_COLS, fixed};
use crate::theme::{Theme, say, text};

/// What the bar was last drawn for: the rows there are, the rows shown
/// and the row at the top; nothing while every row fits.
#[derive(Component, Debug, Default, PartialEq, Eq)]
pub(crate) struct Bar(Option<(u16, u16, u16)>);

impl Bar {
    /// Says the bar again down `column` where `rows`, `shown` or `top`
    /// moved since it was last said: a glyph a row, blank where every
    /// row fits.
    pub(crate) fn redraw(
        &mut self,
        (rows, shown, top): (u16, u16, u16),
        column: Entity,
        commands: &mut Commands,
        theme: &Theme,
    ) {
        let now = (rows > shown).then_some((rows, shown, top));
        if self.0 == now {
            return;
        }
        self.0 = now;
        let glyphs = now.map_or_else(Vec::new, |(rows, shown, top)| {
            glyphs(rows - shown, shown, top)
        });
        say_glyphs(commands, column, glyphs, theme);
    }
}

pub(crate) fn column() -> Node {
    Node {
        width: Val::Em(f32::from(SCROLL_BAR_COLS)),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        ..Node::default()
    }
}

/// The glyph on each of `shown` rows of a bar whose content is `content`
/// places long with `top` the one in view: ratatui's scrollbar, said a
/// row at a time.
pub(crate) fn glyphs(content: u16, shown: u16, top: u16) -> Vec<String> {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, shown));
    let mut state = ScrollbarState::new(usize::from(content)).position(usize::from(top));
    Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
        buffer.area,
        &mut buffer,
        &mut state,
    );
    (0..shown)
        .map(|y| buffer[(0, y)].symbol().to_owned())
        .collect()
}

/// Redraws each bar whose rows, height or offset moved.
pub(super) fn draw_bars(
    theme: Res<Theme>,
    mut grids: Query<(&Grid, &mut Bar)>,
    bodies: Query<(&ComputedNode, Option<&Children>)>,
    mut commands: Commands,
) {
    for (grid, mut bar) in &mut grids {
        let Ok((body, rows)) = bodies.get(grid.body) else {
            continue;
        };
        let rows = rows.map_or(0, |rows| rows.len());
        let (rows, shown) = (
            u16::try_from(rows).unwrap_or(u16::MAX),
            layout::node_rows(body),
        );
        let scrolled = body.scroll_position.y * body.inverse_scale_factor;
        let top = (scrolled / body.rem_size.0).round() as u16;
        bar.redraw((rows, shown, top), grid.bar, &mut commands, &theme);
    }
}

/// Replaces `column`'s rows with one saying each of `glyphs`.
pub(crate) fn say_glyphs(
    commands: &mut Commands,
    column: Entity,
    glyphs: Vec<String>,
    theme: &Theme,
) {
    commands.entity(column).despawn_related::<Children>();
    for glyph in glyphs {
        let row = commands.spawn((text(), fixed(1.0), ChildOf(column))).id();
        say(commands, row, Line::from(glyph), theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_is_ratatui_s_from_its_arrow_to_its_arrow() {
        let drawn = glyphs(30, 10, 0);
        assert_eq!(drawn.len(), 10);
        assert_eq!((drawn[0].as_str(), drawn[9].as_str()), ("▲", "▼"));
        assert!(drawn[1..9].iter().any(|glyph| glyph == "█"), "{drawn:?}");
        let end = glyphs(30, 10, 30);
        assert_ne!(drawn, end, "the thumb moves with the offset");
    }
}
