//! The plan's expenses estimated from a bank's exports: each row sorted
//! into a category by the bank's own column or by its payee's words, the
//! latest twelve months' spending in each annualized in the plan's dollars,
//! and each category proposed as the amount of the expense that is for it,
//! or as a new expense where the plan has none.

mod bank;
mod category;
mod csv;
mod ofx;
mod patterns;
mod payee;

use std::collections::BTreeMap;

pub use bank::{BankError, Cents, Row, merge, read};
pub use category::Category;
pub use patterns::Patterns;
pub use payee::normalize;

use serde::{Deserialize, Serialize};

use crate::plan::{ColaSpec, Dollars, Expense, Plan, PlanDate, Trigger};
use crate::project::trigger_month;
use crate::tax::month_index;

const MONTHS_PER_YEAR: i32 = 12;

/// What the exports say the plan spends.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Estimate {
    /// The whole months the window covers, twelve at most.
    pub months: u8,
    /// The latest row's date: the window ends with its month.
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub through: Option<PlanDate>,
    /// One per spending category seen, in the vocabulary's order.
    pub proposals: Vec<Proposal>,
    /// Each payee nothing named, largest first.
    pub unknown: Vec<Unknown>,
    /// What came in over the window, annualized in the plan's dollars.
    pub income: Dollars,
    /// What moved without being spent over the window, net, annualized.
    pub transfers: Dollars,
}

/// One category's spending as the amount of one expense.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Proposal {
    /// What the rows were for.
    pub category: Category,
    /// The plan expense it sets, or none to add one.
    pub expense: Option<String>,
    /// The expense's name, or the one a new expense is given.
    pub name: String,
    /// Annual, in the plan's start-year dollars.
    pub amount: Dollars,
    /// What the rows add up to over the window, in the same dollars.
    pub total: Dollars,
    /// How many rows.
    pub rows: usize,
    /// Each payee and what it adds up to, largest first.
    pub payees: Vec<(String, Dollars)>,
    /// The latest row's date.
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub last: PlanDate,
    /// Added as a one-time expense on `last` for `total`, not annualized.
    pub one_time: bool,
    /// Which of a phased expense's amounts the proposal sets.
    pub phases: Phases,
}

/// What a proposal does to an expense stated in phases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum Phases {
    /// The amount in force over the window alone.
    #[default]
    This,
    /// Every amount, by the factor the one in force changes by.
    Scaled,
    /// Every amount, to the proposed one.
    All,
}

/// A payee the patterns did not name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Unknown {
    /// The normalized payee.
    pub payee: String,
    /// Annualized, in the plan's dollars, negative where money came in.
    pub amount: Dollars,
    /// How many rows.
    pub rows: usize,
}

#[derive(Default)]
struct Tally {
    total: f64,
    rows: usize,
    payees: BTreeMap<String, f64>,
    last: Option<PlanDate>,
}

/// What `rows` say the plan spends, the latest twelve whole months taken
/// and deflated to the plan's start-year dollars by its inflation.
#[must_use]
pub fn estimate(plan: &Plan, rows: &[Row], patterns: &Patterns) -> Estimate {
    let Some(through) = rows.iter().map(|row| row.on).max() else {
        return Estimate::default();
    };
    let month = |on: PlanDate| month_index(on.year(), on.0.month());
    let end = month(through);
    let earliest = rows.iter().map(|row| month(row.on)).min().unwrap_or(end);
    let start = earliest.max(end - (MONTHS_PER_YEAR - 1));
    let months = end - start + 1;
    let annual = |cents: f64| {
        (cents * f64::from(MONTHS_PER_YEAR) / f64::from(months) / 100.0).round() as Dollars
    };
    let in_window = rows
        .iter()
        .filter(|row| (start..=end).contains(&month(row.on)));
    let (mut by_category, unknown) = tally(plan, in_window, patterns);
    let proposals = Category::ALL
        .into_iter()
        .filter(|category| category.is_spending())
        .filter_map(|category| by_category.remove(&category).map(|tally| (category, tally)))
        .filter(|(_, tally)| tally.total < 0.0)
        .map(|(category, tally)| proposal(plan, category, tally, annual))
        .collect();
    let mut unknown: Vec<Unknown> = unknown
        .into_iter()
        .map(|(payee, tally)| Unknown {
            payee,
            amount: annual(-tally.total),
            rows: tally.rows,
        })
        .collect();
    unknown.sort_by_key(|u| (-u.amount.abs(), u.payee.clone()));
    let mut tallied = |category| by_category.remove(&category).map_or(0, |t| annual(t.total));
    Estimate {
        months: months as u8,
        through: Some(through),
        proposals,
        unknown,
        income: tallied(Category::Income),
        transfers: tallied(Category::Transfer),
    }
}

