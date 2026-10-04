//! What has closed, leaving: its last drawn cells played out over the page
//! that is already live beneath them.

use std::hash::{DefaultHasher, Hash, Hasher};

use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::{Position, Rect};
use tachyonfx::{Effect, EffectTimer, fx};

use super::Leaves;

/// The bits of a place's hash its grain is made of: as many as a float
/// holds exactly.
const GRAIN_BITS: u32 = 24;

pub fn effect(leaves: Leaves, kept: Buffer, within: Rect, timer: EffectTimer) -> Effect {
    match leaves {
        Leaves::Dissolve => fx::effect_fn_buf(kept, timer, |kept, context, frame| {
            dissolve(kept, context.alpha(), frame);
        }),
        Leaves::Slide => fx::effect_fn_buf(kept, timer, move |kept, context, frame| {
            slide(kept, within, context.alpha(), frame);
        }),
    }
}

/// Draws each kept cell whose place has not yet given way to the page.
fn dissolve(kept: &Buffer, alpha: f32, frame: &mut Buffer) {
    for position in kept.area.positions() {
        if grain(position) >= alpha
            && let Some(cell) = frame.cell_mut(position)
        {
            cell.clone_from(&kept[position]);
        }
    }
}

/// How far through a dissolve a place gives way, from nought up to one:
/// scattered across the box, and the same on every frame.
fn grain(position: Position) -> f32 {
    let mut hasher = DefaultHasher::new();
    position.hash(&mut hasher);
    let kept_bits = hasher.finish() >> (u64::BITS - GRAIN_BITS);
    kept_bits as f32 / (1_u32 << GRAIN_BITS) as f32
}

/// Draws the kept rows as far down as the panel has dropped, and none of
/// them outside `within`.
fn slide(kept: &Buffer, within: Rect, alpha: f32, frame: &mut Buffer) {
    let dropped = (alpha * f32::from(kept.area.height)).round() as u16;
    for position in kept.area.positions() {
        let lowered = Position::new(position.x, position.y.saturating_add(dropped));
        if within.contains(lowered)
            && let Some(cell) = frame.cell_mut(lowered)
        {
            cell.clone_from(&kept[position]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANEL: Rect = Rect::new(0, 2, 6, 4);
    const BODY: Rect = Rect::new(0, 0, 6, 6);
    const FRAME: Rect = Rect::new(0, 0, 6, 8);

    fn kept() -> Buffer {
        let mut kept = Buffer::empty(PANEL);
        for position in PANEL.positions() {
            kept[position].set_symbol(if position.y == PANEL.y { "t" } else { "k" });
        }
        kept
    }

    fn drawn(frame: &Buffer, row: u16) -> String {
        (0..FRAME.width)
            .map(|column| frame[(column, row)].symbol())
            .collect()
    }

    #[test]
    fn a_dissolve_starts_whole_and_ends_as_the_page() {
        let mut frame = Buffer::empty(FRAME);
        dissolve(&kept(), 0.0, &mut frame);
        assert_eq!(drawn(&frame, PANEL.y), "tttttt");
        assert_eq!(drawn(&frame, PANEL.bottom() - 1), "kkkkkk");
        assert_eq!(drawn(&frame, PANEL.bottom()), "      ", "nothing outside");

        let mut frame = Buffer::empty(FRAME);
        dissolve(&kept(), 1.0, &mut frame);
        assert_eq!(frame, Buffer::empty(FRAME));
    }

    #[test]
    fn a_dissolve_breaks_up_part_of_the_way_through() {
        let mut frame = Buffer::empty(FRAME);
        dissolve(&kept(), 0.5, &mut frame);
        let left = PANEL
            .positions()
            .filter(|position| frame[*position].symbol() != " ")
            .count();
        assert!(0 < left && left < PANEL.positions().count(), "{left} left");
    }

    #[test]
    fn a_slide_drops_the_panel_and_never_leaves_the_body() {
        let mut frame = Buffer::empty(FRAME);
        slide(&kept(), BODY, 0.0, &mut frame);
        assert_eq!(drawn(&frame, PANEL.y), "tttttt");

        let mut frame = Buffer::empty(FRAME);
        slide(&kept(), BODY, 0.5, &mut frame);
        assert_eq!(drawn(&frame, PANEL.y), "      ", "the page shows above it");
        assert_eq!(drawn(&frame, PANEL.y + PANEL.height / 2), "tttttt");
        for row in BODY.bottom()..FRAME.bottom() {
            assert_eq!(
                drawn(&frame, row),
                "      ",
                "row {row} is beneath the body"
            );
        }

        let mut frame = Buffer::empty(FRAME);
        slide(&kept(), BODY, 1.0, &mut frame);
        assert_eq!(frame, Buffer::empty(FRAME));
    }

    #[test]
    fn a_frame_grown_smaller_than_what_was_kept_is_not_written_past() {
        let mut frame = Buffer::empty(Rect::new(0, 0, 3, 3));
        dissolve(&kept(), 0.0, &mut frame);
        slide(&kept(), BODY, 0.0, &mut frame);
        assert_eq!(drawn_narrow(&frame), "ttt");
    }

    fn drawn_narrow(frame: &Buffer) -> String {
        (0..frame.area.width)
            .map(|column| frame[(column, PANEL.y)].symbol())
            .collect()
    }
}
