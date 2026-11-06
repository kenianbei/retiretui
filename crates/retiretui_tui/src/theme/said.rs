//! What a chrome node says, as the spans of a bevy `Text`: a ratatui line
//! in the theme's roles, each span in its colour and weight, drawn in a
//! terminal's cells by plurimus and in a window by bevy's text.

use bevy_ecs::bundle::Bundle;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity};
use bevy_ui::widget::Text;
use bevy_ui::{Node, Overflow, PositionType, Val};
use plurimus::bui::bevy_text::{
    FontWeight, TextBackgroundColor, TextColor, TextFont, TextLayout, TextSpan,
};
use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::{Position, Rect};
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::ratatui_core::text::{Line, Span};

use super::{Theme, paper};

/// A node that says one line of chrome, never wrapped.
#[must_use]
pub fn text() -> impl Bundle {
    (Text::default(), TextLayout::default().with_no_wrap())
}

/// The text hung over a node by [`hang`].
#[derive(Component, Debug)]
pub struct Hung(pub Entity);

/// Hangs a text over `holder`, placed over its whole box and clipped to
/// it, so that what it says never sizes the holder: a text node measures
/// its longest line, which a pane sharing a band would widen to hold,
/// where a paragraph drawn into an area was cut at its edge.
pub fn hang(commands: &mut Commands, holder: Entity) -> Entity {
    let hung = commands
        .spawn((
            text(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                top: Val::ZERO,
                right: Val::ZERO,
                bottom: Val::ZERO,
                ..Node::default()
            },
            ChildOf(holder),
        ))
        .id();
    commands
        .entity(holder)
        .insert(Hung(hung))
        .entry::<Node>()
        .or_default()
        .and_modify(|mut node| node.overflow = Overflow::clip());
    hung
}

/// Makes `root`'s spans say `lines`, one under another: bui breaks a
/// span at a newline, so the rows are one text.
pub fn say_lines(commands: &mut Commands, root: Entity, lines: Vec<Line<'static>>, theme: &Theme) {
    let mut spans = Vec::new();
    for (at, line) in lines.into_iter().enumerate() {
        if at > 0 {
            spans.push(Span::raw("\n"));
        }
        spans.extend(line.spans);
    }
    say(commands, root, Line::from(spans), theme);
}

/// Makes `root`'s spans say `line`, replacing whatever it said before;
/// each span is said in its own style over the line's, as ratatui draws
/// it.
pub fn say(commands: &mut Commands, root: Entity, line: Line<'static>, theme: &Theme) {
    let mut root = commands.entity(root);
    root.despawn_related::<Children>();
    root.with_children(|spans| {
        for span in line.spans {
            let style = line.style.patch(span.style);
            spans
                .spawn(said(Span::styled(span.content, style), theme))
                .insert_if(TextBackgroundColor(paper(style.bg)), || style.bg.is_some());
        }
    });
}

/// Row `y` of `buffer` as spans of its cells, runs of one style together,
/// the cells in `blank` said as spaces: what ratatui drew, said as text.
pub fn row_line(buffer: &Buffer, y: u16, blank: Rect) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for x in buffer.area.left()..buffer.area.right() {
        let cell = &buffer[(x, y)];
        let (symbol, style) = if blank.contains(Position::new(x, y)) {
            (" ", Style::default())
        } else {
            (cell.symbol(), cell.style())
        };
        match spans.last_mut() {
            Some(last) if last.style == style => last.content.to_mut().push_str(symbol),
            _ => spans.push(Span::styled(symbol.to_owned(), style)),
        }
    }
    Line::from(spans)
}

/// A span as bevy's text takes it: its words, its ink and its weight.
pub fn said(span: Span<'static>, theme: &Theme) -> (TextSpan, TextColor, TextFont) {
    let weight = if span.style.add_modifier.contains(Modifier::BOLD) {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    };
    (
        TextSpan(span.content.into_owned()),
        TextColor(theme.ink(span.style)),
        TextFont {
            weight,
            ..TextFont::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use plurimus::core::ratatui_core::style::Color;

    use super::*;

    #[test]
    fn a_span_is_said_in_its_colour_and_weight_or_the_themes_ink() {
        let theme = Theme {
            fg: Color::Rgb(1, 2, 3),
            accent: Color::Rgb(4, 5, 6),
            ..Theme::default()
        };
        let (_, plain, light) = said(Span::raw("plain"), &theme);
        assert_eq!(plain.0, bevy_color::Color::srgb_u8(1, 2, 3));
        assert!(light.weight.0 < FontWeight::SEMIBOLD.0);
        let bold = theme.accented().add_modifier(Modifier::BOLD);
        let (_, accented, heavy) = said(Span::styled("key", bold), &theme);
        assert_eq!(accented.0, bevy_color::Color::srgb_u8(4, 5, 6));
        assert!(heavy.weight.0 >= FontWeight::SEMIBOLD.0);
    }
}
