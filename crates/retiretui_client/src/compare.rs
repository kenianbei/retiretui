//! What is said of plans compared side by side: each plan's figures in
//! the Overview's words, or against a baseline's; a metric year by year;
//! and what one plan changes of another.

use std::collections::BTreeMap;

use retiretui_engine::plan::{Dollars, Plan, diff};
use retiretui_engine::project::{Summary, YearRow};

use crate::forms::changes::change_words;
use crate::metric::Metric;
use crate::present::{
    self, ENDS_WITH, LIFETIME_TAXES, MONEY_LASTS, PEAKS_AT, SAME, compact_money, signed_money,
};
use crate::table::{basis_amount, running_text};

/// The header of the column naming each plan.
pub const PLAN: &str = "Plan";
/// What the baseline changes of itself.
pub const THE_BASELINE: &str = "The baseline.";
/// What a plan no different from the baseline changes of it.
pub const THE_SAME: &str = "Same as the baseline.";
/// A year a plan does not reach.
pub const UNREACHED: &str = "-";
const WAITING: &str = "waiting";
const FAILED: &str = "—";
const PERCENT: f64 = 100.0;
/// A difference under this many points rounds to none at the one place
/// it is shown to, and reads as the same.
const SAME_POINTS: f64 = 0.05;

/// A plan's success through random markets, as far as it is known.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Success {
    /// Not yet run.
    Waiting,
    /// Under way, `done` of `total` runs made.
    Running {
        /// Runs made.
        done: usize,
        /// Runs to make.
        total: usize,
    },
    /// The share of runs the money lasted through.
    Rate(f64),
    /// The run was refused or failed.
    Failed,
}

impl Success {
    /// The success as a table cell says it.
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::Waiting => WAITING.to_owned(),
            Self::Running { done, total } => running_text(done, total),
            Self::Rate(rate) => present::rate(rate),
            Self::Failed => FAILED.to_owned(),
        }
    }

    /// Points more or fewer than `base`'s, where both have answered.
    #[must_use]
    pub fn against(self, base: Self) -> String {
        let (Self::Rate(own), Self::Rate(base)) = (self, base) else {
            return self.text();
        };
        let points = (own - base) * PERCENT;
        if points.abs() < SAME_POINTS {
            SAME.to_owned()
        } else {
            format!("{points:+.1} pts")
        }
    }
}

/// What a plan's row is read from.
#[derive(Debug)]
pub struct Figured {
    /// The plan's summary in the basis shown.
    pub summary: Summary,
    /// Its success through random markets.
    pub success: Success,
    /// The charted metric in the year shown, as [`figure`] says it.
    pub in_year: String,
}

/// A figure the table can show of each plan.
#[derive(Debug)]
pub struct Column {
    /// The column's header, empty where [`headers`] names it.
    pub header: &'static str,
    figure: Figure,
}

#[derive(Debug)]
enum Figure {
    Money(fn(&Summary) -> Dollars),
    Lasts,
    Success,
    /// The charted metric in the year shown.
    InYear,
    Peaks,
}

impl Figure {
    /// The figure of `own`, or its difference from `base`'s.
    fn cell(&self, own: &Figured, base: Option<&Figured>) -> String {
        let summary = &own.summary;
        match (self, base) {
            (Self::Money(of), None) => compact_money(of(summary)),
            (Self::Money(of), Some(base)) => signed_money(of(summary) - of(&base.summary)),
            (Self::Lasts, None) => present::money_lasts(summary),
            (Self::Lasts, Some(base)) => present::money_lasts_against(summary, &base.summary),
            (Self::Success, None) => own.success.text(),
            (Self::Success, Some(base)) => own.success.against(base.success),
            (Self::InYear, _) => own.in_year.clone(),
            (Self::Peaks, None) => present::peaks_at(summary),
            (Self::Peaks, Some(base)) => present::peaks_at_against(summary, &base.summary),
        }
    }
}

