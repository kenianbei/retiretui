//! State income tax. The rule shape lives here; each state's values come
//! from [`crate::params::StateParams`].

use super::walk_brackets;
use crate::params::{Source, StateParams};
use crate::plan::{Dollars, FilingStatus, PlanDate};

/// One person's taxable ordinary income in a year, by where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonIncome {
    /// The person's birth date, which an age a state's table names is
    /// counted from.
    pub birth: PlanDate,
    /// The year's income by [`Source`], in the order of [`Source::ALL`].
    pub by_source: [Dollars; Source::COUNT],
}

impl PersonIncome {
    /// Someone born on `birth` who has received nothing yet.
    #[must_use]
    pub const fn new(birth: PlanDate) -> Self {
        Self {
            birth,
            by_source: [0; Source::COUNT],
        }
    }

    /// Adds `amount`, which may be negative, to what came from `source`.
    pub const fn add(&mut self, source: Source, amount: Dollars) {
        self.by_source[source as usize] += amount;
    }

    /// The income from every source together.
    #[must_use]
    pub fn total(&self) -> Dollars {
        self.by_source.iter().sum()
    }
}

/// A state's income tax: ordinary income and gains alike, with the taxable
/// share of Social Security where the state taxes it, less its deduction.
#[must_use]
pub fn state_tax(
    state: &StateParams,
    status: FilingStatus,
    income: Dollars,
    taxable_social_security: Dollars,
) -> Dollars {
    let benefits = if state.taxes_social_security {
        taxable_social_security
    } else {
        0
    };
    let taxable = income + benefits - state.deduction.get(status);
    walk_brackets(state.brackets.for_status(status), taxable)
}
