//! The Ledger's To do band, over the cursor year's detail: what to do in
//! that year, as the `actions` command words it, a line each and as tall
//! as they are. What to watch is said under the flows below it.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, Query, Res, With,
};
use bevy_ui::{Node, Val};
use plurimus::core::ratatui_core::style::Style;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use plurimus::widgets::WidgetSystems;
use retiretui_client::actions::{NOTHING_SCHEDULED, actions_said};
use retiretui_client::ledger::TO_DO;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::YearRow;

use super::super::hints::Hints;
use super::super::layout;
use super::super::pane::{self, Framed, Pane};
use super::super::session::Shown;
use super::super::theme::Theme;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, refresh.before(WidgetSystems::Layout));
}

/// The most lines the band shows at once, the rest scrolled to: the years
/// above it are the page's.
const TODO_MOST: u16 = 4;

#[derive(Component)]
struct TodoList;

pub(super) fn spawn_pane(commands: &mut Commands, parent: Entity) {
    let pane = Pane::new(TO_DO)
        .tall(f32::from(pane::BORDERS))
        .spawn(commands, parent);
    let list = layout::spawn_scrolled_list(commands, pane, Hints(&[("↑↓", "scroll")]));
    commands.entity(list).insert(TodoList);
}

/// What `row` has the household do, or that it is nothing.
fn actions(plan: &Plan, row: &YearRow, nominal: bool) -> Vec<String> {
    let mut said = actions_said(plan, row, nominal);
    if said.is_empty() {
        said.push(NOTHING_SCHEDULED.to_owned());
    }
    said
}

/// Rewrites the rows whenever the year, the basis, the plan shown, the
/// theme or the band's width moves, from the first, and makes the band as
/// tall as they are.
fn refresh(
    (shown, theme): (Shown, Res<Theme>),
    mut drawn: Local<Option<u16>>,
    lists: Query<(Entity, &ChildOf, &ScrollArea, &ComputedWidgetArea), With<TodoList>>,
    mut panes: Query<(&mut Framed, &mut Node)>,
    mut commands: Commands,
) {
    let Some(row) = shown.row() else {
        return;
    };
    for (list, pane, scroll, area) in &lists {
        let width = layout::row_width(*scroll, *area);
        if *drawn == Some(width) && !shown.is_changed() && !theme.is_changed() {
            continue;
        }
        *drawn = Some(width);
        let said = actions(&shown.ledger().plan, row, shown.basis.nominal);
        let lines: Vec<String> = said
            .iter()
            .flat_map(|action| layout::wrapped(action, width, layout::CONTINUED))
            .collect();
        let shown_lines = u16::try_from(lines.len())
            .unwrap_or(u16::MAX)
            .min(TODO_MOST);
        let lines = lines.into_iter().map(|line| (0, line, Style::new()));
        layout::fill_lines(&mut commands, list, lines, |_, _| {});
        if let Ok((mut framed, mut node)) = panes.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &format!("{} {TO_DO}", row.year));
            let height = Val::Px(f32::from(shown_lines + pane::BORDERS));
            if node.height != height {
                node.height = height;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Projected;
    use crate::support::test_projected;

    fn said(projected: &Projected, year: i16, nominal: bool) -> Vec<String> {
        let row = projected.projection.row(year).unwrap();
        actions(&projected.plan, row, nominal)
    }

    #[test]
    fn the_year_s_actions_are_listed_in_the_basis_shown() {
        let projected = test_projected();
        let saving = said(&projected, 2026, true);
        assert!(
            saving
                .iter()
                .any(|line| line.starts_with("Save the unspent $")),
            "{saving:?}"
        );
        let mut quiet = test_projected();
        quiet.projection.years[0].actions.clear();
        assert_eq!(said(&quiet, 2026, true), [NOTHING_SCHEDULED]);
        let drawdown = said(&projected, 2045, true);
        assert!(
            drawdown.iter().any(|line| line.starts_with("Withdraw $")),
            "{drawdown:?}"
        );
        assert_ne!(
            said(&projected, 2045, false),
            drawdown,
            "a later year's amounts follow the basis"
        );
    }
}