/// Each row's deflated amount under its category, or under its payee
/// where nothing names one.
fn tally<'a>(
    plan: &Plan,
    rows: impl Iterator<Item = &'a Row>,
    patterns: &Patterns,
) -> (BTreeMap<Category, Tally>, BTreeMap<String, Tally>) {
    let mut by_category: BTreeMap<Category, Tally> = BTreeMap::new();
    let mut unknown: BTreeMap<String, Tally> = BTreeMap::new();
    for row in rows {
        let payee = normalize(&row.payee);
        let deflated = row.amount.0 as f64 / deflator(plan, row.on.year());
        let category = row
            .category
            .as_deref()
            .and_then(Category::from_text)
            .or_else(|| patterns.category(&payee));
        let tally = match category {
            Some(category) => by_category.entry(category).or_default(),
            None => unknown.entry(payee.clone()).or_default(),
        };
        tally.total += deflated;
        tally.rows += 1;
        *tally.payees.entry(payee).or_default() += deflated;
        tally.last = tally.last.max(Some(row.on));
    }
    (by_category, unknown)
}

fn proposal(
    plan: &Plan,
    category: Category,
    tally: Tally,
    annual: impl Fn(f64) -> Dollars,
) -> Proposal {
    let expense = plan.expenses.iter().find(|e| category.matches(e));
    let mut payees: Vec<(String, Dollars)> = tally
        .payees
        .into_iter()
        .map(|(payee, cents)| (payee, (-cents / 100.0).round() as Dollars))
        .collect();
    payees.sort_by_key(|(payee, amount)| (-*amount, payee.clone()));
    Proposal {
        category,
        expense: expense.map(|e| e.id.clone()),
        name: expense
            .and_then(|e| e.name.clone())
            .unwrap_or_else(|| category.title()),
        amount: annual(-tally.total),
        total: (-tally.total / 100.0).round() as Dollars,
        rows: tally.rows,
        payees,
        last: tally.last.expect("a tally holds a row"),
        one_time: false,
        phases: Phases::default(),
    }
}

/// What a dollar of `year` is in start-year dollars: the plan's inflation
/// compounded over the years between, as the projection's deflator holds
/// it for the years it walks.
fn deflator(plan: &Plan, year: i16) -> f64 {
    (1.0 + plan.plan.inflation).powi(i32::from(year - plan.plan.start_year))
}

/// Sets each proposal's expense to its amount, or adds the expense the plan
/// has none for; the proposals left as they came set nothing they did not
/// propose.
pub fn apply(plan: &mut Plan, estimate: &Estimate) {
    let end = estimate
        .through
        .map(|on| month_index(on.year(), on.0.month() + 1) - 1);
    for proposal in &estimate.proposals {
        let at = proposal
            .expense
            .as_deref()
            .and_then(|id| plan.expenses.iter().position(|e| e.id == *id));
        match at {
            Some(at) => {
                let in_force = end.and_then(|end| phase_in_force(plan, &plan.expenses[at], end));
                set_amount(&mut plan.expenses[at], proposal, in_force);
            }
            None => plan.expenses.push(new_expense(proposal)),
        }
    }
}

/// The index of the last phase fired by the window's `end` month, or none
/// where the base amount is still in force.
fn phase_in_force(plan: &Plan, expense: &Expense, end: i32) -> Option<usize> {
    expense
        .phases
        .iter()
        .rposition(|phase| trigger_month(plan, &phase.from).is_some_and(|month| month <= end))
}

fn set_amount(expense: &mut Expense, proposal: &Proposal, in_force: Option<usize>) {
    let current = in_force.map_or(expense.amount, |at| expense.phases[at].amount);
    let scale = |amount: Dollars| {
        if current == 0 {
            proposal.amount
        } else {
            (amount as f64 * proposal.amount as f64 / current as f64).round() as Dollars
        }
    };
    match proposal.phases {
        Phases::This => match in_force {
            Some(at) => expense.phases[at].amount = proposal.amount,
            None => expense.amount = proposal.amount,
        },
        Phases::Scaled => {
            expense.amount = scale(expense.amount);
            for phase in &mut expense.phases {
                phase.amount = scale(phase.amount);
            }
        }
        Phases::All => {
            expense.amount = proposal.amount;
            for phase in &mut expense.phases {
                phase.amount = proposal.amount;
            }
        }
    }
}

fn new_expense(proposal: &Proposal) -> Expense {
    Expense {
        id: proposal.category.slug().to_owned(),
        name: Some(proposal.name.clone()),
        amount: if proposal.one_time {
            proposal.total
        } else {
            proposal.amount
        },
        start: None,
        end: None,
        on: proposal.one_time.then_some(Trigger {
            date: Some(proposal.last),
            age: None,
            owner: None,
            event: None,
            income: None,
            offset: None,
        }),
        cola: ColaSpec::default(),
        medical: false,
        essential: false,
        phases: Vec::new(),
    }
}
