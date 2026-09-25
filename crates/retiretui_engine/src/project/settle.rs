use crate::params::{StateParams, TaxParams};
use crate::plan::{Dollars, FilingStatus, Person, TreatmentClass};
use crate::tax;

use super::Taxes;
use super::residence;
use super::scale;
use super::year::{Simulation, YearAcc};

const MAX_TAX_ITERATIONS: usize = 30;
const PENALTY_FREE_YEARS: i64 = 59;
const PENALTY_FREE_MONTHS: i64 = 6;

impl Simulation<'_> {
    pub(super) fn settle_cash(
        &mut self,
        year: i16,
        params: &TaxParams,
        acc: &mut YearAcc,
    ) -> Taxes {
        let state = residence::state_in(self.plan, &self.resolver, year)
            .and_then(|code| params.states.get(code));
        let status = self.plan.household.filing;
        let mut taxes = compute_taxes(params, state, status, acc);
        acc.cliffs = self.cliff_costs(year, &taxes);
        let order = self.drain_candidates(year);
        for _ in 0..MAX_TAX_ITERATIONS {
            let need = acc.expenses + acc.employee + taxes.total + acc.medicare + acc.cliffs;
            let available = acc.cash + acc.drained_cash;
            let shortfall = need - available;
            if shortfall <= 0 {
                acc.surplus = -shortfall;
                return taxes;
            }
            if self.drain(&order, year, shortfall, acc) == 0 {
                acc.unfunded = shortfall;
                return taxes;
            }
            taxes = compute_taxes(params, state, status, acc);
            acc.cliffs = self.cliff_costs(year, &taxes);
        }
        let residual = acc.expenses + acc.employee + taxes.total + acc.medicare + acc.cliffs
            - acc.cash
            - acc.drained_cash;
        acc.unfunded = residual.max(0);
        acc.surplus = (-residual).max(0);
        taxes
    }

    fn drain(&mut self, order: &[usize], year: i16, want: Dollars, acc: &mut YearAcc) -> Dollars {
        let mut remaining = want;
        for penalized_pass in [false, true] {
            for &index in order {
                if remaining <= 0 {
                    break;
                }
                if self.is_penalized(index, year) != penalized_pass {
                    continue;
                }
                remaining -= self.withdraw(index, remaining, year, acc);
            }
        }
        want - remaining
    }

    fn drain_candidates(&self, year: i16) -> Vec<usize> {
        let unlocked = |index: &usize| self.is_unlocked(*index, year);
        let mut prioritized: Vec<usize> = (0..self.plan.accounts.len())
            .filter(|&index| self.plan.accounts[index].drain_priority.is_some())
            .filter(unlocked)
            .collect();
        prioritized.sort_by_key(|&index| self.plan.accounts[index].drain_priority);
        for class in &self.plan.plan.withdrawal_order {
            for (index, account) in self.plan.accounts.iter().enumerate() {
                if account.drain_priority.is_none()
                    && account.treatment() == *class
                    && self.is_unlocked(index, year)
                {
                    prioritized.push(index);
                }
            }
        }
        prioritized
    }

    pub(super) fn is_unlocked(&self, index: usize, year: i16) -> bool {
        match &self.plan.accounts[index].locked_until {
            None => true,
            Some(trigger) => self
                .resolver
                .trigger_year(self.plan, trigger)
                .is_some_and(|unlock| year >= unlock),
        }
    }

    fn is_penalized(&self, index: usize, year: i16) -> bool {
        let account = &self.plan.accounts[index];
        if account.treatment() != TreatmentClass::Deferred || tax::is_penalty_exempt(account.kind) {
            return false;
        }
        self.plan
            .person(&account.owner)
            .is_none_or(|owner| !is_penalty_free_age(owner, year))
    }

    fn withdraw(&mut self, index: usize, want: Dollars, year: i16, acc: &mut YearAcc) -> Dollars {
        let take = want.min(self.balances[index]);
        if take <= 0 {
            return 0;
        }
        let account = &self.plan.accounts[index];
        let penalized = self.is_penalized(index, year);
        let untaxed = if account.keeps_basis() {
            self.remove_basis(index, take)
        } else {
            0
        };
        self.balances[index] -= take;
        acc.drained_cash += take;
        *acc.withdrawals.entry(account.id.clone()).or_default() += take;
        match acc
            .funding
            .iter_mut()
            .find(|(drained, _)| *drained == index)
        {
            Some((_, total)) => *total += take,
            None => acc.funding.push((index, take)),
        }
        match account.treatment() {
            TreatmentClass::Deferred => {
                acc.ordinary += take - untaxed;
                if penalized {
                    acc.penalty_base += take - untaxed;
                }
            }
            TreatmentClass::Taxable if account.kind.tracks_basis() => {
                acc.gains += take - untaxed;
            }
            TreatmentClass::Taxable | TreatmentClass::Roth | TreatmentClass::Hsa => {}
        }
        take
    }
}

/// The year's taxes. A traditional IRA contribution whose deduction the
/// MAGI decides is deducted here, by how far into the band the MAGI read
/// before it lies, and what was deducted is left on the accumulator.
fn compute_taxes(
    params: &TaxParams,
    state: Option<&StateParams>,
    status: FilingStatus,
    acc: &YearAcc,
) -> Taxes {
    let before = acc.ordinary + acc.gains;
    let ss_before = tax::taxable_social_security(params, status, before, acc.ss_gross);
    let pending: Dollars = acc.ira_to_settle.iter().map(|&(_, amount)| amount).sum();
    let band = params.limits.ira_deduction_phase_out.for_status(status);
    let ira_deducted = scale(pending, 1.0 - band.position(before + ss_before));
    let other_income = before - ira_deducted;
    let taxable_ss = tax::taxable_social_security(params, status, other_income, acc.ss_gross);
    let deduction = params.deductions.standard.get(status);
    let ordinary_taxable = acc.ordinary - ira_deducted + taxable_ss - deduction;
    let ordinary = tax::ordinary_tax(params, status, ordinary_taxable);
    let ltcg = tax::ltcg_tax(params, status, ordinary_taxable, acc.gains);
    let penalty = scale(acc.penalty_base, params.early_withdrawal.penalty);
    let state = state.map_or(0, |state| {
        tax::state_tax(state, status, other_income, taxable_ss)
    });
    Taxes {
        ordinary,
        ltcg,
        penalty,
        state,
        taxable_social_security: taxable_ss,
        ordinary_taxable: ordinary_taxable.max(0),
        magi: (other_income + taxable_ss).max(0),
        total: ordinary + ltcg + penalty + state,
        ira_deducted,
    }
}

fn is_penalty_free_age(person: &Person, year: i16) -> bool {
    let span = jiff::Span::new()
        .years(PENALTY_FREE_YEARS)
        .months(PENALTY_FREE_MONTHS);
    person
        .birth
        .0
        .checked_add(span)
        .is_ok_and(|date| date.year() <= year)
}
