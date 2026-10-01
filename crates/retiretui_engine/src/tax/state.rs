//! State income tax. The rule shape lives here; each state's values come
//! from [`crate::params::StateParams`].

use super::{is_age_reached, walk_brackets};
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

/// What a year's state income tax is figured from.
#[derive(Debug, Clone, Copy)]
pub struct StateIncome<'a> {
    /// The calendar year, which the ages a state's table names are read in.
    pub year: i16,
    /// The household's people, each with their taxable ordinary income.
    pub people: &'a [PersonIncome],
    /// Realized long-term gains.
    pub gains: Dollars,
    /// The federally taxable share of Social Security.
    pub taxable_social_security: Dollars,
    /// Traditional IRA contributions the year's MAGI let be deducted.
    pub ira_deducted: Dollars,
}

/// A state's income tax: what each person's ordinary income leaves once the
/// state's exclusions are taken out of it, gains, and the taxable share of
/// Social Security where the state taxes it, less its deduction.
#[must_use]
pub fn state_tax(state: &StateParams, status: FilingStatus, income: &StateIncome) -> Dollars {
    let people = income.people.iter();
    let ordinary: Dollars = people
        .map(|person| taxed_of(state, person, income.year))
        .sum();
    let ira_deducted = if state.taxes_deferrals {
        0
    } else {
        income.ira_deducted
    };
    let benefits = if state.taxes_social_security {
        income.taxable_social_security
    } else {
        0
    };
    let taxable = ordinary + income.gains - ira_deducted + benefits - state.deduction.get(status);
    walk_brackets(state.brackets.for_status(status), taxable)
}

/// What of a person's ordinary income the state taxes in `year`. A row
/// exempts what a source holds and no more, so one that reduces income is
/// left as it is.
fn taxed_of(state: &StateParams, person: &PersonIncome, year: i16) -> Dollars {
    let mut by_source = person.by_source;
    if state.taxes_deferrals {
        by_source[Source::Deferral as usize] = 0;
    }
    for row in &state.exclusions {
        if row
            .from_age
            .is_some_and(|age| !is_age_reached(person.birth, age, year))
        {
            continue;
        }
        for &source in &row.sources {
            let held = &mut by_source[source as usize];
            *held = (*held).min(0);
        }
    }
    by_source.iter().sum()
}
