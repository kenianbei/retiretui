//! A rate's or a share's slider: the stock headless slider over a text
//! node both skins draw as its track, beside the text that says the value
//! exactly.

use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res};
use bevy_input_focus::InputFocus;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui::{Node, UiRect, Val};
use plurimus::bui::bevy_ui_widgets::{Slider, SliderRange, SliderStep, SliderValue, TrackClick};
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use toml::Value;

use super::build::FormField;
use super::domain::FieldKind;
use super::field::{sharing, spawn_text};
use crate::layout::{placed, sized};
use crate::theme::{Theme, say, text};

/// The highest rate a slider offers; a higher one is still typable.
const RATE_CEILING: f32 = 0.15;
const RATE_STEP: f32 = 0.0025;
/// A share's slider reaches the whole in these steps.
const SHARE_CEILING: f32 = 1.0;
const SHARE_STEP: f32 = 0.05;
/// Cells the slider takes, leaving the rest of the field for the text.
pub const WIDTH: u16 = 20;
/// Cells kept clear between the slider and the text beside it.
pub const GAP: f32 = 1.0;

/// The track's glyphs: filled up to the thumb, the thumb, and the rest.
const FILLED: &str = "━";
const THUMB: &str = "█";
const EMPTY: &str = "─";

/// A slider's track, and whether it was last drawn held.
#[derive(Component, Debug, Default)]
pub struct Track {
    drawn_held: Option<bool>,
}

/// The slider, and the text beside it that says the value exactly.
pub fn spawn(commands: &mut Commands, row: Entity, field: FormField) {
    let (ceiling, step) = if field.spec.kind == FieldKind::Share {
        (SHARE_CEILING, SHARE_STEP)
    } else {
        (RATE_CEILING, RATE_STEP)
    };
    commands.spawn((
        Slider {
            track_click: TrackClick::Snap,
            ..Slider::default()
        },
        SliderRange::new(0.0, ceiling),
        SliderStep(step),
        Track::default(),
        field,
        TabIndex(0),
        text(),
        Node {
            margin: UiRect::right(Val::Em(GAP)),
            ..sized(f32::from(WIDTH), 1.0)
        },
        placed(),
        ChildOf(row),
    ));
    spawn_text(commands, row, sharing(), field);
}

/// A slider's rate reaches the field as a float, rounded to its own step
/// so the text beside it reads as the slider moved.
pub fn value(value: f32) -> Value {
    let stepped = f64::from((value / RATE_STEP).round() * RATE_STEP);
    Value::Float((stepped * 1e6).round() / 1e6)
}

/// `━━━█────`, the thumb where the value falls in the range, in the
/// focused style while the slider holds the keyboard.
pub fn paint(
    theme: Res<Theme>,
    focus: Res<InputFocus>,
    mut tracks: Query<(Entity, &mut Track, Ref<SliderValue>, &SliderRange)>,
    mut commands: Commands,
) {
    for (entity, mut track, value, range) in &mut tracks {
        let is_held = focus.get() == Some(entity);
        let is_stale =
            value.is_changed() || theme.is_changed() || track.drawn_held != Some(is_held);
        if !is_stale {
            continue;
        }
        track.drawn_held = Some(is_held);
        let style = if is_held {
            theme.focused()
        } else {
            Style::default()
        };
        let drawn = glyphs(range.thumb_position(value.0), WIDTH);
        say(&mut commands, entity, Line::styled(drawn, style), &theme);
    }
}

/// The track over `width` cells, the thumb at `ratio` along it.
fn glyphs(ratio: f32, width: u16) -> String {
    let last = width.saturating_sub(1);
    let thumb = (ratio.clamp(0.0, 1.0) * f32::from(last)).round() as u16;
    (0..width)
        .map(|cell| match cell.cmp(&thumb) {
            std::cmp::Ordering::Less => FILLED,
            std::cmp::Ordering::Equal => THUMB,
            std::cmp::Ordering::Greater => EMPTY,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumb_sits_where_the_value_falls_and_at_either_end_at_the_ends() {
        assert_eq!(glyphs(0.0, 5), "█────");
        assert_eq!(glyphs(1.0, 5), "━━━━█");
        assert_eq!(glyphs(0.5, 5), "━━█──");
        assert_eq!(glyphs(2.0, 3), "━━█", "past the end is the end");
    }
}
