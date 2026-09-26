//! The To do pane: what to do in the cursor's year, and what to watch,
//! as the `actions` command words them, a line each, scrolled through.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Local, Query, Res, With};
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use retiretui_engine::params::TaxTables;

use super::rows::{self, Entry, Tone};
use crate::commands::actions::{collect_warnings, sentence};
use crate::commands::tui::layout;
use crate::commands::tui::pane::Framed;
use crate::commands::tui::session::{Projected, Session, Shown};
use crate::commands::tui::theme::Theme;

pub(crate) const WARNING_MARK: &str = "! ";
const NOTHING_SCHEDULED: &str = "Nothing to do this year.";
/// Actions are instructions, so their amounts are the year's own dollars
/// whatever basis the views are in.
const ACTION_DOLLARS: &str = "in that year's dollars";
const TITLE: &str = "to do";

#[derive(Component)]
pub(crate) struct TodoList;

pub(super) fn spawn(commands: &mut Commands, band: Entity, share: f32) {
    let list = super::spawn_list(commands, band, TITLE, share);
    commands.entity(list).insert(TodoList);
}

/// The to-dos of `year`, each warning marked and in a warning's tone.
fn entries(projected: &Projected, tables: &TaxTables, year: i16) -> Vec<Entry> {
    let years = &projected.projection.years;
    let Some(row) = years.iter().find(|row| row.year == year) else {
        return Vec::new();
    };
    let actions =
        (row.actions.iter()).map(|action| Entry::plain(sentence(&projected.plan, action)));
    let warnings = collect_warnings(&projected.plan, tables, row, None)
        .into_iter()
        .map(|warning| Entry {
            tone: Tone::Warning,
            ..Entry::plain(format!("{WARNING_MARK}{warning}"))
        });
    let mut entries: Vec<Entry> = actions.chain(warnings).collect();
    if entries.is_empty() {
        entries.push(Entry::plain(NOTHING_SCHEDULED.to_owned()));
    }
    entries
}

/// Rewrites the rows whenever the year, the plan, the theme or the
/// pane's width moves, from the first; the basis leaves them, since
/// actions are in the year's own dollars.
pub(super) fn refresh(
    (shown, session, theme): (Shown, Res<Session>, Res<Theme>),
    mut drawn: Local<Option<u16>>,
    lists: Query<(Entity, &ChildOf, &ScrollArea, &ComputedWidgetArea), With<TodoList>>,
    mut frames: Query<&mut Framed>,
    mut commands: Commands,
) {
    let is_stale = shown.projected.is_changed() || shown.is_year_changed() || theme.is_changed();
    for (list, pane, scroll, area) in &lists {
        let width = layout::row_width(*scroll, *area);
        if *drawn == Some(width) && !is_stale {
            continue;
        }
        *drawn = Some(width);
        let year = shown.year();
        let entries = entries(&shown.projected, &session.tables, year);
        rows::spawn_rows(&mut commands, (list, width), &entries, &theme);
        if let Ok(mut framed) = frames.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &format!("{year} · {TITLE}"));
            Framed::renote(&mut framed, ACTION_DOLLARS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::support::test_projected;

    fn said(projected: &Projected, year: i16) -> Vec<String> {
        let tables = TaxTables::embedded();
        let entries = entries(projected, &tables, year);
        entries.into_iter().map(|entry| entry.text).collect()
    }

    #[test]
    fn the_year_s_actions_and_warnings_are_listed() {
        let projected = test_projected();
        let saving = said(&projected, 2026);
        assert!(
            saving
                .iter()
                .any(|line| line.starts_with("Save the unspent $")),
            "{saving:?}"
        );
        let mut quiet = test_projected();
        quiet.projection.years[0].actions.clear();
        assert_eq!(said(&quiet, 2026), [NOTHING_SCHEDULED]);
        let drawdown = said(&projected, 2045);
        assert!(
            drawdown.iter().any(|line| line.starts_with("Withdraw $")),
            "{drawdown:?}"
        );
    }
}
