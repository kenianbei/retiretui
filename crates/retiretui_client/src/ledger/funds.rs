//! What a year lived on and where it went: income, withdrawals and any
//! shortfall on one side, spending, tax and what was put away on the
//! other. The two sides come to the same total.

use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::{Action, YearRow};
use serde::Serialize;

use crate::present::{account_name, income_name, money};
use crate::table::basis_amount;

/// What the Ledger titles where a year's money came from.
pub const MONEY_IN: &str = "Money in";
/// What the Ledger titles where a year's money went.
pub const MONEY_OUT: &str = "Money out";
const TOTAL: &str = "Total";
const SPENDING: &str = "Spending";

/// One labelled amount of a year, said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DetailLine {
    /// What the amount is.
    pub label: String,
    /// The amount, on the basis asked for.
    pub amount: String,
}

/// Lines that belong together, and what they come to where there are
/// several.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Group {
    /// Each amount of the group.
    pub lines: Vec<DetailLine>,
    /// What they add up to.
    pub subtotal: Option<DetailLine>,
}

/// One side of a year's money: its groups, and what they all come to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Funds {
    /// The groups, in the order they are read.
    pub groups: Vec<Group>,
    /// The side's total, which the other side's equals.
    pub total: DetailLine,
}

/// Amounts not yet said, each under its label.
type Amounts = Vec<(String, Dollars)>;

/// Says a year's amounts on one basis. A sum is said from the sum of what
/// it adds, so both sides' totals agree to the dollar on either basis.
struct Saying {
    deflator: f64,
    is_nominal: bool,
}

impl Saying {
    fn line(&self, label: &str, amount: Dollars) -> DetailLine {
        DetailLine {
            label: label.to_owned(),
            amount: money(basis_amount(amount, self.deflator, self.is_nominal)),
        }
    }

    /// `amounts` as a group, under `sum` where there are several; none of
    /// nothing.
    fn group(&self, amounts: &Amounts, sum: Option<&str>) -> Option<Group> {
        let lines: Vec<DetailLine> = amounts
            .iter()
            .map(|(label, amount)| self.line(label, *amount))
            .collect();
        let whole: Dollars = amounts.iter().map(|&(_, amount)| amount).sum();
        let subtotal = sum.filter(|_| lines.len() > 1);
        (!lines.is_empty()).then(|| Group {
            lines,
            subtotal: subtotal.map(|label| self.line(label, whole)),
        })
    }

    fn funds(&self, groups: Vec<Option<Group>>, total: Dollars) -> Funds {
        Funds {
            groups: groups.into_iter().flatten().collect(),
            total: self.line(TOTAL, total),
        }
    }
}

/// `amounts` that are more than nothing.
fn paid<'a>(amounts: impl IntoIterator<Item = (&'a str, Dollars)>) -> Amounts {
    let paid = amounts.into_iter().filter(|&(_, amount)| amount != 0);
    paid.map(|(label, amount)| (label.to_owned(), amount))
        .collect()
}

/// What `row` lived on: its income by source, what it drew from each
/// account - a required distribution named as one - and what it could not
/// find.
#[must_use]
pub fn money_in(plan: &Plan, row: &YearRow, is_nominal: bool) -> Funds {
    let saying = Saying {
        deflator: row.deflator,
        is_nominal,
    };
    let income = row.income.iter();
    let income = paid(income.map(|(id, &amount)| (income_name(plan, id), amount)));
    let drawn: Amounts = (row.actions.iter())
        .filter_map(|action| match action {
            Action::Rmd { account, amount } => {
                Some((format!("RMD from {}", account_name(plan, account)), *amount))
            }
            Action::Withdrawal { account, amount } => {
                Some((format!("From {}", account_name(plan, account)), *amount))
            }
            _ => None,
        })
        .collect();
    let groups = vec![
        saying.group(&income, Some("Income")),
        saying.group(&drawn, Some("Withdrawn")),
        saying.group(&paid([("Unfunded", row.unfunded)]), None),
    ];
    let total = row.total_income + row.total_withdrawals() + row.unfunded;
    saying.funds(groups, total)
}

/// Where what `row` lived on went: its spending - by kind where it is of
/// more than one - its tax and Medicare's surcharges, what the household
/// paid into its accounts, and what was left over and saved.
#[must_use]
pub fn money_out(row: &YearRow, is_nominal: bool) -> Funds {
    let saying = Saying {
        deflator: row.deflator,
        is_nominal,
    };
    let mut spent = paid([
        ("Essential", row.expenses_essential),
        ("Flexible", row.expenses_flexible),
        ("One-time", row.expenses_once()),
    ]);
    if spent.len() < 2 {
        spent = paid([(SPENDING, row.expenses)]);
    }
    let rest = paid([
        ("Tax", row.taxes.total),
        ("Medicare surcharges", row.medicare),
        ("Paid into accounts", row.contributions_employee),
        ("Surplus saved", row.surplus),
    ]);
    let groups = vec![
        saying.group(&spent, Some(SPENDING)),
        saying.group(&rest, None),
    ];
    let put_away = row.medicare + row.contributions_employee + row.surplus;
    saying.funds(groups, row.expenses + row.taxes.total + put_away)
}
