//! The Overview's chart, which `v` turns between the balances by tax
//! treatment, net worth, income against taxes, and net worth through
//! random markets, with the key under it: what each mark is, and under
//! the pointer what each reads that year. ⏎ and a press open the Ledger.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, Res, ResMut, Resource, With};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Color;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Span;
use plurimus::ui::{ComputedWidgetArea, PointerPress};
use retiretui_client::overview::{Chart, ChartLine, ChartMark, Charted, Spread, band_words};
use retiretui_engine::market::Band;

use super::key::{ChartKey, Keyed, line_swatch, swatch};
use crate::chart::{Legend, Mark, Series, SeriesChart, Shade};
use crate::command::Outcome;
use crate::edit::Turn;
use crate::hints::Hints;
use crate::layout::{filling, fixed, placed};
use crate::nav::{FocusStop, Page};
use crate::pane::{Framed, Pane};
use crate::present;
use crate::session::{LedgerRun, Shown};
use crate::success::Successes;
use crate::theme::Theme;
use crate::tools::markets::chart::{INNER, OUTER, band};
use crate::tools::{EnterRuns, handle_enter};

/// Which of the theme's series colours each chart's lines start at:
/// net worth's is its own, and income's follows it.
const WORTH_SERIES: usize = 0;
pub(super) const INCOME_SERIES: usize = 1;

/// The chart on show.
#[derive(Resource, Default)]
pub(crate) struct ChartView(pub(crate) Chart);

#[derive(Component)]
pub(crate) struct OverviewChart;

pub(super) fn spawn(commands: &mut Commands, band: Entity, share: f32) {
    let pane = Pane::new(Chart::default().title())
        .sharing(share)
        .spawn(commands, band);
    let ledger = Page::Ledger.label();
    commands
        .spawn((
            OverviewChart,
            SeriesChart::default(),
            FocusStop,
            EnterRuns(ledger),
            Hints(&[("⏎", "ledger")]),
            filling(),
            ChildOf(pane),
        ))
        .observe(handle_enter)
        .observe(open_ledger);
    commands.spawn((
        ChartKey::default(),
        UiWidget::default(),
        fixed(1.0),
        placed(),
        ChildOf(pane),
    ));
}

/// The `overview-chart` command: the chart's next view.
pub(crate) fn cycle_chart(mut view: ResMut<ChartView>) -> Outcome {
    let turned = Chart::ALL.iter().position(|&chart| chart == view.0);
    view.0 = Chart::ALL[turned.map_or(0, |at| (at + 1) % Chart::ALL.len())];
    Outcome::Done
}

/// A press on a year of the chart opens the Ledger there: the press has
/// set the year, as it does on every chart.
fn open_ledger(
    press: On<PointerPress>,
    charts: Query<(&SeriesChart, &ComputedWidgetArea)>,
    mut run: ResMut<LedgerRun>,
    mut turn: Turn,
) {
    let Ok((chart, area)) = charts.get(press.entity) else {
        return;
    };
    if chart.year_at(area.0, press.position).is_none() {
        return;
    }
    if run.0.is_some() {
        run.0 = None;
    }
    turn.to(Page::Ledger, None);
}

/// Redraws the chart whenever the plan, the basis, the theme or the view
/// moves, or the markets answer while they are on show, naming the view
/// and its dollars in the pane's title.
pub(super) fn refresh(
    (shown, theme, view, successes): (Shown, Res<Theme>, Res<ChartView>, Res<Successes>),
    mut charts: Query<(&mut SeriesChart, &ChildOf), With<OverviewChart>>,
    mut keys: Query<&mut ChartKey>,
    mut frames: Query<&mut Framed>,
) {
    let is_drawn = shown.projected.is_changed()
        || shown.basis.is_changed()
        || theme.is_changed()
        || view.is_changed()
        || (view.0 == Chart::Markets && successes.is_changed());
    if !is_drawn {
        return;
    }
    let nominal = shown.basis.nominal;
    let charted = view.0.charted(&shown.projected, nominal);
    let (drawn, keyed) = match view.0 {
        Chart::Balances => balances(&charted, &theme),
        Chart::NetWorth => lines(&charted.lines, WORTH_SERIES, &theme),
        Chart::IncomeTaxes => lines(&charted.lines, INCOME_SERIES, &theme),
        Chart::Markets => match successes.bands(&shown.projected.plan) {
            Some(bands) => markets(bands, &theme),
            None => (SeriesChart::default(), Vec::new()),
        },
    };
    // The runs are kept in today's dollars alone.
    let basis = present::basis_name(nominal && view.0 != Chart::Markets);
    let title = format!("{} · {basis}", view.0.title());
    let marks = marks(&charted.marks, &theme);
    for (mut chart, pane) in &mut charts {
        *chart = SeriesChart {
            marks: marks.clone(),
            legend: Legend::Keyed,
            ..drawn.clone()
        };
        if let Ok(mut framed) = frames.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &title);
        }
    }
    let waits_on = if keyed.is_empty() {
        successes.of(&shown.projected.plan).text()
    } else {
        String::new()
    };
    for mut key in &mut keys {
        key.keyed.clone_from(&keyed);
        key.waits_on.clone_from(&waits_on);
    }
}

