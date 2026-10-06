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

    /// `lines` over each of `sums` that is more than nothing, and `total`
    /// under them all; no sums where one would only repeat the total.
    fn funds(&self, lines: &Amounts, sums: &[(&str, Dollars)], total: Dollars) -> Funds {
        let mut sums: Vec<_> = sums.iter().filter(|&&(_, sum)| sum != 0).collect();
        if matches!(sums[..], [&(_, only)] if only == total) {
            sums.clear();
        }
        let said = |&(ref label, amount): &(String, Dollars)| self.line(label, amount);
        Funds {
            lines: lines.iter().map(said).collect(),
            sums: (sums.iter())
                .map(|&&(kind, sum)| self.line(kind, sum))
                .collect(),
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

/// What `row` lived on: its income by source, then what it drew from each
/// account - a required distribution named as one - and what it could not
/// find, over what the income and the withdrawals each come to.
#[must_use]
pub fn money_in(plan: &Plan, row: &YearRow, is_nominal: bool) -> Funds {
    let saying = Saying {
        deflator: row.deflator,
        is_nominal,
    };
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
    let sums = [("Income", row.total_income), ("Withdrawn", drawn)];
    saying.funds(&lines, &sums, row.total_income + drawn + row.unfunded)
}

/// Where what `row` lived on went: each expense that spent anything, in the
/// plan's order, then its tax and Medicare's surcharges, what the household
/// paid into its accounts, and what was left over and saved - over what the
/// spending comes to, by kind where it is of more than one.
#[must_use]
pub fn money_out(plan: &Plan, row: &YearRow, is_nominal: bool) -> Funds {
    let saying = Saying {
        deflator: row.deflator,
        is_nominal,
    };
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
    let sums: &[(&str, Dollars)] = if is_split {
        &kinds
    } else {
        &[(SPENDING, row.expenses)]
    };
    let put_away = row.medicare + row.contributions_employee + row.surplus;
    saying.funds(&lines, sums, row.expenses + row.taxes.total + put_away)
}