/// Every column after the plan's name, most wanted first, so a narrow
/// table may drop them from the end.
pub const COLUMNS: [Column; 10] = [
    Column {
        header: ENDS_WITH,
        figure: Figure::Money(|summary| summary.final_net_worth),
    },
    Column {
        header: MONEY_LASTS,
        figure: Figure::Lasts,
    },
    Column {
        header: "Success",
        figure: Figure::Success,
    },
    Column {
        header: "",
        figure: Figure::InYear,
    },
    Column {
        header: LIFETIME_TAXES,
        figure: Figure::Money(|summary| summary.lifetime_taxes),
    },
    Column {
        header: PEAKS_AT,
        figure: Figure::Peaks,
    },
    Column {
        header: "Medicare",
        figure: Figure::Money(|summary| summary.lifetime_medicare),
    },
    Column {
        header: "Lifetime conversions",
        figure: Figure::Money(|summary| summary.lifetime_conversions),
    },
    Column {
        header: "Unfunded",
        figure: Figure::Money(|summary| summary.lifetime_unfunded),
    },
    Column {
        header: "Pre-tax at end",
        figure: Figure::Money(|summary| summary.final_deferred),
    },
];

/// The table's headers: the plan's name, then each column's, the charted
/// `metric`'s naming it and `year`.
#[must_use]
pub fn headers(metric: Metric, year: i16) -> Vec<String> {
    let columns = COLUMNS.iter().map(|column| match column.figure {
        Figure::InYear => format!("{} {year}", metric.title()),
        _ => column.header.to_owned(),
    });
    std::iter::once(PLAN.to_owned()).chain(columns).collect()
}

/// `own`'s figures, or under difference each against `base`'s.
#[must_use]
pub fn cells(own: &Figured, base: Option<&Figured>) -> Vec<String> {
    (COLUMNS.iter())
        .map(|column| column.figure.cell(own, base))
        .collect()
}

/// `metric` in each of `years`, in the basis shown.
#[must_use]
pub fn amounts(years: &[YearRow], metric: Metric, nominal: bool) -> BTreeMap<i16, Dollars> {
    (years.iter())
        .map(|row| {
            let amount = basis_amount(metric.value(row), row.deflator, nominal);
            (row.year, amount)
        })
        .collect()
}

/// `amounts` less `base`'s, in the years both reach.
#[must_use]
pub fn less(
    amounts: &BTreeMap<i16, Dollars>,
    base: &BTreeMap<i16, Dollars>,
) -> BTreeMap<i16, Dollars> {
    (amounts.iter())
        .filter_map(|(year, amount)| Some((*year, amount - base.get(year)?)))
        .collect()
}

/// A year's amount, signed where it is a difference from the baseline,
/// [`UNREACHED`] where the plan does not reach the year.
#[must_use]
pub fn figure(amount: Option<Dollars>, is_difference: bool) -> String {
    let figure = if is_difference {
        amount.map(signed_money)
    } else {
        amount.map(compact_money)
    };
    figure.unwrap_or_else(|| UNREACHED.to_owned())
}

/// What `other` changes of `base`, a line per change; none where it is
/// [`THE_SAME`].
///
/// # Errors
///
/// Where either plan cannot be read back as the tables a diff compares.
pub fn changes(base: &Plan, other: &Plan) -> Result<Vec<String>, String> {
    let changes = diff(base, other).map_err(|error| error.to_string())?;
    Ok((changes.iter())
        .map(|change| change_words(change, (base, other)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_against_a_base_is_in_points_or_the_same() {
        let own = Success::Rate(0.912);
        assert_eq!(own.against(Success::Rate(0.9)), "+1.2 pts");
        assert_eq!(own.against(Success::Rate(0.9121)), SAME);
        assert_eq!(own.against(Success::Waiting), own.text());
        assert_eq!(Success::Failed.against(Success::Rate(0.9)), "—");
        let running = Success::Running {
            done: 1_200,
            total: 5_000,
        };
        assert_eq!(running.text(), "running 1,200 of 5,000");
    }

    #[test]
    fn a_difference_is_taken_in_the_years_both_reach() {
        let own = BTreeMap::from([(2030, 100), (2031, 200), (2032, 300)]);
        let base = BTreeMap::from([(2030, 50), (2031, 250)]);
        assert_eq!(less(&own, &base), BTreeMap::from([(2030, 50), (2031, -50)]));
        assert_eq!(figure(None, true), UNREACHED);
        assert!(figure(Some(-50), true).starts_with('-'));
        assert!(figure(Some(50), true).starts_with('+'));
    }

    #[test]
    fn the_in_year_column_names_the_metric_and_year() {
        let headers = headers(Metric::Taxes, 2031);
        assert_eq!(headers.len(), COLUMNS.len() + 1);
        assert_eq!(headers[..2], [PLAN, ENDS_WITH]);
        assert_eq!(headers[4], "Taxes 2031");
    }
}
