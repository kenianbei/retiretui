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

/// The gutter beside a row the cursor is not on, as wide as the cursor.
pub const GUTTER: &str = "  ";
const _: () = assert!(GUTTER.len() == CURSOR_COLS as usize);

/// The column a scrolled list or table keeps for its scrollbar.
pub const SCROLL_BAR_COLS: u16 = 1;

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

/// The bar a cursor row shows: full while `is_held` says the keyboard is
/// on its table, thin otherwise.
#[must_use]
pub fn cursor_bar(is_held: bool) -> &'static str {
    if is_held { CURSOR_BAR } else { RESTING_BAR }
}

fn bar_for(holder: Option<Entity>, entity: Entity) -> Line<'static> {
    Line::from(cursor_bar(holder == Some(entity)))
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
