//! What the pane under the Plans row shows, which `v` cycles: one metric
//! charted, a grid of four, or the metric year by year.

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, ResMut, Resource, With, Without};
use bevy_ecs::system::SystemParam;
use bevy_ui::{Display, FlexDirection, Node};
use plurimus::ui::ScrollArea;

use super::{Plans, by_year};
use crate::commands::compare::Metric;
use crate::commands::tui::chart::SeriesChart;
use crate::commands::tui::command::Outcome;
use crate::commands::tui::focus::PageFocus;
use crate::commands::tui::layout::{self, filling, growing};
use crate::commands::tui::nav::FocusStop;
use crate::commands::tui::pane::{Framed, Pane};
use crate::commands::tui::present;

/// What the view pane shows.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum View {
    #[default]
    Chart,
    Grid,
    ByYear,
}

impl View {
    fn next(self) -> Self {
        match self {
            Self::Chart => Self::Grid,
            Self::Grid => Self::ByYear,
            Self::ByYear => Self::Chart,
        }
    }
}

/// The four metrics the grid charts at once.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum Set {
    #[default]
    Money,
    Tax,
}

impl Set {
    const fn title(self) -> &'static str {
        match self {
            Self::Money => "Money",
            Self::Tax => "Tax",
        }
    }

    const fn metrics(self) -> [Metric; GRID_CHARTS] {
        match self {
            Self::Money => [
                Metric::NetWorth,
                Metric::Income,
                Metric::Expenses,
                Metric::Withdrawals,
            ],
            Self::Tax => [
                Metric::Taxes,
                Metric::Conversions,
                Metric::Magi,
                Metric::Unfunded,
            ],
        }
    }

    const fn other(self) -> Self {
        match self {
            Self::Money => Self::Tax,
            Self::Tax => Self::Money,
        }
    }
}

const GRID_CHARTS: usize = 4;
const GRID_COLUMNS: usize = 2;

/// What the view pane shows, and of what.
#[derive(Resource, Default)]
pub(crate) struct Charted {
    pub(super) view: View,
    /// The one metric the chart and the by-year table show.
    pub(super) metric: Metric,
    set: Set,
}

impl Charted {
    fn title(&self, basis: &str, year: i16) -> String {
        let named = match self.view {
            View::Grid => self.set.title().to_owned(),
            View::Chart | View::ByYear => format!("{} by year", self.metric.title()),
        };
        format!("{named} · {basis} · {year}")
    }
}

/// One part of the view pane, shown only in its view.
#[derive(Component)]
pub(crate) struct ViewPart(View);

/// The pane the parts share.
#[derive(Component)]
struct ViewPane;

/// The chart of the one metric.
#[derive(Component)]
pub(super) struct CompareChart;

/// One of the grid's charts, by its place in the set.
#[derive(Component)]
pub(super) struct GridChart(usize);

/// The view pane under `view`, the chart shown and the rest hidden.
pub(super) fn spawn(commands: &mut Commands, view: Entity) {
    let pane = Pane::new(Metric::default().title())
        .sharing(1.0)
        .spawn(commands, view);
    commands.entity(pane).insert(ViewPane);
    let hidden = || Node {
        display: Display::None,
        ..filling()
    };
    commands.spawn((
        CompareChart,
        SeriesChart::default(),
        ViewPart(View::Chart),
        FocusStop,
        filling(),
        ChildOf(pane),
    ));
    let grid = Node {
        flex_direction: FlexDirection::Column,
        ..hidden()
    };
    let grid = commands
        .spawn((ViewPart(View::Grid), grid, ChildOf(pane)))
        .id();
    for row in 0..GRID_CHARTS / GRID_COLUMNS {
        let across = Node {
            flex_direction: FlexDirection::Row,
            ..growing()
        };
        let across = commands.spawn((across, ChildOf(grid))).id();
        for column in 0..GRID_COLUMNS {
            let small = Pane::new("").sharing(1.0).spawn(commands, across);
            let place = row * GRID_COLUMNS + column;
            commands.spawn((
                GridChart(place),
                SeriesChart::default(),
                filling(),
                ChildOf(small),
            ));
        }
    }
    let table = by_year::spawn(commands, pane);
    commands
        .entity(table)
        .insert((ViewPart(View::ByYear), hidden()));
}

