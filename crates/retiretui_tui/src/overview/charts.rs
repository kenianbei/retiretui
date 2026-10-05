//! The Overview's chart, which `v` turns between the balances by tax
//! treatment, net worth, income against taxes, and net worth through
//! random markets, with the key under it: what each mark is, and under
//! the pointer what each reads that year. ⏎ and a press open the Ledger.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, Res, ResMut, Resource, With};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Style;
use plurimus::ui::{ComputedWidgetArea, PointerPress};
use retiretui_client::ledger::salary_marks;
use retiretui_client::overview::CHARTS;
use retiretui_client::present::treatment_word;
use retiretui_engine::market::Runs;
use retiretui_engine::plan::{Dollars, TreatmentClass};
use retiretui_engine::project::{ClassTotals, Projection};

use super::key::{ChartKey, Keyed, line_swatch, swatch};
use crate::chart::{KeyedBeside, Mark, Series, SeriesChart, Shade};
use crate::command::Outcome;
use crate::edit::Turn;
use crate::hints::Hints;
use crate::layout::{filling, fixed, placed};
use crate::nav::{FocusStop, Page};
use crate::pane::{Framed, Pane};
use crate::present;
use crate::session::{LedgerRun, Projected, Shown};
use crate::success::Successes;
use crate::table::basis_amount;
use crate::theme::Theme;
use crate::tools::markets::chart::{INNER, INNER_BAND, OUTER, OUTER_BAND, PLANNED, band, line};
use crate::tools::{EnterRuns, handle_enter};

/// Which of the theme's series colours each dataset is drawn in.
const WORTH_SERIES: usize = 0;
const INCOME_SERIES: usize = 1;
const TAXES_SERIES: usize = 2;

/// What the chart shows.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum View {
    #[default]
    Balances,
    NetWorth,
    IncomeTaxes,
    Markets,
}

impl View {
    const ALL: [Self; 4] = [
        Self::Balances,
        Self::NetWorth,
        Self::IncomeTaxes,
        Self::Markets,
    ];

    fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }

    const fn title(self) -> &'static str {
        CHARTS[self as usize]
    }
}

#[derive(Resource, Default)]
pub(crate) struct ChartView(pub(crate) View);

#[derive(Component)]
pub(crate) struct OverviewChart;

