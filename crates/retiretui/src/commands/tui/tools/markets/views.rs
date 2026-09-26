//! What the chart pane under a market tool's runs shows, which `v` cycles:
//! the bands, net worth year by year at each percentile, the share of runs
//! still funded, and how the runs end. Historical has no by-year table:
//! percentiles over overlapping histories claim a precision they do not
//! have.

use std::marker::PhantomData;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Query, Res, ResMut, Resource,
};
use bevy_ui::{Display, Node};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::layout::Direction;
use plurimus::core::ratatui_core::style::Style;
use plurimus::ui::ScrollArea;
use plurimus::widgets::WidgetSystems;
use plurimus::widgets::ratatui_widgets::barchart::{Bar, BarChart};
use retiretui_engine::market::{BAND_PERCENTILES, Runs};
use retiretui_engine::plan::Dollars;

use super::{MarketTool, count_text};
use crate::commands::tui::chart::{Series, SeriesChart};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::edit::table_bundle;
use crate::commands::tui::layout::{self, filling, placed};
use crate::commands::tui::nav::Page;
use crate::commands::tui::present::{compact_dollars, rate};
use crate::commands::tui::tabulate;
use crate::commands::tui::theme::Theme;
use crate::commands::tui::tools::Tool;

/// What the chart pane shows.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum View {
    #[default]
    Bands,
    ByYear,
    StillFunded,
    Endings,
}

impl View {
    /// The view `v` turns to next; `has_by_year` is whether the page has
    /// the by-year table.
    fn next(self, has_by_year: bool) -> Self {
        match self {
            Self::Bands if has_by_year => Self::ByYear,
            Self::Bands | Self::ByYear => Self::StillFunded,
            Self::StillFunded => Self::Endings,
            Self::Endings => Self::Bands,
        }
    }
}

/// The view a tool's chart pane shows.
#[derive(Resource)]
pub(crate) struct ViewOf<R>(pub View, PhantomData<R>);

impl<R> Default for ViewOf<R> {
    fn default() -> Self {
        Self(View::default(), PhantomData)
    }
}

/// One part of a tool's chart pane, shown only in its view.
#[derive(Component)]
pub(super) struct ViewPart(pub Page, pub View);

pub(super) const BANDS_TITLE: &str = "Net Worth";
const BY_YEAR_TITLE: &str = "By Year · today's dollars";
const FUNDED_TITLE: &str = "Still Funded · share of runs";
const FUNDED_SERIES: usize = 0;
const PERCENT: f64 = 100.0;
const SHORT: &str = "short";
/// Where the ending buckets break, in today's dollars: under the first,
/// between each two, and over the last.
const ENDING_BREAKS: [Dollars; 4] = [1_000_000, 2_000_000, 4_000_000, 8_000_000];
const GAP: u16 = 1;

pub(super) fn install<R: MarketTool>(app: &mut App) {
    app.init_resource::<ViewOf<R>>();
    app.add_systems(
        Update,
        (show_view::<R>, refresh_views::<R>).before(WidgetSystems::Layout),
    );
}

/// The parts the bands' chart shares its pane with, each hidden until its
/// view is shown.
pub(super) fn spawn_parts(commands: &mut Commands, framed: Entity, page: Page) {
    let hidden = || Node {
        display: Display::None,
        ..filling()
    };
    commands.spawn((
        table_bundle(),
        ViewPart(page, View::ByYear),
        layout::Rests,
        hidden(),
        placed(),
        ChildOf(framed),
    ));
    commands.spawn((
        SeriesChart::default(),
        ViewPart(page, View::StillFunded),
        hidden(),
        ChildOf(framed),
    ));
    commands.spawn((
        UiWidget::default(),
        ViewPart(page, View::Endings),
        hidden(),
        placed(),
        ChildOf(framed),
    ));
}

/// The `*-view` commands: the chart pane's next view.
pub(crate) fn cycle_view<R: MarketTool>(mut view: ResMut<ViewOf<R>>) -> Outcome {
    view.0 = view.0.next(R::HAS_BY_YEAR);
    Outcome::Done
}

/// Shows the view's part and hides the rest.
fn show_view<R: MarketTool>(view: Res<ViewOf<R>>, mut parts: Query<(&ViewPart, &mut Node)>) {
    if !view.is_changed() {
        return;
    }
    for (part, mut node) in &mut parts {
        if part.0 != R::PAGE {
            continue;
        }
        layout::set_display(&mut node, part.1 == view.0);
    }
}

/// The chart pane's title under `view`, `label` naming the run the bands'
/// line follows.
pub(super) fn title(view: View, runs: &Runs, label: &str) -> String {
    match view {
        View::Bands => format!("{BANDS_TITLE} · {label}"),
        View::ByYear => BY_YEAR_TITLE.to_owned(),
        View::StillFunded => FUNDED_TITLE.to_owned(),
        View::Endings => format!("Ends With · {} runs", count_text(runs.runs.len())),
    }
}

