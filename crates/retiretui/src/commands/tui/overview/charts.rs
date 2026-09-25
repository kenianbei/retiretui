//! The Overview's chart, which `v` turns between the balances by tax
//! treatment, net worth, and income against taxes, and the years it
//! points out.

use std::collections::BTreeMap;
use std::iter;

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, Query, Res, ResMut, Resource, With};
use bevy_ui::Node;
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use retiretui_engine::plan::{Dollars, IncomeKind};
use retiretui_engine::project::{ClassTotals, Projection};

use crate::commands::table::basis_amount;
use crate::commands::tui::chart::{Mark, Series, SeriesChart, Shade};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::layout::{self, filling, fixed, placed};
use crate::commands::tui::nav::FocusStop;
use crate::commands::tui::pane::{Framed, Pane};
use crate::commands::tui::present;
use crate::commands::tui::session::{Projected, Shown};
use crate::commands::tui::theme::Theme;

/// How many of its glyph each treatment is keyed by.
const KEY_SWATCH: usize = 2;
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
}

impl View {
    fn next(self) -> Self {
        match self {
            Self::Balances => Self::NetWorth,
            Self::NetWorth => Self::IncomeTaxes,
            Self::IncomeTaxes => Self::Balances,
        }
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Balances => "Balances by tax treatment",
            Self::NetWorth => "Net worth",
            Self::IncomeTaxes => "Income against taxes",
        }
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
    commands.spawn((
        OverviewChart,
        SeriesChart::default(),
        FocusStop,
        filling(),
        ChildOf(pane),
    ));
    commands.spawn((
        BalancesKey,
        UiWidget::default(),
        fixed(1.0),
        placed(),
        ChildOf(pane),
    ));
}

/// The line under the chart keying the balances, shown with them alone.
#[derive(Component)]
pub(crate) struct BalancesKey;

/// The `overview-chart` command: the chart's next view.
pub(crate) fn cycle_chart(mut view: ResMut<ChartView>) -> Outcome {
    view.0 = view.0.next();
    Outcome::Done
}

/// Redraws the chart whenever the plan, the basis, the theme or the view
/// moves, naming the view and its dollars in the pane's title; the year
/// moves its marks alone.
pub(super) fn refresh(
    (shown, theme, view): (Shown, Res<Theme>, Res<ChartView>),
    mut charts: Query<(&mut SeriesChart, &ChildOf), With<OverviewChart>>,
    mut keys: Query<(&mut UiWidget, &mut Node), With<BalancesKey>>,
    mut frames: Query<&mut Framed>,
) {
    let is_drawn = shown.projected.is_changed()
        || shown.basis.is_changed()
        || theme.is_changed()
        || view.is_changed();
    if !is_drawn && !shown.is_year_changed() {
        return;
    }
    let marks = marks(&shown.projected, shown.year(), &theme);
    if !is_drawn {
        for (mut chart, _) in &mut charts {
            chart.marks.clone_from(&marks);
        }
        return;
    }
    let (projection, nominal) = (&shown.projected.projection, shown.basis.nominal);
    let drawn = match view.0 {
        View::Balances => balances(projection, nominal, &theme),
        View::NetWorth => net_worth(projection, nominal, &theme),
        View::IncomeTaxes => income_taxes(projection, nominal, &theme),
    };
    let title = format!("{} · {}", view.0.title(), present::basis_name(nominal));
    for (mut chart, pane) in &mut charts {
        *chart = SeriesChart {
            marks: marks.clone(),
            ..drawn.clone()
        };
        if let Ok(mut framed) = frames.get_mut(pane.parent()) {
            Framed::retitle(&mut framed, &title);
        }
    }
    for (mut widget, mut node) in &mut keys {
        *widget = UiWidget::new(Paragraph::new(key(&theme)));
        layout::set_display(&mut node, view.0 == View::Balances);
    }
}

/// The tax treatments stacked from the bottom, each named, shaded in a
/// glyph of its own, and read from a year's totals. The HSA, usually the
/// smallest, lies along the axis where no line crosses it.
const CLASSES: [(&str, &str, fn(&ClassTotals) -> Dollars); 4] = [
    ("HSA", "█", |totals| totals.hsa),
    ("pre-tax", "░", |totals| totals.deferred),
    ("Roth", "▒", |totals| totals.roth),
    ("taxable", "▓", |totals| totals.taxable),
];

