//! The Changes pane: what the highlighted plan changes of the baseline,
//! a row per change in the words an issue is read in.

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Local, Query, With};
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use retiretui_engine::plan::diff;

use super::Plans;
use super::plans::Cursor;
use crate::commands::tui::edit::change_words;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout;
use crate::commands::tui::pane::{Framed, Pane};

const TITLE: &str = "Changes";
/// The narrowest the pane gets, borders included; otherwise it takes a
/// third of the row, the Plans table the rest.
const LEAST_COLS: f32 = 40.0;
pub(super) const THE_BASELINE: &str = "The baseline.";
pub(super) const THE_SAME: &str = "Same as the baseline.";

#[derive(Component)]
pub(crate) struct ChangesList;

/// The pane, beside the Plans pane in `row`.
pub(super) fn spawn(commands: &mut Commands, row: Entity) {
    let pane = Pane::new(TITLE)
        .sharing(1.0)
        .at_least_wide(LEAST_COLS)
        .spawn(commands, row);
    let list = layout::spawn_scrolled_list(commands, pane, Hints(&[("↑↓", "scroll")]));
    commands.entity(list).insert(ChangesList);
}

/// Rewrites the rows whenever a plan, the highlighted one, the baseline
/// or the pane's width moves; a change too wide for the pane runs on to
/// indented lines, each a row of its own.
pub(super) fn refresh_changes(
    (plans, cursor): (Plans, Cursor),
    mut shown: Local<Option<((usize, usize), u16)>>,
    lists: Query<(Entity, &ChildOf, &ScrollArea, &ComputedWidgetArea), With<ChangesList>>,
    mut frames: Query<&mut Framed>,
    mut commands: Commands,
) {
    let places = (cursor.place(), plans.compared.baseline_at());
    let is_stale = plans.is_plan_changed();
    for (list, pane, scroll, area) in &lists {
        let width = layout::row_width(*scroll, *area);
        if *shown == Some((places, width)) && !is_stale {
            continue;
        }
        *shown = Some((places, width));
        let texts = (lines(&plans, places).into_iter()).map(|line| (line.to_string(), line.style));
        layout::fill_wrapped(&mut commands, (list, width), texts, |_, _| {});
        if let Ok(mut framed) = frames.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &title(&plans, places));
        }
    }
}

/// "Changes", and of what against what.
fn title(plans: &Plans, (place, baseline): (usize, usize)) -> String {
    let name = |at: usize| plans.each().nth(at).map(|(name, _)| name);
    match (name(place), name(baseline)) {
        (Some(name), Some(against)) if place != baseline => {
            format!("{TITLE} · {name} against {against}")
        }
        (Some(name), _) => format!("{TITLE} · {name}"),
        _ => TITLE.to_owned(),
    }
}

/// A line per change of the plan at `place` against the one at
/// `baseline`; one saying so where it is the baseline or the same.
fn lines(plans: &Plans, (place, baseline): (usize, usize)) -> Vec<Line<'static>> {
    let dimmed = plans.theme.dimmed();
    let said = |words: &str| vec![Line::styled(words.to_owned(), dimmed)];
    if place == baseline {
        return said(THE_BASELINE);
    }
    let plan_at = |at: usize| plans.each_plan().nth(at);
    let (Some(base), Some(other)) = (plan_at(baseline), plan_at(place)) else {
        return Vec::new();
    };
    match diff(base, other) {
        Ok(changes) if changes.is_empty() => said(THE_SAME),
        Ok(changes) => changes
            .iter()
            .map(|change| Line::from(change_words(change, (base, other))))
            .collect(),
        Err(error) => vec![Line::styled(error.to_string(), plans.theme.exceeded())],
    }
}
