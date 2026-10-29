//! What a chrome node says, as the spans of a bevy `Text`: a ratatui line
//! in the theme's roles, each span in its colour and weight, drawn in a
//! terminal's cells by plurimus and in a window by bevy's text.

use bevy_ecs::bundle::Bundle;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::{Commands, Entity};
use bevy_ui::widget::Text;
use plurimus::bui::bevy_text::{FontWeight, TextColor, TextFont, TextLayout, TextSpan};
use plurimus::core::ratatui_core::style::Modifier;
use plurimus::core::ratatui_core::text::{Line, Span};

use super::Theme;

/// A node that says one line of chrome, never wrapped.
#[must_use]
pub fn text() -> impl Bundle {
    (Text::default(), TextLayout::default().with_no_wrap())
}

/// Makes `root`'s spans say `line`, replacing whatever it said before.
pub fn say(commands: &mut Commands, root: Entity, line: Line<'static>, theme: &Theme) {
    let mut root = commands.entity(root);
    root.despawn_related::<Children>();
    root.with_children(|spans| {
        for span in line.spans {
            spans.spawn(said(span, theme));
        }
    });
}

fn said(span: Span<'static>, theme: &Theme) -> (TextSpan, TextColor, TextFont) {
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
