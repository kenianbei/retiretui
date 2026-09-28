//! The projected quantities plans are compared by, year by year.

use retiretui_engine::plan::Dollars;
use retiretui_engine::project::YearRow;
use serde::{Deserialize, Serialize};

/// A projected quantity comparable year by year, kept as its kebab-case
/// name.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Metric {
    /// Sum of all end-of-year balances.
    #[default]
    NetWorth,
    /// Gross income, Social Security included.
    Income,
    /// Spending for the year.
    Expenses,
    /// Total taxes assessed.
    Taxes,
    /// Money withdrawn, RMDs included.
    Withdrawals,
    /// Roth conversions executed.
    Conversions,
    /// Modified adjusted gross income.
    Magi,
    /// Spending the accounts could not cover.
    Unfunded,
}

impl Metric {
    /// Every metric, in the order they are offered.
    pub const ALL: [Self; 8] = [
        Self::NetWorth,
        Self::Income,
        Self::Expenses,
        Self::Taxes,
        Self::Withdrawals,
        Self::Conversions,
        Self::Magi,
        Self::Unfunded,
    ];

    /// The metric's name as a heading says it.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::NetWorth => "Net worth",
            Self::Income => "Income",
            Self::Expenses => "Expenses",
            Self::Taxes => "Taxes",
            Self::Withdrawals => "Withdrawals",
            Self::Conversions => "Conversions",
            Self::Magi => "MAGI",
            Self::Unfunded => "Unfunded",
        }
    }

    /// The metric `step` places along, wrapping at either end.
    #[must_use]
    pub fn neighbor(self, step: isize) -> Self {
        let all = Self::ALL;
        let at = all.iter().position(|metric| *metric == self).unwrap_or(0);
        let count = all.len() as isize;
        all[(at as isize + step).rem_euclid(count) as usize]
    }

    /// The metric's figure in `row`, nominal.
    #[must_use]
    pub fn value(self, row: &YearRow) -> Dollars {
        match self {
            Self::NetWorth => row.net_worth,
            Self::Income => row.total_income,
            Self::Expenses => row.expenses,
            Self::Taxes => row.taxes.total,
            Self::Withdrawals => row.total_withdrawals(),
            Self::Conversions => row.conversions,
            Self::Magi => row.taxes.magi,
            Self::Unfunded => row.unfunded,
        }
    }
}
