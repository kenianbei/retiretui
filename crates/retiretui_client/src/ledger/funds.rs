//! What a year lived on and where it went: income, withdrawals and any
//! shortfall on one side, each expense, tax and what was put away on the
//! other, each side over what its kinds come to. The two sides come to
//! the same total.

use retiretui_engine::plan::{Dollars, Item, Plan};
use retiretui_engine::project::{Action, YearRow};
use serde::Serialize;

use crate::present::{account_name, income_name, money};
use crate::table::basis_amount;

/// What the Ledger titles where a year's money came from.
pub const MONEY_IN: &str = "Money in";
/// What the Ledger titles where a year's money went.
pub const MONEY_OUT: &str = "Money out";
pub(super) const TOTAL: &str = "Total";
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

/// One side of a year's money: every amount in one list, what each kind
/// of them comes to, and what they all come to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Funds {
    /// Each amount, in the order it is read.
    pub lines: Vec<DetailLine>,
    /// What the lines of each kind come to, said beneath them as a
    /// summary; none where one sum would only repeat the total.
    pub sums: Vec<DetailLine>,
    /// The side's total, which the other side's equals.
    pub total: DetailLine,
}

/// Amounts not yet said, each under its label.
type Amounts = Vec<(String, Dollars)>;

/// `amount` of `row` under `label`, nominal or in today's dollars.
pub(super) fn line(row: &YearRow, is_nominal: bool, label: &str, amount: Dollars) -> DetailLine {
    DetailLine {
        label: label.to_owned(),
        amount: money(basis_amount(amount, row.deflator, is_nominal)),
    }
}

/// `amounts` that are more than nothing.
pub(super) fn paid<'a>(amounts: impl IntoIterator<Item = (&'a str, Dollars)>) -> Amounts {
    let paid = amounts.into_iter().filter(|&(_, amount)| amount != 0);
    paid.map(|(label, amount)| (label.to_owned(), amount))
        .collect()
}

/// `lines` over each of `kinds`' sums that is more than nothing, and
/// `total` under them all; no sums where one would only repeat the total.
/// The total is said from the sum rather than added from the lines said,
/// so both sides' totals agree to the dollar on either basis.
fn funds(
    (row, is_nominal): (&YearRow, bool),
    lines: &Amounts,
    kinds: &[(&str, Dollars)],
    total: Dollars,
) -> Funds {
    let said = |label: &str, amount| line(row, is_nominal, label, amount);
    let mut sums: Vec<_> = kinds.iter().filter(|&&(_, sum)| sum != 0).collect();
    if matches!(sums[..], [&(_, only)] if only == total) {
        sums.clear();
    }
    Funds {
        lines: (lines.iter())
            .map(|(label, amount)| said(label, *amount))
            .collect(),
        sums: sums.iter().map(|&&(kind, sum)| said(kind, sum)).collect(),
        total: said(TOTAL, total),
    }
}

/// What `row` lived on: its income by source, then what it drew from each
/// account - a required distribution named as one - and what it could not
/// find, over what the income and the withdrawals each come to.
#[must_use]
pub fn money_in(plan: &Plan, row: &YearRow, is_nominal: bool) -> Funds {
    let income = row.income.iter();
    let mut lines = paid(income.map(|(id, &amount)| (income_name(plan, id), amount)));
    lines.extend(row.actions.iter().filter_map(|action| match action {
        Action::Rmd { account, amount } => {
            Some((format!("RMD from {}", account_name(plan, account)), *amount))
        }
        Action::Withdrawal { account, amount } => {
            Some((format!("From {}", account_name(plan, account)), *amount))
        }
        _ => None,
    }));
    lines.extend(paid([("Unfunded", row.unfunded)]));
    let drawn = row.total_withdrawals();
    let kinds = [("Income", row.total_income), ("Withdrawn", drawn)];
    let total = row.total_income + drawn + row.unfunded;
    funds((row, is_nominal), &lines, &kinds, total)
}

/// Where what `row` lived on went: each expense that spent anything, in the
/// plan's order, then its tax and Medicare's surcharges, what the household
/// paid into its accounts, and what was left over and saved - over what the
/// spending comes to, by kind where it is of more than one.
#[must_use]
pub fn money_out(plan: &Plan, row: &YearRow, is_nominal: bool) -> Funds {
    let mut lines: Amounts = (plan.expenses.iter())
        .filter_map(|expense| {
            let spent = *row.spending.get(&expense.id)?;
            Some((expense.display_name().to_owned(), spent))
        })
        .collect();
    lines.extend(paid([
        ("Tax", row.taxes.total),
        ("Medicare surcharges", row.medicare),
        ("Paid into accounts", row.contributions_employee),
        ("Surplus saved", row.surplus),
    ]));
    let kinds = [
        ("Essential", row.expenses_essential),
        ("Flexible", row.expenses_flexible),
        ("One-time", row.expenses_once()),
    ];
    let is_split = kinds.iter().filter(|&&(_, spent)| spent != 0).count() > 1;
    let kinds: &[(&str, Dollars)] = if is_split {
        &kinds
    } else {
        &[(SPENDING, row.expenses)]
    };
    let put_away = row.medicare + row.contributions_employee + row.surplus;
    let total = row.expenses + row.taxes.total + put_away;
    funds((row, is_nominal), &lines, kinds, total)
}