/// The parts of the view pane, and the frames they are named in.
#[derive(SystemParam)]
pub(super) struct Parts<'w, 's> {
    charts: Query<'w, 's, &'static mut SeriesChart, With<CompareChart>>,
    grid: Query<
        'w,
        's,
        (
            &'static mut SeriesChart,
            &'static GridChart,
            &'static ChildOf,
        ),
        Without<CompareChart>,
    >,
    tables: Query<'w, 's, (Entity, &'static mut ScrollArea), With<by_year::ByYearTable>>,
    panes: Query<'w, 's, Entity, With<ViewPane>>,
    frames: Query<'w, 's, &'static mut Framed>,
}

impl Parts<'_, '_> {
    /// Each of the grid's charts, its pane named for its metric.
    fn draw_grid(&mut self, plans: &Plans) {
        let metrics = plans.charted.set.metrics();
        for (mut chart, GridChart(place), small) in &mut self.grid {
            *chart = plans.chart(metrics[*place]);
            if let Ok(mut framed) = self.frames.get_mut(small.parent()) {
                Framed::retitle(&mut framed, metrics[*place].title());
            }
        }
    }

    fn retitle(&mut self, title: &str) {
        for pane in &self.panes {
            if let Ok(mut framed) = self.frames.get_mut(pane) {
                Framed::retitle(&mut framed, title);
            }
        }
    }
}

/// Redraws the part on show whenever a plan, the basis, the year or the
/// view moves, and names what it shows in the pane's title.
pub(super) fn refresh_views(plans: Plans, mut parts: Parts, mut commands: Commands) {
    if !plans.is_changed() {
        return;
    }
    let charted = &plans.charted;
    match charted.view {
        View::Chart => {
            for mut chart in &mut parts.charts {
                *chart = plans.chart(charted.metric);
            }
        }
        View::Grid => parts.draw_grid(&plans),
        View::ByYear => {
            for (table, mut scroll) in &mut parts.tables {
                by_year::fill(&mut commands, (table, &mut scroll), &plans);
            }
        }
    }
    let basis = present::basis_name(plans.shown.basis.nominal);
    parts.retitle(&charted.title(basis, plans.year()));
}

/// The `compare-view` command: the view pane's next view, its part shown
/// and the rest hidden. The keyboard walks to the part on show, and one
/// held by the part turned from goes with the turn.
pub(crate) fn cycle_view(
    mut charted: ResMut<Charted>,
    mut parts: Query<(Entity, &ViewPart, &mut Node)>,
    mut focus: PageFocus,
    mut commands: Commands,
) -> Outcome {
    charted.view = charted.view.next();
    let holder = focus.holder();
    let mut is_held = false;
    let mut shown = None;
    for (part, ViewPart(view), mut node) in &mut parts {
        let is_shown = *view == charted.view;
        layout::set_display(&mut node, is_shown);
        if is_shown {
            commands.entity(part).insert(FocusStop);
            shown = Some(part);
        } else {
            commands.entity(part).remove::<FocusStop>();
            is_held |= holder == Some(part);
        }
    }
    if is_held {
        focus.set(shown);
    }
    Outcome::Done
}

/// Walks the metrics `step` along, or flips the grid's set.
fn step_metric(charted: &mut Charted, step: isize) {
    match charted.view {
        View::Grid => charted.set = charted.set.other(),
        View::Chart | View::ByYear => charted.metric = charted.metric.neighbor(step),
    }
}

pub(crate) fn next_metric(mut charted: ResMut<Charted>) -> Outcome {
    step_metric(&mut charted, 1);
    Outcome::Done
}

pub(crate) fn previous_metric(mut charted: ResMut<Charted>) -> Outcome {
    step_metric(&mut charted, -1);
    Outcome::Done
}
