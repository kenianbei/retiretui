use crate::params::{StateParams, TaxParams};
use crate::plan::{Account, Dollars, FilingStatus, Person, TreatmentClass};
use crate::tax;

use super::Taxes;
use super::ira::ira_deducted;
use super::residence;
use super::scale;
use super::year::{Simulation, YearAcc};

const MAX_TAX_ITERATIONS: usize = 30;
const PENALTY_FREE_YEARS: i64 = 59;
const PENALTY_FREE_MONTHS: i64 = 6;
/// IRC §72(t)(2)(A)(v).
const SEPARATION_AGE: i16 = 55;
/// IRC §72(t)(10).
const PUBLIC_SAFETY_SEPARATION_AGE: i16 = 50;

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

    /// Takes `want` from the candidates in two passes: from each, in order,
    /// what leaves it without penalty, then the rest, which only those a
    /// penalty held back still have.
    fn drain(&mut self, order: &[usize], year: i16, want: Dollars, acc: &mut YearAcc) -> Dollars {
        let mut remaining = want;
        for is_free_pass in [true, false] {
            for &index in order {
                if remaining <= 0 {
                    break;
                }
                if self.balances[index] <= 0 {
                    continue;
                }
                let limit = if is_free_pass {
                    remaining.min(self.penalty_free_room(index, year, acc))
                } else {
                    remaining
                };
                remaining -= self.withdraw(index, limit, year, acc);
            }
        }
        want - remaining
    }

    /// What can leave the account in `year` without paying a penalty:
    /// without limit where none can reach it.
    fn penalty_free_room(&self, index: usize, year: i16, acc: &YearAcc) -> Dollars {
        let account = &self.plan.accounts[index];
        match account.treatment() {
            TreatmentClass::Roth => self.roth_room(index, year),
            TreatmentClass::Hsa if self.is_under_medicare_age(account, year) => acc.medical,
            TreatmentClass::Deferred if self.pays_penalty(account, year) => 0,
            TreatmentClass::Deferred | TreatmentClass::Taxable | TreatmentClass::Hsa => {
                Dollars::MAX
            }
        }
    }

    fn is_under_medicare_age(&self, account: &Account, year: i16) -> bool {
        self.plan
            .person(&account.owner)
            .is_none_or(|owner| !tax::is_medicare_covered(owner, year))
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

    /// Whether the penalty reaches the account in `year`: its owner under
    /// 59½, its kind not exempt, its job not left at the age that frees it.
    pub(super) fn pays_penalty(&self, account: &Account, year: i16) -> bool {
        if tax::is_penalty_exempt(account.kind) {
            return false;
        }
        self.plan.person(&account.owner).is_none_or(|owner| {
            !is_penalty_free_age(owner, year) && !self.is_freed_by_separation(account, owner, year)
        })
    }

    pub(super) fn is_under_penalty_age(&self, account: &Account, year: i16) -> bool {
        self.plan
            .person(&account.owner)
            .is_none_or(|owner| !is_penalty_free_age(owner, year))
    }

    /// Whether the owner has left the job the plan is with by `year`, in or
    /// after the year they reached the age that frees it.
    fn is_freed_by_separation(&self, account: &Account, owner: &Person, year: i16) -> bool {
        let Some(trigger) = &account.separated else {
            return false;
        };
        let freeing_age = if account.public_safety {
            PUBLIC_SAFETY_SEPARATION_AGE
        } else {
            SEPARATION_AGE
        };
        self.resolver
            .trigger_year(self.plan, trigger)
            .is_some_and(|left| left <= year && owner.age_in_year(left) >= freeing_age)
    }

    fn withdraw(&mut self, index: usize, want: Dollars, year: i16, acc: &mut YearAcc) -> Dollars {
        let take = want.min(self.balances[index]);
        if take <= 0 {
            return 0;
        }
        let account = &self.plan.accounts[index];
        match account.treatment() {
            TreatmentClass::Deferred => {
                let taxed = take - self.remove_basis(index, take);
                acc.ordinary += taxed;
                if self.pays_penalty(account, year) {
                    acc.penalty_base += taxed;
                }
            }
            TreatmentClass::Taxable if account.kind.tracks_basis() => {
                acc.gains += take - self.remove_basis(index, take);
            }
            TreatmentClass::Roth => {
                let draw = self.draw_roth(index, take, year);
                acc.ordinary += draw.taxed;
                acc.penalty_base += draw.penalized;
            }
            TreatmentClass::Hsa => {
                let medical = take.min(acc.medical);
                acc.medical -= medical;
                acc.ordinary += take - medical;
                if self.is_under_medicare_age(account, year) {
                    acc.hsa_penalty_base += take - medical;
                }
            }
            TreatmentClass::Taxable => {}
        }
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
    let by_band = ira_deducted(params, status, &acc.ira_to_settle, before + ss_before);
    let deducted: Dollars = by_band.iter().sum();
    let other_income = before - deducted;
    let taxable_ss = tax::taxable_social_security(params, status, other_income, acc.ss_gross);
    let deduction = params.deductions.standard.get(status);
    let ordinary_taxable = acc.ordinary - deducted + taxable_ss - deduction;
    let ordinary = tax::ordinary_tax(params, status, ordinary_taxable);
    let ltcg = tax::ltcg_tax(params, status, ordinary_taxable, acc.gains);
    let early = params.early_withdrawal;
    let penalty =
        scale(acc.penalty_base, early.penalty) + scale(acc.hsa_penalty_base, early.hsa_penalty);
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
        gains: acc.gains,
        magi: (other_income + taxable_ss).max(0),
        total: ordinary + ltcg + penalty + state,
        ira_deducted: by_band,
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
