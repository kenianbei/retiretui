//! A click on a grid, told apart by where it lands: a body row takes the
//! cursor and the keyboard, as a press on a terminal's table does, and
//! the header names the column pressed. Each is also an event on the
//! grid's root, for what hangs on the grid to act on.

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Entity, EntityEvent, On, Query, Without};
use bevy_ui::ComputedNode;
use plurimus::bui::bevy_picking::events::PointerClick;
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::widgets::ActiveDescendant;

use super::{Grid, GridColumns, Said, solved};
use crate::focus::PageFocus;
use crate::layout;

/// A click on a grid's header, on the cell of its `column`th column.
#[derive(EntityEvent, Debug)]
pub struct HeaderClick {
    pub entity: Entity,
    pub column: usize,
}

/// A click on a row of a grid's body, once the cursor and the keyboard
/// are on it.
#[derive(EntityEvent, Debug)]
pub struct RowClick {
    pub entity: Entity,
    pub row: Entity,
}

/// bevy places a hit across a node from its centre, the left edge at this.
const LEFT_EDGE: f32 = -0.5;

/// The click reaches the root by bubbling, so the row is the click's
/// first target.
pub(super) fn point_on_click(
    click: On<PointerClick>,
    grids: Query<(&Grid, &GridColumns), Without<Said>>,
    (parents, nodes): (Query<&ChildOf>, Query<&ComputedNode>),
    mut focus: PageFocus,
    mut commands: Commands,
) {
    let root = click.entity;
    let row = click.original_event_target();
    let Ok((grid, columns)) = grids.get(root) else {
        return;
    };
    if row == grid.header {
        let Ok(header) = nodes.get(row) else {
            return;
        };
        let across = click.hit.position.map(|hit| hit.x - LEFT_EDGE);
        if let Some(column) = across.and_then(|across| column_at(across, header, &columns.0)) {
            commands.trigger(HeaderClick {
                entity: root,
                column,
            });
        }
        return;
    }
    if parents.get(row).map(ChildOf::parent) != Ok(grid.body) {
        return;
    }
    commands.entity(root).insert(ActiveDescendant(Some(row)));
    focus.set(Some(root));
    commands.trigger(RowClick { entity: root, row });
}

/// The column under a point `across` the header, as a share of its
/// width from the left; none over the gutter or a gap.
fn column_at(across: f32, header: &ComputedNode, columns: &[Constraint]) -> Option<usize> {
    let width = layout::node_cols(header);
    let cell = (across * f32::from(width)).floor();
    let cell = u16::try_from(cell as i32).ok()?;
    solved(columns, width)
        .iter()
        .position(|rect| (rect.x..rect.x.saturating_add(rect.width)).contains(&cell))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_the_header_names_its_column_and_nothing_between() {
        let header = ComputedNode {
            size: bevy_math::Vec2::new(14.0, 1.0),
            inverse_scale_factor: 1.0,
            em_size: plurimus::bui::bevy_text::EmSize(1.0),
            ..ComputedNode::default()
        };
        let columns = [Constraint::Length(4), Constraint::Fill(1)];
        let at = |cell: f32| column_at((cell + 0.5) / 14.0, &header, &columns);
        assert_eq!(at(0.0), None, "the gutter");
        assert_eq!(at(2.0), Some(0));
        assert_eq!(at(5.0), Some(0));
        assert_eq!(at(6.0), None, "the gap");
        assert_eq!(at(13.0), Some(1));
        assert_eq!(column_at(-0.1, &header, &columns), None);
    }
}
