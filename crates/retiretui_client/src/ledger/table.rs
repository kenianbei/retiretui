//! Every year as a row: the year, the ages reached in it, what marks it,
//! and its figures under one of the column sets the table turns through.

use std::collections::BTreeSet;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Item};
use retiretui_engine::project::YearRow;
use serde::{Deserialize, Serialize};

use crate::actions::has_warnings;
use crate::overview::milestones;
use crate::present::treatment_class;
use crate::session::Projected;
use crate::table::{Column, ages_text, basis_amount, present_classes, year_figures};

/// What the Ledger titles its list of years.
pub const YEARS: &str = "Years";
const TEXT_HEADERS: [&str; 2] = ["Year", "Age"];
/// What every set's figures open with.
const LEADING: [&str; 4] = ["Income", "Spending", "Tax", "Withdrawn"];
const TAX_HEADERS: [&str; 4] = ["MAGI", "Taxable inc", "Converted", "RMDs"];
const NET_WORTH: &str = "Net worth";

/// The figures the year table shows between what every year leads with
/// and its net worth.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ColumnSet {
    /// What each tax treatment holds.
    #[default]
    Treatments,
    /// What each account holds.
    Accounts,
    /// What the year's tax is worked out from, and what it converted and
    /// was required to draw.
    Tax,
}

impl ColumnSet {
    /// Every set, in the order they are turned through.
    pub const ALL: [Self; 3] = [Self::Treatments, Self::Accounts, Self::Tax];

    /// The set as a heading says it.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Treatments => "Balances by treatment",
            Self::Accounts => "Balances by account",
            Self::Tax => "Tax figures",
        }
    }

    /// The set as an address names it.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Treatments => "treatments",
            Self::Accounts => "accounts",
            Self::Tax => "tax",
        }
    }

    /// The set `step` places along, wrapping at either end.
    #[must_use]
    pub fn neighbor(self, step: isize) -> Self {
        let sets = Self::ALL.len().cast_signed();
        let at = Self::ALL.iter().position(|&each| each == self);
        let along = (at.unwrap_or(0).cast_signed() + step).rem_euclid(sets);
        Self::ALL[along.cast_unsigned()]
    }
}

/// What sets a year apart in a list of them.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Marks {
    /// A milestone of the plan falls in it.
    pub is_milestone: bool,
    /// It has a warning.
    pub has_warning: bool,
}

impl Marks {
    /// Whether the year is one a reader steps to.
    #[must_use]
    pub const fn is_marked(self) -> bool {
        self.is_milestone || self.has_warning
    }
}

/// One year of the table.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableRow {
    /// The calendar year.
    pub year: i16,
    /// The ages reached in it, in household order.
    pub ages: String,
    /// What sets it apart.
    pub marks: Marks,
    /// Its figures on the basis asked for, in header order.
    pub figures: Vec<Dollars>,
    /// Whether it could not pay for everything.
    pub is_exceeded: bool,
}

/// Every projected year under one column set.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Table {
    /// The year's and the ages' headers and then each figure's, each beside
    /// whether its column holds figures.
    pub headers: Vec<(String, bool)>,
    /// Each year, first to last.
    pub rows: Vec<TableRow>,
}

impl Table {
    /// `projected` year by year under `set`, nominal or in today's dollars.
    /// Income, spending, tax and what was withdrawn lead every set, and net
    /// worth ends it.
    #[must_use]
    pub fn new(
        projected: &Projected,
        tables: &TaxTables,
        set: ColumnSet,
        is_nominal: bool,
    ) -> Self {
        let plan = &projected.plan;
        let accounts = plan.accounts.iter();
        let (balances, named): (Vec<Column>, Vec<&str>) = match set {
            ColumnSet::Treatments => (present_classes(plan).into_iter())
                .map(|class| (Column::Class(class), treatment_class(class)))
                .unzip(),
            ColumnSet::Accounts => accounts
                .map(|account| (Column::Account(account.id.clone()), account.display_name()))
                .unzip(),
            ColumnSet::Tax => (Vec::new(), TAX_HEADERS.to_vec()),
        };
        let figures = LEADING.into_iter().chain(named).chain([NET_WORTH]);
        let text = TEXT_HEADERS.into_iter().map(|header| (header, false));
        let headers = text.chain(figures.map(|header| (header, true)));
        let dated = milestones(projected, is_nominal);
        let dated: BTreeSet<i16> = dated.iter().filter_map(|row| row.year).collect();
        let rows = projected.projection.years.iter().map(|row| {
            let mut figures = year_figures(row, &balances);
            if set == ColumnSet::Tax {
                figures.splice(LEADING.len()..LEADING.len(), tax_figures(row));
            }
            let on_basis = |amount| basis_amount(amount, row.deflator, is_nominal);
            TableRow {
                year: row.year,
                ages: ages_text(plan, row),
                marks: Marks {
                    is_milestone: dated.contains(&row.year),
                    has_warning: has_warnings(plan, tables, row),
                },
                figures: figures.into_iter().map(on_basis).collect(),
                is_exceeded: row.unfunded > 0,
            }
        });
        Self {
            headers: headers
                .map(|(header, is_figure)| (header.to_owned(), is_figure))
                .collect(),
            rows: rows.collect(),
        }
    }

    /// The nearest marked year past `from`, later where `step` is above
    /// zero and earlier where it is not.
    #[must_use]
    pub fn marked_year(&self, from: i16, step: i16) -> Option<i16> {
        let mut marked = (self.rows.iter())
            .filter(|row| row.marks.is_marked())
            .map(|row| row.year);
        if step > 0 {
            marked.find(|&year| year > from)
        } else {
            marked.rfind(|&year| year < from)
        }
    }
}

/// A year's figures under [`TAX_HEADERS`].
fn tax_figures(row: &YearRow) -> [Dollars; 4] {
    let taxes = &row.taxes;
    [
        taxes.magi,
        taxes.ordinary_taxable,
        row.conversions,
        row.rmds,
    ]
}