/// The glyph each treatment is shaded in by its place in the stack, from
/// the bottom, so that neighbours read apart without colour.
const GLYPHS: [&str; 4] = ["▓", "░", "▒", "█"];

/// `line` as a chart draws it, in `color`.
fn series(line: &ChartLine, color: Color) -> Series {
    Series {
        label: line.label.to_owned(),
        color,
        points: (line.points.iter())
            .map(|&(year, amount)| (f64::from(year), amount as f64))
            .collect(),
    }
}

/// Each treatment's balance stacked on those under it, a band each in its
/// own colour and glyph, under net worth's line along the top edge, and
/// each keyed by what it holds. A later band wins the row it shares with
/// an earlier one, so the top one keeps a row wherever it holds anything.
pub(super) fn balances(charted: &Charted, theme: &Theme) -> (SeriesChart, Vec<Keyed>) {
    let years = charted.stacked.first().map_or(0, |line| line.points.len());
    let mut lows = vec![0.0_f64; years];
    let mut shades = Vec::with_capacity(charted.stacked.len());
    let mut keyed = Vec::with_capacity(charted.stacked.len() + charted.lines.len());
    for (place, line) in charted.stacked.iter().enumerate() {
        let (color, symbol) = (theme.series(place), GLYPHS[place % GLYPHS.len()]);
        let points = (line.points.iter().zip(&mut lows))
            .map(|(&(year, amount), low)| {
                let high = *low + amount as f64;
                let band = (f64::from(year), *low, high);
                *low = high;
                band
            })
            .collect();
        keyed.push(Keyed::of(swatch(symbol, Style::new().fg(color)), line));
        shades.push(Shade {
            points,
            symbol,
            color,
        });
    }
    let worth: Vec<Series> = (charted.lines.iter())
        .map(|line| series(line, theme.fg))
        .collect();
    keyed.extend(
        (charted.lines.iter()).map(|line| Keyed::of(line_swatch(Style::new().fg(theme.fg)), line)),
    );
    let chart = SeriesChart::of(worth, theme.dimmed()).shaded(shades);
    (chart, keyed)
}

/// `charted` as lines in the theme's series colours from `first` on, each
/// keyed by its colour.
pub(super) fn lines(
    charted: &[ChartLine],
    first: usize,
    theme: &Theme,
) -> (SeriesChart, Vec<Keyed>) {
    let colored = charted
        .iter()
        .enumerate()
        .map(|(place, line)| (line, theme.series(first + place)));
    let keyed = (colored.clone())
        .map(|(line, color)| Keyed::of(line_swatch(Style::new().fg(color)), line))
        .collect();
    let drawn = colored.map(|(line, color)| series(line, color)).collect();
    (SeriesChart::of(drawn, theme.dimmed()), keyed)
}