pub(super) fn spawn(commands: &mut Commands, band: Entity, share: f32) {
    let pane = Pane::new(View::default().title())
        .sharing(share)
        .spawn(commands, band);
    let ledger = Page::Ledger.label();
    commands
        .spawn((
            OverviewChart,
            KeyedBeside,
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
    view.0 = view.0.next();
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
        || (view.0 == View::Markets && successes.is_changed());
    if !is_drawn {
        return;
    }
    let (projection, nominal) = (&shown.projected.projection, shown.basis.nominal);
    let (drawn, keyed) = match view.0 {
        View::Balances => balances(projection, nominal, &theme),
        View::NetWorth => lines(vec![net_worth(projection, nominal, &theme)], &theme),
        View::IncomeTaxes => lines(income_taxes(projection, nominal, &theme), &theme),
        View::Markets => match successes.runs(&shown.projected.plan) {
            Some(runs) => markets(runs, &theme),
            None => (SeriesChart::default(), Vec::new()),
        },
    };
    // The runs are kept in today's dollars alone.
    let basis = present::basis_name(nominal && view.0 != View::Markets);
    let title = format!("{} · {basis}", view.0.title());
    let marks = marks(&shown.projected, &theme);
    for (mut chart, pane) in &mut charts {
        *chart = SeriesChart {
            marks: marks.clone(),
            is_legend_hidden: true,
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

/// The tax treatments stacked from the bottom, each shaded in a glyph of
/// its own and read from a year's totals. The HSA, usually the smallest,
/// lies along the axis where no line crosses it.
const CLASSES: [(TreatmentClass, &str, fn(&ClassTotals) -> Dollars); 4] = [
    (TreatmentClass::Hsa, "█", |totals| totals.hsa),
    (TreatmentClass::Deferred, "░", |totals| totals.deferred),
    (TreatmentClass::Roth, "▒", |totals| totals.roth),
    (TreatmentClass::Taxable, "▓", |totals| totals.taxable),
];

/// Each treatment's balance stacked on those under it, a band each in its
/// own colour and glyph, under net worth's line along the top edge, and
/// each keyed by what it holds. A later band wins the row it shares with
/// an earlier one, so the HSA is drawn last and keeps a row wherever it
/// holds anything.
pub(super) fn balances(
    projection: &Projection,
    nominal: bool,
    theme: &Theme,
) -> (SeriesChart, Vec<Keyed>) {
    let mut shades = Vec::with_capacity(CLASSES.len());
    let mut keyed = Vec::with_capacity(CLASSES.len());
    let mut lows: Vec<f64> = vec![0.0; projection.years.len()];
    for (place, (class, symbol, value)) in CLASSES.iter().enumerate() {
        let held = |row: &retiretui_engine::project::YearRow| {
            basis_amount(value(&row.class_totals), row.deflator, nominal)
        };
        let points = (projection.years.iter().zip(&mut lows))
            .map(|(row, low)| {
                let high = *low + held(row) as f64;
                let band = (f64::from(row.year), *low, high);
                *low = high;
                band
            })
            .collect();
        let color = theme.series(place);
        keyed.push(Keyed {
            swatch: swatch(symbol, Style::new().fg(color)),
            label: treatment_word(*class).to_owned(),
            reads: (projection.years.iter())
                .map(|row| (row.year, held(row), held(row)))
                .collect(),
        });
        shades.push(Shade {
            points,
            symbol,
            color,
        });
    }
    shades.rotate_left(1);
    let worth = Series::of("net worth", theme.fg, projection, nominal, |row| {
        row.net_worth
    });
    let chart = SeriesChart::of(vec![worth], theme.dimmed()).shaded(shades);
    (chart, keyed)
}

/// `series` charted as lines, each keyed by its colour.
pub(super) fn lines(series: Vec<Series>, theme: &Theme) -> (SeriesChart, Vec<Keyed>) {
    let keyed = series.iter().map(|series| {
        let style = Style::new().fg(series.color);
        Keyed::of(line_swatch(style), series)
    });
    let keyed = keyed.collect();
    (SeriesChart::of(series, theme.dimmed()), keyed)
}

pub(super) fn income_taxes(projection: &Projection, nominal: bool, theme: &Theme) -> Vec<Series> {
    let income = Series::of(
        "income",
        theme.series(INCOME_SERIES),
        projection,
        nominal,
        |row| row.total_income,
    );
    let taxes = Series::of(
        "taxes",
        theme.series(TAXES_SERIES),
        projection,
        nominal,
        |row| row.taxes.total,
    );
    vec![income, taxes]
}

fn net_worth(projection: &Projection, nominal: bool, theme: &Theme) -> Series {
    Series::of(
        "net worth",
        theme.series(WORTH_SERIES),
        projection,
        nominal,
        |row| row.net_worth,
    )
}

/// The plan's net worth through `runs`, as the Monte Carlo tool charts
/// it: the bands most runs fall in, under the plan's own market.
pub(super) fn markets(runs: &Runs, theme: &Theme) -> (SeriesChart, Vec<Keyed>) {
    let planned = line(runs, PLANNED, &runs.planned, theme);
    let between = |(low, high): (usize, usize), symbol: &str, label: &str| Keyed {
        swatch: swatch(symbol, Style::new().fg(theme.dim)),
        label: label.to_owned(),
        reads: (runs.bands.iter())
            .map(|band| (band.year, band.net_worth[low], band.net_worth[high]))
            .collect(),
    };
    let keyed = vec![
        between(OUTER_BAND, OUTER, "10th to 90th"),
        between(INNER_BAND, INNER, "25th to 75th"),
        Keyed::of(line_swatch(Style::new().fg(planned.color)), &planned),
    ];
    let shades = vec![
        band(runs, OUTER_BAND, OUTER, theme),
        band(runs, INNER_BAND, INNER, theme),
    ];
    let chart = SeriesChart::of(vec![planned], theme.dimmed()).shaded(shades);
    (chart, keyed)
}

/// Where each earner's salary ends.
fn marks(projected: &Projected, theme: &Theme) -> Vec<Mark> {
    let ends = salary_marks(projected).into_iter();
    ends.map(|(label, year)| Mark {
        year,
        label,
        style: theme.dimmed(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use retiretui_client::ledger::salary_ends;

    use super::*;
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
        let marks = marks(projected, &Theme::terminal());
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
        assert!(salary_ends(&unsalaried).is_empty());
        let working =
            projected_from(&TEST_PLAN.replace("end = { age = 60, owner = \"me\" }\n", ""));
        assert_eq!(working.plan.income[0].end, None);
        assert!(salary_ends(&working).is_empty());
    }

    #[test]
    fn series_deflation_and_bounds() {
        let projected = test_projected();
        let projection = &projected.projection;
        let theme = Theme::terminal();
        let (nominal, _) = lines(income_taxes(projection, true, &theme), &theme);
        let (todays, _) = lines(income_taxes(projection, false, &theme), &theme);
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

    #[test]
    fn v_turns_through_the_four_charts_in_the_client_s_order() {
        let mut view = View::default();
        let mut titles = Vec::new();
        for _ in 0..CHARTS.len() {
            titles.push(view.title());
            view = view.next();
        }
        assert_eq!(titles, CHARTS);
        assert_eq!(view, View::default(), "and back to the first");
    }
}
