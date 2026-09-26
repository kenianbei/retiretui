//! The chart under a market tool's runs: the spread of net worth across
//! them, the 10th to 90th percentile shaded light and the 25th to 75th
//! darker, and the highlighted run's own line over it.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Entity, IntoScheduleConfigs, Local, Query, Res};
use retiretui_engine::market::{Run, Runs};

use super::views::{View, ViewOf, ViewPart};
use super::{MarketTool, highlighted};
use crate::commands::tui::chart::{Series, SeriesChart, Shade, draw_charts};
use crate::commands::tui::layout::filling;
use crate::commands::tui::pane::{Framed, Pane};
use crate::commands::tui::theme::Theme;
use crate::commands::tui::tools::Tool;

const OUTER: &str = "░";
const INNER: &str = "▒";
/// The bands' places in [`retiretui_engine::market::BAND_PERCENTILES`].
const OUTER_BAND: (usize, usize) = (0, 4);
const INNER_BAND: (usize, usize) = (1, 3);
const RUN_SERIES: usize = 0;
const PLANNED: &str = "As planned";

pub(super) fn install<R: MarketTool>(app: &mut App) {
    app.add_systems(
        Update,
        redraw::<R>
            .after(crate::commands::tui::tools::poll_search::<R>)
            .before(draw_charts),
    );
}

pub(super) fn spawn_pane<R: MarketTool>(commands: &mut Commands, column: Entity) {
    let pane = Pane::new(super::views::BANDS_TITLE)
        .sharing(1.0)
        .spawn(commands, column);
    commands.spawn((
        SeriesChart::default(),
        ViewPart(R::PAGE, View::Bands),
        filling(),
        ChildOf(pane),
    ));
    super::views::spawn_parts(commands, pane, R::PAGE);
}

/// A band between two of the percentiles, year by year.
fn band(runs: &Runs, (low, high): (usize, usize), symbol: &'static str, theme: &Theme) -> Shade {
    Shade {
        points: runs
            .bands
            .iter()
            .map(|band| {
                let (low, high) = (band.net_worth[low], band.net_worth[high]);
                (f64::from(band.year), low as f64, high as f64)
            })
            .collect(),
        symbol,
        color: theme.dim,
    }
}

/// One run's net worth year by year, from the plan's start.
fn line(runs: &Runs, label: &str, run: &Run, theme: &Theme) -> Series {
    let years = runs.bands.iter().map(|band| f64::from(band.year));
    Series {
        label: label.to_owned(),
        color: theme.series(RUN_SERIES),
        points: years
            .zip(&run.net_worth)
            .map(|(year, &worth)| (year, worth as f64))
            .collect(),
    }
}

/// Redraws the bands whenever the search answers, the highlight moves, or
/// a chart is spawned after the answer, and names the view on show in the
/// pane's title; the highlight moves without marking the tool changed.
fn redraw<R: MarketTool>(
    (tool, theme, view): (Res<Tool<R>>, Res<Theme>, Res<ViewOf<R>>),
    mut drawn: Local<Option<usize>>,
    mut charts: Query<(&mut SeriesChart, &ViewPart, &ChildOf)>,
    mut panes: Query<&mut Framed>,
) {
    let at = tool.highlighted;
    let is_new_chart = charts
        .iter_mut()
        .any(|(chart, part, _)| chart.is_added() && part.0 == R::PAGE && part.1 == View::Bands);
    if !(tool.is_changed()
        || theme.is_changed()
        || view.is_changed()
        || is_new_chart
        || *drawn != Some(at))
    {
        return;
    }
    *drawn = Some(at);
    let Some(found) = tool.found() else {
        return;
    };
    let runs = found.runs();
    let (label, run) = highlighted(&tool).unwrap_or_else(|| (PLANNED.to_owned(), &runs.planned));
    let shades = vec![
        band(runs, OUTER_BAND, OUTER, &theme),
        band(runs, INNER_BAND, INNER, &theme),
    ];
    let title = super::views::title(view.0, runs, &label);
    for (mut chart, part, parent) in &mut charts {
        if part.0 != R::PAGE || part.1 != View::Bands {
            continue;
        }
        *chart = SeriesChart::of(vec![line(runs, &label, run, &theme)], theme.dimmed())
            .shaded(shades.clone());
        if let Ok(mut pane) = panes.get_mut(parent.parent()) {
            Framed::retitle(&mut pane, &title);
        }
    }
}
