//! State income tax. The rule shape lives here; each state's values come
//! from [`crate::params::StateParams`].

use super::{is_age_reached, walk_brackets};
use crate::params::{Source, StateParams};
use crate::plan::{Dollars, FilingStatus, PlanDate};

const SOURCES: usize = Source::ALL.len();

/// One person's taxable ordinary income in a year, by where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonIncome {
    birth: PlanDate,
    by_source: [Dollars; SOURCES],
}

impl PersonIncome {
    /// Someone born on `birth`, which an age a state's table names is
    /// counted from, who has received nothing yet.
    #[must_use]
    pub fn new(birth: PlanDate) -> Self {
        Self {
            birth,
            by_source: [0; SOURCES],
        }
    }

    /// Adds `amount`, which may be negative, to what came from `source`.
    pub fn add(&mut self, source: Source, amount: Dollars) {
        self.by_source[source as usize] += amount;
    }

    /// The income from every source together.
    #[must_use]
    pub fn total(&self) -> Dollars {
        self.by_source.iter().sum()
    }
}

/// What a year's state income tax is figured from.
#[derive(Debug)]
pub struct StateIncome<'a> {
    /// The calendar year, which the ages a state's table names are read in.
    pub year: i16,
    /// The household's people, each with their taxable ordinary income.
    pub people: &'a [PersonIncome],
    /// Realized long-term gains.
    pub gains: Dollars,
    /// The federally taxable share of Social Security.
    pub taxable_social_security: Dollars,
    /// What was paid into tax-deferred accounts and deducted: pre-tax
    /// deferrals, and the traditional IRA contributions the year's MAGI let
    /// be deducted.
    pub deferred: Dollars,
}

/// A state's income tax: what each person's ordinary income leaves once the
/// state's exclusions are taken out of it, gains, and the taxable share of
/// Social Security where the state taxes it, less what was deferred where
/// the state follows that, and less its deduction.
#[must_use]
pub fn state_tax(state: &StateParams, status: FilingStatus, income: &StateIncome) -> Dollars {
    let people = income.people.iter();
    let ordinary: Dollars = people
        .map(|person| taxed_of(state, person, income.year))
        .sum();
    let deferred = if state.taxes_deferrals {
        0
    } else {
        income.deferred
    };
    let benefits = if state.taxes_social_security {
        income.taxable_social_security
    } else {
        0
    };
    let taxable = ordinary + income.gains - deferred + benefits - state.deduction.get(status);
    walk_brackets(state.brackets.for_status(status), taxable)
}

/// What of a person's ordinary income the state taxes in `year`. A row
/// exempts what a source holds and no more, so one that reduces income is
/// left as it is.
fn taxed_of(state: &StateParams, person: &PersonIncome, year: i16) -> Dollars {
    let mut by_source = person.by_source;
    for row in &state.exclusions {
        if !row
            .from_age
            .is_none_or(|age| is_age_reached(person.birth, age, year))
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
