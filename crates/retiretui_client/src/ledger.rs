//! What a ledger says of a year: its headers, each account's flows from
//! its open to its close, the income and what was paid, and the years
//! each earner's salary ends.

use std::collections::BTreeMap;

use retiretui_engine::plan::{Dollars, IncomeKind, Item, Plan, TreatmentClass};
use retiretui_engine::project::{Action, ContributionNote, YearRow};
use serde::Serialize;

use crate::actions::note_phrase;
use crate::present::{account_name, income_name, money, treatment_class};
use crate::session::Projected;
use crate::table::basis_amount;

const NOTE_JOIN: &str = " · ";
/// Said after a conversion's counterpart, where a narrow pane clips first.
const CONVERSION: &str = " (conversion)";
const LEADING_HEADERS: [&str; 6] = ["Year", "Age", "Income", "Spending", "Tax", "Withdrawn"];
const NET_WORTH: &str = "Net worth";

/// The year table's headers: the year, ages and the figures
/// [`crate::table::year_figures`] gives over `classes`.
#[must_use]
pub fn ledger_headers(classes: &[TreatmentClass]) -> Vec<&'static str> {
    let classes = classes.iter().map(|&class| treatment_class(class));
    LEADING_HEADERS
        .into_iter()
        .chain(classes)
        .chain([NET_WORTH])
        .collect()
}

/// An account's year: what it opened on, what came in and went out -
/// each named by where from or to - what it grew, and what it closed on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AccountFlows {
    /// The account's display name.
    pub account: String,
    /// Its balance as the year opened.
    pub open: Dollars,
    /// What came in, said.
    pub ins: Vec<String>,
    /// What went out, said.
    pub outs: Vec<String>,
    /// What it grew.
    pub growth: Dollars,
    /// Its balance as the year closed.
    pub close: Dollars,
}

/// Each account the year touches, in the plan's order. Every figure is on
/// `row`'s own basis, so an account's year adds up; the first year opens
/// on the plan's balances.
#[must_use]
pub fn account_flows(
    plan: &Plan,
    previous: Option<&YearRow>,
    row: &YearRow,
    is_nominal: bool,
) -> Vec<AccountFlows> {
    let at_basis = |amount: Dollars| basis_amount(amount, row.deflator, is_nominal);
    let show = |amount: Dollars| money(at_basis(amount));
    plan.accounts
        .iter()
        .filter_map(|account| {
            let id = account.id.as_str();
            let open = previous.map_or(account.balance, |before| balance(before, id));
            let close = balance(row, id);
            let growth = row.growth.get(id).copied().unwrap_or(0);
            let (ins, outs) = moves(plan, row, id, &show);
            let is_idle = open == 0 && close == 0 && growth == 0;
            (!is_idle || !ins.is_empty() || !outs.is_empty()).then(|| AccountFlows {
                account: account.display_name().to_owned(),
                open: at_basis(open),
                ins,
                outs,
                growth: at_basis(growth),
                close: at_basis(close),
            })
        })
        .collect()
}

fn balance(row: &YearRow, id: &str) -> Dollars {
    row.balances.get(id).copied().unwrap_or(0)
}

fn moves(
    plan: &Plan,
    row: &YearRow,
    id: &str,
    show: &impl Fn(Dollars) -> String,
) -> (Vec<String>, Vec<String>) {
    let mut ins = Vec::new();
    let mut outs = Vec::new();
    for action in &row.actions {
        let between = match action {
            Action::Transfer { from, to, amount } => Some((from, to, amount, "")),
            Action::Conversion { from, to, amount } => Some((from, to, amount, CONVERSION)),
            _ => None,
        };
        if let Some((from, to, amount, kind)) = between {
            if to == id {
                let from = account_name(plan, from);
                ins.push(format!("+{} ← {from}{kind}", show(*amount)));
            }
            if from == id {
                let to = account_name(plan, to);
                outs.push(format!("-{} → {to}{kind}", show(*amount)));
            }
            continue;
        }
        match action {
            Action::Contribution {
                account,
                employee,
                employer,
                notes,
            } if account == id => ins.extend(paid_in(plan, (*employee, *employer), notes, show)),
            Action::Rmd { account, amount } if account == id => {
                outs.push(format!("-{} RMD", show(*amount)));
            }
            Action::Withdrawal { account, amount } if account == id => {
                outs.push(format!("-{} for spending", show(*amount)));
            }
            Action::Surplus { account, amount } if account == id => {
                ins.push(format!("+{} surplus", show(*amount)));
            }
            _ => {}
        }
    }
    (ins, outs)
}

