//! What the Overview's charts draw, as every surface keys it: each line's
//! name and its figure in each year, and what the market runs' bands are
//! called.

use retiretui_engine::market::BAND_PERCENTILES;
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::YearRow;
use serde::Serialize;

use super::Chart;
use crate::ledger::salary_marks;
use crate::metric::Metric;
use crate::present::treatment_word;
use crate::session::Projected;
use crate::table::{basis_amount, present_classes};

/// One line of a chart: what it is called and its figure in each year.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChartLine {
    /// What the chart's key calls it.
    pub label: &'static str,
    /// Each year beside its figure, first to last.
    pub points: Vec<(i16, Dollars)>,
}

/// A year a chart points out.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChartMark {
    /// The calendar year.
    pub year: i16,
    /// What happens in it.
    pub label: String,
}

/// What one of the Overview's charts draws of the plan's own projection.
#[derive(Clone, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Charted {
    /// What each treatment the plan holds has, stacked from the bottom in
    /// this order.
    pub stacked: Vec<ChartLine>,
    /// The lines drawn over them.
    pub lines: Vec<ChartLine>,
    /// The years each earner's salary ends.
    pub marks: Vec<ChartMark>,
}

impl Chart {
    /// What the chart draws of `projected`, nominal or in today's dollars.
    /// The market runs' bands are not the projection's, so it draws only
    /// the marks of them.
    #[must_use]
    pub fn charted(self, projected: &Projected, nominal: bool) -> Charted {
        let years = &projected.projection.years;
        let line = |label, figure: &dyn Fn(&YearRow) -> Dollars| ChartLine {
            label,
            points: (years.iter())
                .map(|row| (row.year, basis_amount(figure(row), row.deflator, nominal)))
                .collect(),
        };
        let worth = || line(Metric::NetWorth.title(), &|row| row.net_worth);
        let (stacked, lines) = match self {
            Self::Balances => {
                let classes = present_classes(&projected.plan).into_iter();
                let held = classes
                    .map(|class| line(treatment_word(class), &|row| row.class_totals.get(class)));
                (held.collect(), vec![worth()])
            }
            Self::NetWorth => (Vec::new(), vec![worth()]),
            Self::IncomeTaxes => (
                Vec::new(),
                vec![
                    line(Metric::Income.title(), &|row| row.total_income),
                    line(Metric::Taxes.title(), &|row| row.taxes.total),
                ],
            ),
            Self::Markets => (Vec::new(), Vec::new()),
        };
        let marks = salary_marks(projected).into_iter();
        Charted {
            stacked,
            lines,
            marks: marks
                .map(|(label, year)| ChartMark { year, label })
                .collect(),
        }
    }
}

/// A band of the market runs' spread, or their median: what it is called,
/// and the places in [`BAND_PERCENTILES`] it lies between, one place twice
/// for the median.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Spread {
    /// What a chart's key calls it.
    pub label: String,
    /// The place of the percentile below it.
    pub low: usize,
    /// The place of the one above it.
    pub high: usize,
}

/// The market runs' outer and inner bands and their median.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BandWords {
    /// The widest band, drawn lightest.
    pub outer: Spread,
    /// The band within it.
    pub inner: Spread,
    /// The line through their middle.
    pub median: Spread,
}

/// What the market runs' bands and median are called, and where each
/// lies among [`BAND_PERCENTILES`].
#[must_use]
pub fn band_words() -> BandWords {
    let last = BAND_PERCENTILES.len() - 1;
    let between = |low: usize, high: usize| Spread {
        label: format!(
            "{}th to {}th",
            BAND_PERCENTILES[low], BAND_PERCENTILES[high]
        ),
        low,
        high,
    };
    let middle = last / 2;
    BandWords {
        outer: between(0, last),
        inner: between(1, last - 1),
        median: Spread {
            label: "median".to_owned(),
            low: middle,
            high: middle,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overview::tests::test_projected;

    fn labels(lines: &[ChartLine]) -> Vec<&str> {
        lines.iter().map(|line| line.label).collect()
    }

    #[test]
    fn balances_stack_the_treatments_held_taxable_first_under_net_worth() {
        let projected = test_projected();
        let charted = Chart::Balances.charted(&projected, false);
        assert_eq!(labels(&charted.stacked), ["taxable", "pre-tax"]);
        assert_eq!(labels(&charted.lines), ["Net worth"]);
        let row = &projected.projection.years[0];
        assert_eq!(
            charted.stacked[0].points[0],
            (
                row.year,
                basis_amount(row.class_totals.taxable, row.deflator, false)
            )
        );
    }

    #[test]
    fn each_chart_names_its_lines_by_their_metric() {
        let projected = test_projected();
        let named = |chart: Chart| labels(&chart.charted(&projected, true).lines).join(", ");
        assert_eq!(named(Chart::NetWorth), "Net worth");
        assert_eq!(named(Chart::IncomeTaxes), "Income, Taxes");
        assert_eq!(named(Chart::Markets), "");
        let markets = Chart::Markets.charted(&projected, true);
        assert_eq!(
            markets.marks,
            Chart::NetWorth.charted(&projected, true).marks
        );
    }

    #[test]
    fn a_line_follows_the_basis() {
        let projected = test_projected();
        let nominal = Chart::NetWorth.charted(&projected, true).lines.remove(0);
        let today = Chart::NetWorth.charted(&projected, false).lines.remove(0);
        let last = nominal.points.len() - 1;
        assert_eq!(nominal.points[last].0, today.points[last].0);
        assert_ne!(nominal.points[last].1, today.points[last].1);
    }

    #[test]
    fn the_bands_are_named_by_the_percentiles_they_lie_between() {
        let words = band_words();
        assert_eq!(words.outer.label, "10th to 90th");
        assert_eq!((words.outer.low, words.outer.high), (0, 4));
        assert_eq!(words.inner.label, "25th to 75th");
        assert_eq!((words.inner.low, words.inner.high), (1, 3));
        assert_eq!(words.median.label, "median");
        assert_eq!(BAND_PERCENTILES[words.median.low], 50);
    }
}