/// Each treatment's balance stacked on those under it, a band each in its
/// own colour and glyph, under net worth's line along the top edge. A
/// later band wins the row it shares with an earlier one, so the HSA is
/// drawn last and keeps a row wherever it holds anything.
fn balances(projection: &Projection, nominal: bool, theme: &Theme) -> SeriesChart {
    let mut shades = Vec::with_capacity(CLASSES.len());
    let mut lows: Vec<f64> = vec![0.0; projection.years.len()];
    for (place, (_, symbol, value)) in CLASSES.iter().enumerate() {
        let points = (projection.years.iter().zip(&mut lows))
            .map(|(row, low)| {
                let amount = basis_amount(value(&row.class_totals), row.deflator, nominal);
                let high = *low + amount as f64;
                let band = (f64::from(row.year), *low, high);
                *low = high;
                band
            })
            .collect();
        shades.push(Shade {
            points,
            symbol,
            color: theme.series(place),
        });
    }
    shades.rotate_left(1);
    let worth = Series::of("net worth", theme.fg, projection, nominal, |row| {
        row.net_worth
    });
    let mut chart = SeriesChart::of(vec![worth], theme.dimmed()).shaded(shades);
    chart.is_legend_hidden = true;
    chart
}

/// The key the balances are read by: each treatment's glyph in its colour.
fn key(theme: &Theme) -> Line<'static> {
    let spans = CLASSES
        .iter()
        .enumerate()
        .flat_map(|(place, (label, symbol, _))| {
            let swatch = Span::styled(
                symbol.repeat(KEY_SWATCH),
                Style::new().fg(theme.series(place)),
            );
            [swatch, Span::raw(format!(" {label}  "))]
        });
    Line::from(spans.collect::<Vec<_>>())
}

fn income_taxes(projection: &Projection, nominal: bool, theme: &Theme) -> SeriesChart {
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
    SeriesChart::of(vec![income, taxes], theme.dimmed())
}

fn net_worth(projection: &Projection, nominal: bool, theme: &Theme) -> SeriesChart {
    let worth = Series::of(
        "net worth",
        theme.series(WORTH_SERIES),
        projection,
        nominal,
        |row| row.net_worth,
    );
    SeriesChart::of(vec![worth], theme.dimmed())
}

/// The year cursor, then where each earner's salary ends; a label that
/// does not fit gives way in that order.
fn marks(projected: &Projected, cursor: i16, theme: &Theme) -> Vec<Mark> {
    let ends = salary_ends(projected);
    let is_only_earner = ends.len() == 1;
    let cursor = Mark::cursor(cursor, theme);
    let ends = ends.into_iter().map(|(owner, year)| Mark {
        year,
        label: if is_only_earner {
            format!("retire {year}")
        } else {
            format!("{} {year}", projected.plan.person_name(owner))
        },
        style: theme.dimmed(),
    });
    iter::once(cursor).chain(ends).collect()
}

/// Each earner with the first year none of their salaries pays, earliest
/// first and then by owner; an earner paid to the horizon has none.
fn salary_ends(projected: &Projected) -> Vec<(&str, i16)> {
    let years = &projected.projection.years;
    let mut last_paid: BTreeMap<&str, i16> = BTreeMap::new();
    let salaries = projected.plan.income.iter();
    for income in salaries.filter(|income| income.kind == IncomeKind::Salary) {
        let Some(row) = years
            .iter()
            .rev()
            .find(|row| row.income.get(&income.id).is_some_and(|&amount| amount > 0))
        else {
            continue;
        };
        last_paid
            .entry(&income.owner)
            .and_modify(|last| *last = (*last).max(row.year))
            .or_insert(row.year);
    }
    let horizon = years.last().map(|row| row.year);
    let mut ends: Vec<_> = last_paid
        .into_iter()
        .filter(|&(_, year)| Some(year) != horizon)
        .map(|(owner, year)| (owner, year + 1))
        .collect();
    ends.sort_by_key(|&(_, year)| year);
    ends
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::support::{TEST_PLAN, projected_from, test_projected};

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
        let marks = marks(projected, 2030, &Theme::terminal());
        marks.into_iter().map(|mark| mark.label).collect()
    }

    #[test]
    fn a_salary_ends_the_year_after_it_last_pays() {
        let alone = test_projected();
        let end = salary_ends(&alone)[0].1;
        let paid = |year: i16| {
            let row = alone.projection.years.iter().find(|row| row.year == year);
            row.unwrap().income.contains_key("salary")
        };
        assert!(paid(end - 1) && !paid(end), "{end}");
        assert_eq!(labels(&alone), ["2030".to_owned(), format!("retire {end}")]);
    }

    #[test]
    fn each_earner_is_marked_by_name_at_their_last_salary() {
        let pair = projected_from(&format!("{TEST_PLAN}{SECOND_EARNER}"));
        let end = salary_ends(&test_projected())[0].1;
        assert_eq!(salary_ends(&pair), [("you", 2035), ("me", end)]);
        assert_eq!(labels(&pair)[1], "you 2035");
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
        let nominal = income_taxes(projection, true, &Theme::terminal());
        let todays = income_taxes(projection, false, &Theme::terminal());
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
