//! The proposals applied: each expense the plan holds set to its amount -
//! in the phase in force, every phase scaled, or every phase alike - and
//! each one it has none for added.

use super::sort::month_of;
use super::{Estimate, Phases, Proposal};
use crate::params::scale;
use crate::plan::{ColaSpec, Dollars, Expense, Plan, Trigger};
use crate::project::trigger_month;

/// Sets each proposal's expense to its amount, or adds the expense the plan
/// has none for; the proposals left as they came set nothing they did not
/// propose.
pub fn apply(plan: &mut Plan, estimate: &Estimate) {
    let end = estimate.through.map(month_of);
    for proposal in &estimate.proposals {
        let at = (!proposal.is_new)
            .then(|| plan.expenses.iter().position(|e| e.id == proposal.id))
            .flatten();
        match at {
            Some(at) => {
                let in_force = end.and_then(|end| phase_in_force(plan, &plan.expenses[at], end));
                set_amount(&mut plan.expenses[at], proposal, in_force);
            }
            None => plan.expenses.push(new_expense(proposal)),
        }
    }
}

/// What `expense` spends a year in the month `end`.
pub(super) fn in_force_amount(plan: &Plan, expense: &Expense, end: i32) -> Dollars {
    phase_in_force(plan, expense, end).map_or(expense.amount, |at| expense.phases[at].amount)
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
            scale(amount, proposal.amount as f64 / current as f64)
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
        id: proposal.id.clone(),
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