/// Fills the part on show from what the search found, as it is turned to:
/// a table given its cursor while hidden scrolls that row into no area and
/// is shown a row down, its header gone.
fn refresh_views<R: MarketTool>(
    (tool, theme, view): (Res<Tool<R>>, Res<Theme>, Res<ViewOf<R>>),
    mut parts: Query<(Entity, &ViewPart, Option<&mut ScrollArea>)>,
    mut funded: Query<&mut SeriesChart>,
    mut endings: Query<&mut UiWidget>,
    mut commands: Commands,
) {
    if !tool.is_changed() && !theme.is_changed() && !view.is_changed() {
        return;
    }
    let Some(found) = tool.found() else {
        return;
    };
    let runs = found.runs();
    for (part, shown, scroll) in &mut parts {
        if shown.0 != R::PAGE || shown.1 != view.0 {
            continue;
        }
        match (shown.1, scroll) {
            (View::ByYear, Some(mut scroll)) => {
                fill_by_year(&mut commands, (part, &mut scroll), runs);
            }
            (View::StillFunded, _) => {
                if let Ok(mut chart) = funded.get_mut(part) {
                    *chart = still_funded(runs, &theme);
                }
            }
            (View::Endings, _) => {
                if let Ok(mut widget) = endings.get_mut(part) {
                    *widget = UiWidget::new(endings_chart(runs, &theme));
                }
            }
            _ => {}
        }
    }
}

/// Net worth at each percentile, and the share still funded, year by year.
fn fill_by_year(commands: &mut Commands, (table, scroll): (Entity, &mut ScrollArea), runs: &Runs) {
    let mut header = vec!["Year".to_owned()];
    header.extend(
        BAND_PERCENTILES
            .iter()
            .map(|percentile| format!("{percentile}th")),
    );
    header.push("Funded".to_owned());
    let rows: Vec<Vec<String>> = runs
        .bands
        .iter()
        .map(|band| {
            let mut cells = vec![band.year.to_string()];
            cells.extend(band.net_worth.iter().map(|&worth| compact_dollars(worth)));
            cells.push(rate(band.funded));
            cells
        })
        .collect();
    let columns = tabulate::gapped_columns((&header, &rows), GAP);
    commands.entity(table).insert(columns);
    tabulate::refill(commands, (table, scroll), (&header, &rows), &[0]);
}

/// The share of runs not yet short, year by year, as a percent.
fn still_funded(runs: &Runs, theme: &Theme) -> SeriesChart {
    let points = runs
        .bands
        .iter()
        .map(|band| (f64::from(band.year), band.funded * PERCENT))
        .collect();
    let series = Series {
        label: "still funded".to_owned(),
        color: theme.series(FUNDED_SERIES),
        points,
    };
    let mut chart = SeriesChart::of(vec![series], theme.dimmed());
    chart.y_bounds = [0.0, PERCENT];
    chart.y_labels = vec!["0%".to_owned(), "100%".to_owned()];
    chart
}

/// How many runs end in each bucket, those that fell short first.
fn endings_chart(runs: &Runs, theme: &Theme) -> BarChart<'static> {
    let mut counts = vec![0_u64; ENDING_BREAKS.len() + 2];
    for run in &runs.runs {
        let at = if run.first_short.is_some() {
            0
        } else {
            1 + ENDING_BREAKS
                .iter()
                .filter(|&&edge| run.ending >= edge)
                .count()
        };
        counts[at] += 1;
    }
    let bars: Vec<Bar<'static>> = counts
        .iter()
        .enumerate()
        .map(|(at, &count)| {
            let bar = Bar::with_label(bucket_label(at), count).text_value(count.to_string());
            if at == 0 {
                bar.style(theme.exceeded())
            } else {
                bar.style(Style::new().fg(theme.series(FUNDED_SERIES)))
            }
        })
        .collect();
    BarChart::horizontal(bars)
        .direction(Direction::Horizontal)
        .bar_width(1)
        .bar_gap(0)
        .label_style(theme.dimmed())
}

/// The words for bucket `at`: short, then each span between the breaks.
fn bucket_label(at: usize) -> String {
    match at {
        0 => SHORT.to_owned(),
        1 => format!("<{}", compact_dollars(ENDING_BREAKS[0])),
        _ if at > ENDING_BREAKS.len() => format!(
            "{}+",
            compact_dollars(ENDING_BREAKS[ENDING_BREAKS.len() - 1])
        ),
        _ => format!(
            "{}–{}",
            compact_dollars(ENDING_BREAKS[at - 2]),
            compact_dollars(ENDING_BREAKS[at - 1])
        ),
    }
}
