//! What each account earns: a fixed rate, or the year's market through
//! the mix it holds that year.

use crate::plan::{Account, Allocation, ClassReturns, Dollars, Mix};

use super::resolve::Resolver;
use super::year::{Simulation, YearAcc};
use super::{Plan, scale};

/// How an account's return is found, its triggers resolved once.
pub(super) enum Holding {
    /// The same rate every year.
    Fixed(f64),
    /// Glide-path steps as the years they fire, in the plan's order; a
    /// step that never fires is never held.
    Mixes(Vec<(Option<i16>, Mix)>),
}

impl Holding {
    pub(super) fn of(plan: &Plan, resolver: &Resolver, account: &Account) -> Self {
        match &account.allocation {
            None => Self::Fixed(account.expected_return.unwrap_or_default()),
            Some(Allocation::Mix(mix)) => Self::Mixes(vec![(None, *mix)]),
            Some(Allocation::GlidePath(phases)) => Self::Mixes(
                phases
                    .iter()
                    .map(|phase| (resolver.trigger_year(plan, &phase.from), phase.mix()))
                    .collect(),
            ),
        }
    }

    /// The step in force in `year`: the one fired latest by then, a later
    /// step winning a tie; before any has fired, the first.
    fn mix_in(steps: &[(Option<i16>, Mix)], year: i16) -> Option<Mix> {
        steps
            .iter()
            .filter(|(fired, _)| fired.is_some_and(|fired| fired <= year))
            .max_by_key(|(fired, _)| *fired)
            .or(steps.first())
            .map(|&(_, mix)| mix)
    }

    fn rate(&self, year: i16, returns: &ClassReturns) -> f64 {
        match self {
            Self::Fixed(rate) => *rate,
            Self::Mixes(steps) => Self::mix_in(steps, year).map_or(0.0, |mix| mix.blend(returns)),
        }
    }
}

impl Simulation<'_> {
    /// The year's growth on what each account held at its open, credited
    /// before any flow clamps to the balance, so an account emptied this
    /// year ends it at zero rather than holding growth on what it gave.
    pub(super) fn grow(&mut self, year: i16, snapshot: &[Dollars], acc: &mut YearAcc) {
        let returns = self.path.returns(year);
        for (index, account) in self.plan.accounts.iter().enumerate() {
            let growth = scale(snapshot[index], self.holdings[index].rate(year, &returns));
            self.balances[index] += growth;
            if growth != 0 {
                acc.growth.insert(account.id.clone(), growth);
            }
        }
    }
}
