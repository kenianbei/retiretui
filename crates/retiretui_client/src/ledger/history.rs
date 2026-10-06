//! Each of the year's three money panes across every year: the two
//! figures a chart draws beneath it, so the year is read against the plan.

use retiretui_engine::plan::Dollars;
use retiretui_engine::project::{Projection, YearRow};
use serde::Serialize;

use crate::table::basis_amount;

/// One of the year's money panes, charted over the plan.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum History {
    /// Income against what was withdrawn.
    MoneyIn,
    /// Spending against tax.
    MoneyOut,
    /// MAGI against taxable income.
    Tax,
}

/// A figure of a year, under the name a chart keys it by.
type Figure = (&'static str, fn(&YearRow) -> Dollars);

impl History {
    /// Every history, in the order of the panes they stand under.
    pub const ALL: [Self; 3] = [Self::MoneyIn, Self::MoneyOut, Self::Tax];

    /// What the chart is titled.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::MoneyIn => "Money in by year",
            Self::MoneyOut => "Money out by year",
            Self::Tax => "Tax by year",
        }
    }

    fn figures(self) -> [Figure; 2] {
        match self {
            Self::MoneyIn => [
                ("Income", |row| row.total_income),
                ("Withdrawn", YearRow::total_withdrawals),
            ],
            Self::MoneyOut => [
                ("Spending", |row| row.expenses),
                ("Tax", |row| row.taxes.total),
            ],
            Self::Tax => [
                ("MAGI", |row| row.taxes.magi),
                ("Taxable income", |row| row.taxes.ordinary_taxable),
            ],
        }
    }

    /// What its two lines are named.
    #[must_use]
    pub fn lines(self) -> [&'static str; 2] {
        self.figures().map(|(name, _)| name)
    }

    /// Each year of `projection` beside its two figures, nominal or in
    /// today's dollars.
    #[must_use]
    pub fn points(self, projection: &Projection, is_nominal: bool) -> Vec<(i16, [Dollars; 2])> {
        let figures = self.figures();
        let years = projection.years.iter();
        years
            .map(|row| {
                let on_basis =
                    |(_, figure): Figure| basis_amount(figure(row), row.deflator, is_nominal);
                (row.year, figures.map(on_basis))
            })
            .collect()
    }
}
