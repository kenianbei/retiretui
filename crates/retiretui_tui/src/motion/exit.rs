//! What has closed, leaving: its last drawn cells played out over the page
//! that is already live beneath them.

use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::Position;
use tachyonfx::{Effect, EffectTimer, SimpleRng, fx};

use super::Leaves;

/// `kept` is the closure's own rather than the effect's state, which is
/// cloned for a reset nothing asks for.
pub fn effect(leaves: Leaves, kept: Buffer, timer: EffectTimer) -> Effect {
    fx::effect_fn_buf((), timer, move |(), context, frame| match leaves {
        Leaves::Dissolve => dissolve(&kept, context.alpha(), frame),
        Leaves::Slide => slide(&kept, context.alpha(), frame),
    })
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
    let place = u32::from(position.x) << u16::BITS | u32::from(position.y);
    SimpleRng::new(place).gen_f32()
}

/// Draws the kept rows as far down as the panel has dropped, and none of
/// them past the edge it stood on: a panel stands on the bottom of the
/// body, and leaves through it.
fn slide(kept: &Buffer, alpha: f32, frame: &mut Buffer) {
    let dropped = (alpha * f32::from(kept.area.height)).round() as u16;
    for position in kept.area.positions() {
        let lowered = Position::new(position.x, position.y.saturating_add(dropped));
        if lowered.y < kept.area.bottom()
            && let Some(cell) = frame.cell_mut(lowered)
        {
            cell.clone_from(&kept[position]);
        }
    }
}

#[cfg(test)]
mod tests {
    use plurimus::core::ratatui_core::layout::Rect;

    use super::*;
    use crate::support::frame_to_string;

    const PANEL: Rect = Rect::new(0, 2, 6, 4);
    const FRAME: Rect = Rect::new(0, 0, 6, 8);

    fn kept() -> Buffer {
        let mut kept = Buffer::empty(PANEL);
        for position in PANEL.positions() {
            kept[position].set_symbol(if position.y == PANEL.y { "t" } else { "k" });
        }
        kept
    }

    fn drawn(frame: &Buffer, row: u16) -> String {
        let rows = frame_to_string(frame);
        rows.lines().nth(usize::from(row)).unwrap().to_owned()
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
    fn a_slide_drops_the_panel_and_never_past_the_edge_it_stood_on() {
        let mut frame = Buffer::empty(FRAME);
        slide(&kept(), 0.0, &mut frame);
        assert_eq!(drawn(&frame, PANEL.y), "tttttt");

        let mut frame = Buffer::empty(FRAME);
        slide(&kept(), 0.5, &mut frame);
        assert_eq!(drawn(&frame, PANEL.y), "      ", "the page shows above it");
        assert_eq!(drawn(&frame, PANEL.y + PANEL.height / 2), "tttttt");
        for row in PANEL.bottom()..FRAME.bottom() {
            assert_eq!(drawn(&frame, row), "      ", "row {row} is beneath it");
        }

        let mut frame = Buffer::empty(FRAME);
        slide(&kept(), 1.0, &mut frame);
        assert_eq!(frame, Buffer::empty(FRAME));
    }

    #[test]
    fn a_frame_grown_smaller_than_what_was_kept_is_not_written_past() {
        let mut frame = Buffer::empty(Rect::new(0, 0, 3, 3));
        dissolve(&kept(), 0.0, &mut frame);
        slide(&kept(), 0.0, &mut frame);
        assert_eq!(drawn(&frame, PANEL.y), "ttt");
    }
}
