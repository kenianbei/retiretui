//! The cursor year's own panes, side by side. To do: the milestones that
//! fall in the year, what to do in it and what to watch, a line each. So
//! far: what the plan has paid, converted and drawn through it, each beside
//! its lifetime total. The two are as tall as the longer.

use bevy_app::{App, Update};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, Query, With, Without,
};
use bevy_ecs::system::SystemParam;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::style::Style;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use plurimus::widgets::TableColumns;
use retiretui_client::ledger::Year;

use super::arrange::DetailStop;
use super::{Detail, LedgerSystems, MILESTONE, WARNING, shows_the_year};
use crate::edit::table_bundle;
use crate::hints::Hints;
use crate::layout::{self, filling, fixed, placed};
use crate::pane::{self, Framed, Pane};
use crate::tabulate;
use crate::theme::Theme;

pub(super) fn plugin(app: &mut App) {
    let drawn = refresh.run_if(shows_the_year);
    app.add_systems(Update, drawn.in_set(LedgerSystems::Draw));
}

/// The most lines the panes show at once, the rest scrolled to: the flows
/// and the money under them are the page's.
const YEAR_MOST: u16 = 6;
/// The columns So far takes, borders included: its title with the dollars
/// it is in, and the longest of its lines.
const SO_FAR_COLS: f32 = 34.0;

/// The row the two panes stand in.
#[derive(Component)]
struct YearBand;

/// The list the year's to-dos are drawn in.
#[derive(Component)]
struct YearLines;

/// The table of how far the plan has come.
#[derive(Component)]
struct SoFar;

pub(super) fn spawn_panes(commands: &mut Commands, detail: Entity) {
    let row = Node {
        flex_direction: FlexDirection::Row,
        ..fixed(f32::from(pane::BORDERS))
    };
    let row = commands.spawn((row, YearBand, ChildOf(detail))).id();
    let to_do = Pane::new("").sharing(1.0).spawn(commands, row);
    let list = layout::spawn_scrolled_list(commands, to_do, Hints(&[("↑↓", "scroll")]));
    commands.entity(list).insert((YearLines, DetailStop));
    let so_far = Pane::new("").wide(SO_FAR_COLS).spawn(commands, row);
    commands.spawn((
        table_bundle(),
        SoFar,
        DetailStop,
        Hints(&[("↑↓", "line")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(so_far),
    ));
}

/// The year's lines, each in what it is drawn in: a milestone in the
/// accent, a warning as a figure past its limit.
fn lines(year: &Year, theme: &Theme) -> Vec<(String, Style)> {
    let milestones = year.milestones.iter();
    let milestones = milestones.map(|each| (format!("{MILESTONE} {each}"), theme.accented()));
    let to_do = year.to_do.iter().map(|each| (each.clone(), Style::new()));
    let warnings = year.warnings.iter();
    let warnings = warnings.map(|each| (format!("{WARNING} {each}"), theme.exceeded()));
    milestones.chain(to_do).chain(warnings).collect()
}

/// The two panes' widgets, their frames and the row they stand in.
#[derive(SystemParam)]
struct YearPanes<'w, 's> {
    lists: Query<
        'w,
        's,
        (
            Entity,
            &'static ChildOf,
            &'static ScrollArea,
            &'static ComputedWidgetArea,
        ),
        (With<YearLines>, Without<SoFar>),
    >,
    tables: Query<'w, 's, (Entity, &'static ChildOf, &'static mut ScrollArea), With<SoFar>>,
    frames: Query<'w, 's, &'static mut Framed>,
    rows: Query<'w, 's, &'static mut Node, With<YearBand>>,
}

/// Rewrites both panes whenever the year said or the list's width moves,
/// and makes their row as tall as the longer of them.
fn refresh(
    detail: Detail,
    mut drawn: Local<Option<u16>>,
    mut panes: YearPanes,
    mut commands: Commands,
) {
    let Ok((list, to_do, scroll, area)) = panes.lists.single() else {
        return;
    };
    let width = layout::row_width(*scroll, *area);
    let is_resized = drawn.replace(width) != Some(width);
    let Some((year, theme)) = detail.due(is_resized) else {
        return;
    };
    let wrapped = layout::fill_wrapped(&mut commands, (list, width), lines(year, theme));
    if let Ok(mut framed) = panes.frames.get_mut(to_do.parent()) {
        Framed::retitle(&mut framed, &year.title);
    }
    let so_far = year.so_far.iter();
    let so_far: Vec<Vec<String>> = so_far
        .map(|line| vec![line.label.clone(), line.amount.clone()])
        .collect();
    if let Ok((table, pane, mut scroll)) = panes.tables.single_mut() {
        let widest = so_far.iter().map(|line| line[1].chars().count()).max();
        let widest = u16::try_from(widest.unwrap_or(0)).unwrap_or(u16::MAX);
        let columns = vec![Constraint::Fill(1), Constraint::Length(widest)];
        commands.entity(table).insert(TableColumns(columns));
        tabulate::refill(&mut commands, (table, &mut *scroll), (&[], &so_far), &[0]);
        if let Ok(mut framed) = panes.frames.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &year.so_far_title);
        }
    }
    let longer = wrapped.max(so_far.len());
    let shown = u16::try_from(longer).unwrap_or(u16::MAX).min(YEAR_MOST);
    let height = Val::Px(f32::from(shown + pane::BORDERS));
    for mut node in &mut panes.rows {
        if node.height != height {
            node.height = height;
        }
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
    fn a_year_leads_with_its_milestones_over_what_to_do() {
        let theme = Theme::terminal();
        let said = lines(&year_of(FULL, 2037), &theme);
        let first = said.first().unwrap();
        assert!(first.0.starts_with(MILESTONE), "{said:?}");
        assert_eq!(first.1, theme.accented());
        let plain = said.iter().filter(|(_, style)| *style == Style::new());
        assert_eq!(plain.count(), 5, "the year's five actions: {said:?}");
        let has_totals = said.iter().any(|(line, _)| line.starts_with("So far"));
        assert!(!has_totals, "how far the plan has come is its own pane");
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
