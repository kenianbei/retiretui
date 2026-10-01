//! State income tax. The rule shape lives here; each state's values come
//! from [`crate::params::StateParams`].

use super::walk_brackets;
use crate::params::StateParams;
use crate::plan::{Dollars, FilingStatus};

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
