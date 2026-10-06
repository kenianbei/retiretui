//! Each account's year from its open to its close, with what came in and
//! went out named by where from or to, and every account as one.

use retiretui_engine::plan::{Dollars, Item, Plan};
use retiretui_engine::project::{Action, ContributionNote, YearRow};
use serde::Serialize;

use crate::actions::note_phrase;
use crate::present::{account_name, money};
use crate::table::basis_amount;

const NOTE_JOIN: &str = " · ";
/// Said after a conversion's counterpart, where a narrow pane clips first.
const CONVERSION: &str = " (conversion)";
const PERCENT: f64 = 100.0;
/// What the row for every account together is named.
const ALL_ACCOUNTS: &str = "All accounts";

/// The year's flows through each account, as every surface titles them.
pub const FLOWS: &str = "Flows";
/// The flows table's columns, each beside whether it holds figures.
pub const FLOW_COLUMNS: [(&str, bool); 5] = [
    ("Account", false),
    ("Open", true),
    ("Moves", false),
    ("Growth", true),
    ("Close", true),
];

/// An account's year, said: what it opened on, what came in and went out -
/// each named by where from or to - what it grew, and what it closed on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AccountFlows {
    /// The account's display name.
    pub account: String,
    /// Its balance as the year opened.
    pub open: String,
    /// What came in, said.
    pub ins: Vec<String>,
    /// What went out, said.
    pub outs: Vec<String>,
    /// What came in and then what went out, said.
    pub moves: Vec<String>,
    /// What it grew, signed; blank where it did not.
    pub growth: String,
    /// What it grew over what it opened on; blank where it opened empty.
    pub growth_rate: String,
    /// Its balance as the year closed.
    pub close: String,
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
    let show = |amount: Dollars| money(basis_amount(amount, row.deflator, is_nominal));
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
                open: show(open),
                moves: ins.iter().chain(&outs).cloned().collect(),
                ins,
                outs,
                growth: signed(growth, &show),
                growth_rate: growth_rate(growth, open),
                close: show(close),
            })
        })
        .collect()
}

/// Every account as one: what the household held as the year opened and
/// closed, what came into its accounts from outside them and left them, and
/// what they grew. A move between two accounts is in neither.
#[must_use]
pub fn all_accounts(
    plan: &Plan,
    previous: Option<&YearRow>,
    row: &YearRow,
    is_nominal: bool,
) -> AccountFlows {
    let show = |amount: Dollars| money(basis_amount(amount, row.deflator, is_nominal));
    let held = || plan.accounts.iter().map(|account| account.balance).sum();
    let open = previous.map_or_else(held, |before| before.net_worth);
    let growth = row.growth.values().sum();
    let paid_in = row.contributions_employee + row.contributions_employer + row.surplus;
    let from_outside = [(paid_in, "+"), (-row.total_withdrawals(), "")];
    let moves = from_outside.into_iter().filter(|&(amount, _)| amount != 0);
    AccountFlows {
        account: ALL_ACCOUNTS.to_owned(),
        open: show(open),
        ins: Vec::new(),
        outs: Vec::new(),
        moves: moves
            .map(|(amount, sign)| format!("{sign}{}", show(amount)))
            .collect(),
        growth: signed(growth, &show),
        growth_rate: growth_rate(growth, open),
        close: show(row.net_worth),
    }
}

/// `growth` over `open` as a percent to a tenth; blank over nothing.
fn growth_rate(growth: Dollars, open: Dollars) -> String {
    if open <= 0 {
        return String::new();
    }
    format!("{:.1}%", growth as f64 / open as f64 * PERCENT)
}

fn signed(amount: Dollars, show: &impl Fn(Dollars) -> String) -> String {
    match amount {
        0 => String::new(),
        ..0 => show(amount),
        _ => format!("+{}", show(amount)),
    }
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
