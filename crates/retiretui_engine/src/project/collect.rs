//! What the year reads from the plan into its accumulator - income,
//! expenses, Medicare surcharges, cliff costs - touching no balance.

use crate::params::TaxParams;
use crate::plan::{Cliff, Dollars, IncomeKind, Plan};
use crate::tax;

use super::resolve::Resolver;
use super::year::{Simulation, YearAcc};
use super::{Taxes, scale};

impl Simulation<'_> {
    pub(super) fn collect_income(&mut self, year: i16, acc: &mut YearAcc) {
        let plan = self.plan;
        for (i, income) in plan.income.iter().enumerate() {
            if !self.item_active(year, income.span()) {
                continue;
            }
            let amount = match income.amount {
                Some(amount) => amount,
                None => self.derived_benefit(i, year),
            };
            let escalated = scale(amount, self.cola_factor(income.cola, year));
            let nominal = self.social_security_paid(income, year, escalated);
            if nominal <= 0 {
                continue;
            }
            *acc.income.entry(income.id.clone()).or_default() += nominal;
            acc.cash += nominal;
            match income.kind {
                IncomeKind::SocialSecurity => acc.ss_gross += nominal,
                IncomeKind::Windfall => {}
                IncomeKind::Salary => {
                    self.record_covered(&income.owner, year, nominal);
                    acc.ordinary += nominal;
                }
                _ => acc.ordinary += nominal,
            }
        }
    }

    pub(super) fn collect_expenses(&mut self, year: i16, acc: &mut YearAcc) {
        for expense in &self.plan.expenses {
            if self.item_active(year, expense.span()) {
                acc.expenses += scale(expense.amount, self.cola_factor(expense.cola, year));
            }
        }
    }

    /// IRMAA surcharges for every covered person, from the household MAGI
    /// two years back; nothing without an opted-in `[medicare]` section.
    pub(super) fn collect_medicare(&mut self, year: i16, params: &TaxParams, acc: &mut YearAcc) {
        let Some(medicare) = &self.plan.medicare else {
            return;
        };
        let covered = super::covered_count(self.plan, year);
        if covered == 0 {
            return;
        }
        let per_person = tax::irmaa_surcharge(
            params,
            self.plan.household.filing,
            self.magi_lookback[0],
            medicare.part_d,
        );
        acc.medicare += per_person * covered as Dollars;
    }

    /// The cliff costs the year's MAGI incurs, thresholds and costs
    /// escalated per item. Recomputed inside the settle iteration because
    /// paying a cliff can force withdrawals that raise MAGI further.
    pub(super) fn cliff_costs(&self, year: i16, taxes: &Taxes) -> Dollars {
        self.plan
            .cliffs
            .iter()
            .filter_map(|cliff| {
                let is_active = is_cliff_active(
                    self.plan,
                    &self.resolver,
                    self.default_cliff_end,
                    cliff,
                    year,
                );
                let factor = self.cola_factor(cliff.cola, year);
                (is_active && taxes.magi > scale(cliff.magi_over, factor))
                    .then(|| scale(cliff.cost, factor))
            })
            .sum()
    }
}

pub(super) fn seed_magi_lookback(plan: &Plan) -> [Dollars; 2] {
    let mut lookback = [0; 2];
    if let Some(medicare) = &plan.medicare {
        // Entries are oldest first, so they fill the newest slots.
        for (slot, &magi) in lookback
            .iter_mut()
            .rev()
            .zip(medicare.prior_magi.iter().rev())
        {
            *slot = magi;
        }
    }
    lookback
}

/// Whether the cliff's window is open in `year`, shared by the projection
/// and the optimizer's ceilings so the two cannot diverge.
pub(super) fn is_cliff_active(
    plan: &Plan,
    resolver: &Resolver,
    default_end: i16,
    cliff: &Cliff,
    year: i16,
) -> bool {
    if cliff.end.is_none() && year > default_end {
        return false;
    }
    resolver.is_active(plan, year, cliff.span())
}

/// A cliff with no `end` lapses when the youngest person reaches Medicare
/// age - the marketplace window closes with the last enrollee.
pub(super) fn default_cliff_end(plan: &Plan) -> i16 {
    super::latest_birthday_year(plan, tax::MEDICARE_AGE) - 1
}
