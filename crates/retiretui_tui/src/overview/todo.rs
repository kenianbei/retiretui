//! The To do pane: what to do in the cursor's year, and what to watch,
//! as the `actions` command words them, a line each, scrolled through.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Local, Query, Res, With};
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use retiretui_engine::params::TaxTables;

use super::rows::{self, Entry, Tone};
use crate::actions::{collect_warnings, sentence};
use crate::layout;
use crate::pane::Framed;
use crate::present;
use crate::session::{Projected, Session, Shown};
use crate::theme::Theme;

pub(crate) const WARNING_MARK: &str = "! ";
const NOTHING_SCHEDULED: &str = "Nothing to do this year.";
const TITLE: &str = "to do";

#[derive(Component)]
pub(crate) struct TodoList;

pub(super) fn spawn(commands: &mut Commands, band: Entity, share: f32) {
    let list = super::spawn_list(commands, band, TITLE, share);
    commands.entity(list).insert(TodoList);
}

/// The to-dos of `year` in the basis shown, each warning marked and in a
/// warning's tone.
fn entries(projected: &Projected, tables: &TaxTables, (year, nominal): (i16, bool)) -> Vec<Entry> {
    let Some(row) = projected.projection.row(year) else {
        return Vec::new();
    };
    let deflator = (!nominal).then_some(row.deflator);
    let deflating = (!nominal).then_some(&projected.projection);
    let actions = (row.actions.iter())
        .map(|action| Entry::plain(sentence(&projected.plan, action, deflator)));
    let warnings = collect_warnings(&projected.plan, tables, row, deflating)
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

/// Rewrites the rows whenever the year, the basis, the plan, the theme or
/// the pane's width moves, from the first.
pub(super) fn refresh(
    (shown, session, theme): (Shown, Res<Session>, Res<Theme>),
    mut drawn: Local<Option<u16>>,
    lists: Query<(Entity, &ChildOf, &ScrollArea, &ComputedWidgetArea), With<TodoList>>,
    mut frames: Query<&mut Framed>,
    mut commands: Commands,
) {
    let is_stale = shown.projected.is_changed()
        || shown.is_year_changed()
        || shown.basis.is_changed()
        || theme.is_changed();
    for (list, pane, scroll, area) in &lists {
        let width = layout::row_width(*scroll, *area);
        if *drawn == Some(width) && !is_stale {
            continue;
        }
        *drawn = Some(width);
        let year = shown.year();
        let nominal = shown.basis.nominal;
        let entries = entries(&shown.projected, &session.tables, (year, nominal));
        rows::spawn_rows(&mut commands, (list, width), &entries, &theme);
        if let Ok(mut framed) = frames.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &format!("{year} · {TITLE}"));
            Framed::renote(&mut framed, present::basis_name(nominal));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::test_projected;

    fn said(projected: &Projected, year: i16) -> Vec<String> {
        said_in(projected, year, true)
    }

    fn said_in(projected: &Projected, year: i16, nominal: bool) -> Vec<String> {
        let tables = TaxTables::embedded();
        let entries = entries(projected, &tables, (year, nominal));
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
        assert_ne!(
            said_in(&projected, 2045, false),
            drawdown,
            "a later year's amounts follow the basis"
        );
    }
}