/// A contribution's lines: the employee's and the employer's, each with the
/// notes that say how it came to be.
fn paid_in(
    plan: &Plan,
    (yours, theirs): (Dollars, Dollars),
    notes: &[ContributionNote],
    show: &impl Fn(Dollars) -> String,
) -> Vec<String> {
    let with_notes = |said: String, is_employer: bool| {
        let phrases = notes
            .iter()
            .filter(|note| is_employer_note(note) == is_employer)
            .map(|note| note_phrase(plan, note));
        std::iter::once(said)
            .chain(phrases)
            .collect::<Vec<_>>()
            .join(NOTE_JOIN)
    };
    let mut lines = Vec::new();
    if yours > 0 {
        lines.push(with_notes(format!("+{} yours", show(yours)), false));
    }
    if theirs > 0 {
        lines.push(with_notes(format!("+{} employer", show(theirs)), true));
    }
    lines
}

const fn is_employer_note(note: &ContributionNote) -> bool {
    matches!(
        note,
        ContributionNote::Match { .. } | ContributionNote::HeldToOverall
    )
}

/// One labelled amount of a year's income or of what it paid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DetailLine {
    /// What the amount is.
    pub label: String,
    /// The amount, on the basis asked for.
    pub amount: Dollars,
}

/// The year's income by source, and then its spending and what it paid
/// besides - a line only where it paid any - on the basis asked for.
#[must_use]
pub fn income_and_tax(
    plan: &Plan,
    row: &YearRow,
    is_nominal: bool,
) -> (Vec<DetailLine>, Vec<DetailLine>) {
    let line = |label: &str, amount: Dollars| DetailLine {
        label: label.to_owned(),
        amount: basis_amount(amount, row.deflator, is_nominal),
    };
    let income = row
        .income
        .iter()
        .map(|(source, &amount)| line(income_name(plan, source), amount))
        .collect();
    let paid = [
        ("Spending", row.expenses),
        ("Ordinary tax", row.taxes.ordinary),
        ("State tax", row.taxes.state),
        ("Capital gains", row.taxes.ltcg),
        ("Penalties", row.taxes.penalty),
        ("Medicare", row.medicare),
        ("Surplus", row.surplus),
        ("Unfunded", row.unfunded),
    ]
    .into_iter()
    .filter(|&(_, amount)| amount != 0)
    .map(|(label, amount)| line(label, amount))
    .collect();
    (income, paid)
}

/// Each earner with the first year none of their salaries pays, earliest
/// first and then by owner; an earner paid to the horizon has none.
#[must_use]
pub fn salary_ends(projected: &Projected) -> Vec<(&str, i16)> {
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

/// [`salary_ends`] as a chart marks them: "retire" where one earner has
/// an end, each earner's name where several do.
#[must_use]
pub fn salary_marks(projected: &Projected) -> Vec<(String, i16)> {
    let ends = salary_ends(projected);
    let is_only_earner = ends.len() == 1;
    ends.into_iter()
        .map(|(owner, year)| {
            let label = if is_only_earner {
                format!("retire {year}")
            } else {
                format!("{} {year}", projected.plan.person_name(owner))
            };
            (label, year)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use retiretui_engine::params::TaxTables;
    use retiretui_engine::project::project;

    use super::*;

    fn full() -> Projected {
        let plan = Plan::from_toml_str(include_str!(
            "../../retiretui_engine/tests/fixtures/full.toml"
        ))
        .unwrap();
        let projection = project(&plan, &TaxTables::embedded());
        Projected { plan, projection }
    }

    #[test]
    fn headers_frame_the_classes() {
        let headers = ledger_headers(&[TreatmentClass::Roth]);
        assert_eq!(headers[..2], ["Year", "Age"]);
        assert_eq!(headers[6], treatment_class(TreatmentClass::Roth));
        assert_eq!(headers.last(), Some(&NET_WORTH));
    }

    #[test]
    fn the_closes_add_up_to_the_balances() {
        let projected = full();
        let years = &projected.projection.years;
        for (at, row) in years.iter().enumerate() {
            let previous = at.checked_sub(1).map(|before| &years[before]);
            let flows = account_flows(&projected.plan, previous, row, true);
            let closed: Dollars = flows.iter().map(|flow| flow.close).sum();
            assert_eq!(
                closed,
                row.balances.values().sum::<Dollars>(),
                "{}",
                row.year
            );
        }
    }

    #[test]
    fn income_comes_before_what_was_paid_and_follows_the_basis() {
        let projected = full();
        let row = &projected.projection.years[5];
        let (income, paid) = income_and_tax(&projected.plan, row, true);
        assert!(!income.is_empty());
        assert_eq!(paid[0].label, "Spending");
        assert!(paid.iter().all(|line| line.amount != 0));
        let (deflated, _) = income_and_tax(&projected.plan, row, false);
        assert_ne!(income, deflated);
    }
}
