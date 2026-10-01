//! What a state takes: its income tax and its excise on gains. The rule
//! shape lives here; each state's values come from
//! [`crate::params::StateParams`].

use super::{is_age_reached, walk_brackets};
use crate::params::{FederalTaxSubtraction, PerStatus, Source, StateParams};
use crate::plan::{Dollars, FilingStatus, PlanDate};

const SOURCES: usize = Source::ALL.len();

/// The age from which a state's table adds to a person's deduction or
/// credit, as the federal standard deduction does.
const ADDITION_AGE: f64 = 65.0;

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
    /// Federal income tax: on ordinary income and on gains, and the
    /// additional taxes on early withdrawals.
    pub federal_tax: Dollars,
    /// Federal adjusted gross income: the figure
    /// [`Taxes::magi`](crate::project::Taxes::magi) reports.
    pub agi: Dollars,
}

/// What a state takes in a year. Its income tax is on what each person's
/// ordinary income leaves once the state's exclusions are taken out of it,
/// gains, and the taxable share of Social Security where the state taxes it,
/// less what was deferred where the state follows that, its deduction, and
/// the federal tax it lets be subtracted; its credit for each person comes
/// off that tax and is never refunded. Its excise on gains is added.
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
    let taxable = ordinary + income.gains - deferred + benefits
        - deduction_of(state, status, income)
        - subtracted_of(state, status, income);
    let walked = walk_brackets(state.brackets.for_status(status), taxable);
    (walked - credit_of(state, status, income)).max(0) + excise_of(state, income.gains)
}

fn deduction_of(state: &StateParams, status: FilingStatus, income: &StateIncome) -> Dollars {
    if is_over(state.deduction_until_agi, status, income.agi) {
        return 0;
    }
    state.deduction.get(status) + added_at_65(state.deduction_at_65.get(status), income)
}

/// What the people old enough add between them, at `each`.
fn added_at_65(each: Dollars, income: &StateIncome) -> Dollars {
    if each == 0 {
        return 0;
    }
    let people = income.people.iter();
    let aged = people.filter(|person| is_age_reached(person.birth, ADDITION_AGE, income.year));
    each * aged.count() as Dollars
}

fn is_over(until_agi: Option<PerStatus<Dollars>>, status: FilingStatus, agi: Dollars) -> bool {
    until_agi.is_some_and(|limit| agi > limit.get(status))
}

fn subtracted_of(state: &StateParams, status: FilingStatus, income: &StateIncome) -> Dollars {
    let subtraction = state.federal_tax_subtraction.as_ref();
    subtraction.map_or(0, |subtraction| {
        income
            .federal_tax
            .min(cap_at(subtraction, status, income.agi))
    })
}

/// The cap an AGI leaves: whole under the band, and less by one part in
/// `steps` at its foot and at each even step up to its top.
fn cap_at(subtraction: &FederalTaxSubtraction, status: FilingStatus, agi: Dollars) -> Dollars {
    let band = subtraction.phase_out.get(status);
    if agi < band.from {
        return subtraction.cap;
    }
    let steps = Dollars::from(subtraction.steps.get());
    let width = (band.to - band.from).max(1);
    let lost = (1 + (agi - band.from) * (steps - 1) / width).min(steps);
    subtraction.cap * (steps - lost) / steps
}

fn credit_of(state: &StateParams, status: FilingStatus, income: &StateIncome) -> Dollars {
    let Some(credit) = &state.exemption_credit else {
        return 0;
    };
    if is_over(credit.until_agi, status, income.agi) {
        return 0;
    }
    credit.per_person * income.people.len() as Dollars + added_at_65(credit.at_65, income)
}

fn excise_of(state: &StateParams, gains: Dollars) -> Dollars {
    let excise = state.gains_excise.as_ref();
    excise.map_or(0, |excise| {
        walk_brackets(&excise.brackets, gains - excise.deduction)
    })
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
