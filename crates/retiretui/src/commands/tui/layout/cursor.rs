//! The bar marking the row a cursor is on, in place of the stock `> `:
//! full where the keyboard is, and thin where a list only keeps its place.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Component, Entity, Query, Res, With};
use bevy_input_focus::InputFocus;
use plurimus::core::ratatui_core::text::Line;
use plurimus::widgets::{ListBoxCursor, TableCursor};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, rest_cursors);
}

/// Left unstyled so that the row's own cursor style paints it, and as
/// wide as the stock cursor so no column moves under it.
const CURSOR_BAR: &str = "\u{258c} ";

/// The cells the cursor takes at the head of a row: what the columns do
/// not get, and what everything drawn beside the rows lines up under.
pub const CURSOR_COLS: u16 = 2;

/// The cursor every table draws, shared so two tables cannot drift.
#[must_use]
pub fn table_cursor() -> TableCursor {
    TableCursor(Line::from(CURSOR_BAR))
}

/// The cursor every list draws, the same bar a table's is.
#[must_use]
pub fn list_cursor() -> ListBoxCursor {
    ListBoxCursor(Line::from(CURSOR_BAR))
}

/// The bar of a list the keyboard has left, which still says where its
/// cursor is without claiming the keys go there.
const RESTING_BAR: &str = "\u{258f} ";

/// A list or table that stands beside another the keyboard moves
/// between, and so draws its cursor thin while it is elsewhere.
#[derive(Component, Debug)]
pub struct Rests;

fn bar_for(holder: Option<Entity>, entity: Entity) -> Line<'static> {
    if holder == Some(entity) {
        Line::from(CURSOR_BAR)
    } else {
        Line::from(RESTING_BAR)
    }
}

fn rest_cursors(
    focus: Res<InputFocus>,
    mut lists: Query<(Entity, &mut ListBoxCursor), With<Rests>>,
    mut tables: Query<(Entity, &mut TableCursor), With<Rests>>,
) {
    if !focus.is_changed() {
        return;
    }
    let holder = focus.get();
    for (entity, mut cursor) in &mut lists {
        let wanted = bar_for(holder, entity);
        if cursor.0 != wanted {
            cursor.0 = wanted;
        }
    }
    for (entity, mut cursor) in &mut tables {
        let wanted = bar_for(holder, entity);
        if cursor.0 != wanted {
            cursor.0 = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_bar_is_as_wide_as_the_gutter_it_is_drawn_in() {
        for bar in [CURSOR_BAR, RESTING_BAR] {
            assert_eq!(
                Line::from(bar).width(),
                usize::from(CURSOR_COLS),
                "a wider bar would shift every column under it"
            );
        }
    }
}
