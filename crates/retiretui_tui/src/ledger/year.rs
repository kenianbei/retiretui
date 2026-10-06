//! The cursor year's own pane: the milestones that fall in it, what to do
//! and what to watch, and how far the plan has come - a line each, the
//! pane as tall as they are - under the title the client gives the year.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, IntoScheduleConfigs, Local, Query, With};
use bevy_ecs::system::SystemParam;
use bevy_ui::{Node, Val};
use plurimus::core::ratatui_core::style::Style;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use retiretui_client::ledger::Year;

use super::arrange::DetailStop;
use super::{Detail, LedgerSystems, MILESTONE, WARNING, shows_the_year};
use crate::hints::Hints;
use crate::layout;
use crate::pane::{self, Framed, Pane};
use crate::theme::Theme;

pub(super) fn plugin(app: &mut App) {
    let drawn = refresh.run_if(shows_the_year);
    app.add_systems(Update, drawn.in_set(LedgerSystems::Draw));
}

/// The most lines the pane shows at once, the rest scrolled to: the flows
/// and the money under it are the page's.
const YEAR_MOST: u16 = 6;

/// The list the year's lines are drawn in.
#[derive(Component)]
pub(super) struct YearLines;

pub(super) fn spawn_pane(commands: &mut Commands, detail: Entity) {
    let pane = Pane::new("")
        .tall(f32::from(pane::BORDERS))
        .spawn(commands, detail);
    let list = layout::spawn_scrolled_list(commands, pane, Hints(&[("↑↓", "scroll")]));
    commands.entity(list).insert((YearLines, DetailStop));
}

/// The year's lines, each in what it is drawn in: a milestone in the
/// accent, a warning as a figure past its limit, how far the plan has
/// come dimmed.
fn lines(year: &Year, theme: &Theme) -> Vec<(String, Style)> {
    let milestones = year.milestones.iter();
    let milestones = milestones.map(|each| (format!("{MILESTONE} {each}"), theme.accented()));
    let to_do = year.to_do.iter().map(|each| (each.clone(), Style::new()));
    let warnings = year.warnings.iter();
    let warnings = warnings.map(|each| (format!("{WARNING} {each}"), theme.exceeded()));
    let so_far = year
        .so_far
        .iter()
        .map(|said| (said.clone(), theme.dimmed()));
    milestones
        .chain(to_do)
        .chain(warnings)
        .chain(so_far)
        .collect()
}

/// The year's list and the pane it is in.
#[derive(SystemParam)]
struct YearPane<'w, 's> {
    lists: Query<
        'w,
        's,
        (
            Entity,
            &'static ChildOf,
            &'static ScrollArea,
            &'static ComputedWidgetArea,
        ),
        With<YearLines>,
    >,
    panes: Query<'w, 's, (&'static mut Framed, &'static mut Node)>,
}

/// Rewrites the lines whenever the year said or the pane's width moves,
/// and makes the pane as tall as they are.
fn refresh(
    detail: Detail,
    mut drawn: Local<Option<u16>>,
    mut pane: YearPane,
    mut commands: Commands,
) {
    let Ok((list, parent, scroll, area)) = pane.lists.single() else {
        return;
    };
    let width = layout::row_width(*scroll, *area);
    let is_resized = drawn.replace(width) != Some(width);
    let Some((year, theme)) = detail.due(is_resized) else {
        return;
    };
    let lines = layout::fill_wrapped(&mut commands, (list, width), lines(year, theme));
    let shown_lines = u16::try_from(lines).unwrap_or(u16::MAX).min(YEAR_MOST);
    let Ok((mut framed, mut node)) = pane.panes.get_mut(parent.parent()) else {
        return;
    };
    Framed::retitle(&mut framed, &year.title);
    let height = Val::Px(f32::from(shown_lines + pane::BORDERS));
    if node.height != height {
        node.height = height;
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::ledger::Asked;
    use retiretui_engine::params::TaxTables;

    use super::*;
    use crate::support::{TEST_PLAN, projected_from};

    const FULL: &str = include_str!("../../../retiretui_engine/tests/fixtures/full.toml");

    fn year_of(plan_text: &str, year: i16) -> Year {
        let asked = Asked {
            year,
            is_nominal: false,
            run: None,
        };
        Year::new(&projected_from(plan_text), &TaxTables::embedded(), asked).unwrap()
    }

    #[test]
    fn a_year_leads_with_its_milestones_and_ends_on_how_far_the_plan_has_come() {
        let theme = Theme::terminal();
        let said = lines(&year_of(FULL, 2037), &theme);
        let (first, last) = (said.first().unwrap(), said.last().unwrap());
        assert!(first.0.starts_with(MILESTONE), "{said:?}");
        assert_eq!(first.1, theme.accented());
        assert!(last.0.starts_with("So far: "), "{said:?}");
        assert_eq!(last.1, theme.dimmed());
        let plain = said.iter().filter(|(_, style)| *style == Style::new());
        assert_eq!(plain.count(), 5, "the year's five actions: {said:?}");
    }

    #[test]
    fn a_warning_is_marked_and_drawn_as_a_figure_past_its_limit() {
        let theme = Theme::terminal();
        let short = TEST_PLAN.replace("amount = 60000", "amount = 600000");
        let said = lines(&year_of(&short, 2031), &theme);
        let warned = said.iter().find(|(line, _)| line.starts_with(WARNING));
        let (line, style) = warned.expect("a year that runs short");
        assert!(line.contains("Unfunded"), "{line}");
        assert_eq!(*style, theme.exceeded());
    }
}