/// The plan's net worth through random markets: the `bands` most runs
/// fall in, as the Monte Carlo tool shades them, under the median's line.
pub(super) fn markets(bands: &[Band], theme: &Theme) -> (SeriesChart, Vec<Keyed>) {
    let words = band_words();
    let color = theme.series(WORTH_SERIES);
    let median = Series {
        label: words.median.label.clone(),
        color,
        points: (bands.iter())
            .map(|band| {
                (
                    f64::from(band.year),
                    band.net_worth[words.median.low] as f64,
                )
            })
            .collect(),
    };
    let keyed = |spread: &Spread, swatch: Span<'static>| Keyed {
        swatch,
        label: spread.label.clone(),
        reads: (bands.iter())
            .map(|band| {
                (
                    band.year,
                    band.net_worth[spread.low],
                    band.net_worth[spread.high],
                )
            })
            .collect(),
    };
    let dim = Style::new().fg(theme.dim);
    let keyed = vec![
        keyed(&words.outer, swatch(OUTER, dim)),
        keyed(&words.inner, swatch(INNER, dim)),
        keyed(&words.median, line_swatch(Style::new().fg(color))),
    ];
    let shades = vec![
        band(bands, &words.outer, OUTER, theme),
        band(bands, &words.inner, INNER, theme),
    ];
    let chart = SeriesChart::of(vec![median], theme.dimmed()).shaded(shades);
    (chart, keyed)
}

/// Where each earner's salary ends.
fn marks(marks: &[ChartMark], theme: &Theme) -> Vec<Mark> {
    (marks.iter())
        .map(|mark| Mark {
            year: mark.year,
            label: mark.label.clone(),
            style: theme.dimmed(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use retiretui_client::ledger::salary_ends;

    use super::*;
    use crate::session::Projected;
    use crate::support::{TEST_PLAN, projected_from, test_projected};

    const SECOND_EARNER: &str = r#"
[[household.people]]
id = "you"
birth = 1980-01-01

[[income]]
id = "wages"
kind = "salary"
owner = "you"
amount = 50000
end = { date = 2034-12-31 }

[[income]]
id = "bonus"
kind = "salary"
owner = "you"
amount = 5000
end = { date = 2030-12-31 }
"#;

    fn labels(projected: &Projected) -> Vec<String> {
        let charted = Chart::NetWorth.charted(projected, false);
        let marks = marks(&charted.marks, &Theme::terminal());
        marks.into_iter().map(|mark| mark.label).collect()
    }

    #[test]
    fn a_salary_ends_the_year_after_it_last_pays() {
        let alone = test_projected();
        let end = salary_ends(&alone)[0].1;
        let paid = |year: i16| {
            let row = alone.projection.row(year);
            row.unwrap().income.contains_key("salary")
        };
        assert!(paid(end - 1) && !paid(end), "{end}");
        assert_eq!(labels(&alone), [format!("retire {end}")]);
    }

    #[test]
    fn each_earner_is_marked_by_name_at_their_last_salary() {
        let pair = projected_from(&format!("{TEST_PLAN}{SECOND_EARNER}"));
        let end = salary_ends(&test_projected())[0].1;
        assert_eq!(salary_ends(&pair), [("you", 2035), ("me", end)]);
        assert_eq!(labels(&pair)[0], "you 2035");
    }

    #[test]
    fn no_salary_and_a_salary_paid_to_the_horizon_mark_nothing() {
        let mut unsalaried = test_projected();
        unsalaried.plan.income.clear();
        assert_eq!(salary_ends(&unsalaried), []);
        let working =
            projected_from(&TEST_PLAN.replace("end = { age = 60, owner = \"me\" }\n", ""));
        assert_eq!(working.plan.income[0].end, None);
        assert_eq!(salary_ends(&working), []);
    }

    #[test]
    fn series_deflation_and_bounds() {
        let projected = test_projected();
        let projection = &projected.projection;
        let theme = Theme::terminal();
        let drawn = |nominal| {
            let charted = Chart::IncomeTaxes.charted(&projected, nominal);
            lines(&charted.lines, INCOME_SERIES, &theme).0
        };
        let (nominal, todays) = (drawn(true), drawn(false));
        let first_year = f64::from(projection.years.first().unwrap().year);
        let last_year = f64::from(projection.years.last().unwrap().year);
        assert!((nominal.x_bounds[0] - first_year).abs() < f64::EPSILON);
        assert!((nominal.x_bounds[1] - last_year).abs() < f64::EPSILON);
        assert_eq!(nominal.series.len(), 2);
        // Year one has deflator 1.0, so the bases agree there and diverge
        // once inflation accrues; index 5 is still inside the salary window.
        assert_eq!(nominal.series[0].points[0], todays.series[0].points[0]);
        let later = nominal.series[0].points[5];
        let later_deflated = todays.series[0].points[5];
        assert!(later.1 > later_deflated.1);
        assert!(nominal.y_bounds[1] >= later.1);
    }
}
