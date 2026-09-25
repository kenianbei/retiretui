//! A pane's list of word-wrapped rows, scrolled through.

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Entity, EntityCommands};
use plurimus::core::ratatui_core::layout::Size;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use plurimus::widgets::{ActiveDescendant, list_item, listbox};

use super::{CURSOR_COLS, Rests, filling, list_cursor, placed, wrapped};
use crate::commands::tui::hints::Hints;
use crate::commands::tui::nav::FocusStop;

/// What leads each line a row runs on to past its first.
const CONTINUED: &str = "  ";

/// A list filling `pane`, its rows scrolled through.
pub fn spawn_scrolled_list(commands: &mut Commands, pane: Entity, hints: Hints) -> Entity {
    commands
        .spawn((
            listbox(),
            list_cursor(),
            ScrollArea::new(Size::default()),
            hints,
            Rests,
            FocusStop,
            filling(),
            placed(),
            ChildOf(pane),
        ))
        .id()
}

/// The cells a row's text has in a list scrolled in `scroll` over `area`.
pub fn row_width(scroll: ScrollArea, area: ComputedWidgetArea) -> u16 {
    scroll
        .content_width(area.0.width)
        .saturating_sub(CURSOR_COLS)
}

/// Replaces `list`'s rows with a row per line each of `texts` wraps to at
/// `width` - one too wide running on to indented lines - each `tag`ged
/// with the place of the text it comes from; the cursor on the first.
pub fn fill_wrapped(
    commands: &mut Commands,
    (list, width): (Entity, u16),
    texts: impl IntoIterator<Item = (String, Style)>,
    mut tag: impl FnMut(usize, &mut EntityCommands),
) {
    commands.entity(list).despawn_related::<Children>();
    let mut first = None;
    for (at, (text, style)) in texts.into_iter().enumerate() {
        for line in wrapped(&text, width, CONTINUED) {
            let mut row = commands.spawn((list_item(Line::styled(line, style)), ChildOf(list)));
            tag(at, &mut row);
            first.get_or_insert(row.id());
        }
    }
    commands.entity(list).insert(ActiveDescendant(first));
}
