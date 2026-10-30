//! Where a placed widget is: its node's laid-out box in whole cells, read
//! from bevy's layout as the terminal skin reads it from bui's, so that
//! what measures itself by its area measures the same in a window.

use bevy_app::{App, PostUpdate};
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::{Changed, IntoScheduleConfigs, Or, Query, Res};
use bevy_math::Vec2;
use bevy_ui::{ComputedNode, UiGlobalTransform, UiSystems};
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::{DefaultCamera, ResolvedViewport, UiArea, local_area};

use super::CELL;
use crate::layout;

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, place_widgets.after(UiSystems::PostLayout));
}

type Moved = Or<(Changed<ComputedNode>, Changed<UiGlobalTransform>)>;

/// Writes each widget's area from its node's box in logical pixels over
/// the cell, in the camera-local cells `UiArea::Fixed` speaks.
fn place_widgets(
    default_camera: Res<DefaultCamera>,
    cameras: Query<&ResolvedViewport>,
    mut widgets: Query<(&ComputedNode, &UiGlobalTransform, &mut UiArea), Moved>,
) {
    let Some(viewport) = layout::default_viewport(*default_camera, &cameras) else {
        return;
    };
    for (computed, transform, mut area) in &mut widgets {
        let rect = widget_cells(computed, transform, viewport, CELL);
        area.set_if_neq(UiArea::Fixed(local_area(rect, viewport)));
    }
}

/// A node's box in screen cells, `cell` logical pixels each.
fn widget_cells(
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
    viewport: Rect,
    cell: Vec2,
) -> Rect {
    let half = computed.size / 2.0;
    let in_cells = |corner: Vec2| corner * computed.inverse_scale_factor() / cell;
    let min = in_cells(transform.translation - half);
    let max = in_cells(transform.translation + half);
    cell_rect(min, max, viewport)
}

/// The cells between two corners given in cells, each edge rounded on its
/// own so that adjacent nodes stay gapless, cut to `viewport`; nothing
/// where none of it is in view.
fn cell_rect(min: Vec2, max: Vec2, viewport: Rect) -> Rect {
    let edge = |position: f32, from: u16, to: u16| {
        (position.round() as i32).clamp(from.into(), to.into()) as u16
    };
    let (left, right) = (
        edge(min.x, viewport.left(), viewport.right()),
        edge(max.x, viewport.left(), viewport.right()),
    );
    let (top, bottom) = (
        edge(min.y, viewport.top(), viewport.bottom()),
        edge(max.y, viewport.top(), viewport.bottom()),
    );
    if right <= left || bottom <= top {
        return Rect::ZERO;
    }
    Rect::new(left, top, right - left, bottom - top)
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::With;
    use plurimus::bui::ComputedNodeRect;
    use plurimus::core::UiWidget;

    use super::*;
    use crate::nav::Page;
    use crate::support::{SIZE, headless_app, show};

    const VIEWPORT: Rect = Rect::new(0, 0, 128, 32);

    /// bui lays the terminal out at one pixel per cell, so its rects are
    /// what this arithmetic must give for the same nodes at that cell.
    #[test]
    fn every_placed_widget_lands_where_bui_lays_it() {
        let mut app = headless_app(SIZE);
        show(&mut app, Page::Ledger);
        let viewport = Rect::new(0, 0, SIZE.cols, SIZE.rows);
        let mut widgets = app
            .world_mut()
            .query_filtered::<(&ComputedNode, &UiGlobalTransform, &ComputedNodeRect), With<UiWidget>>();
        let mut placed = 0;
        for (computed, transform, laid) in widgets.iter(app.world()) {
            let ours = widget_cells(computed, transform, viewport, Vec2::ONE);
            assert_eq!(ours, laid.rect, "{laid:?}");
            placed += usize::from(!ours.is_empty());
        }
        assert!(placed >= 3, "the frame, the band and the years: {placed}");
    }

    #[test]
    fn a_box_lands_on_the_cells_its_edges_round_to_and_within_the_viewport() {
        let whole = cell_rect(Vec2::new(2.0, 3.0), Vec2::new(12.4, 5.6), VIEWPORT);
        assert_eq!(whole, Rect::new(2, 3, 10, 3));
        let hanging = cell_rect(Vec2::new(-3.0, 30.0), Vec2::new(4.0, 40.0), VIEWPORT);
        assert_eq!(hanging, Rect::new(0, 30, 4, 2), "cut to what is in view");
        let outside = cell_rect(Vec2::new(130.0, 0.0), Vec2::new(140.0, 2.0), VIEWPORT);
        assert_eq!(outside, Rect::ZERO);
        let thin = cell_rect(Vec2::new(5.0, 5.0), Vec2::new(5.4, 6.0), VIEWPORT);
        assert_eq!(thin, Rect::ZERO, "thinner than a cell is nothing");
    }
}
