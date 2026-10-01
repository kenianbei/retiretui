//! What moves money between accounts in a year - transfers, RMDs, Roth
//! conversions - and the basis each move carries with it.

use crate::params::TaxParams;
use crate::plan::{Account, Dollars, TreatmentClass};
use crate::tax;

use super::year::{Simulation, YearAcc};
use super::{Action, scale};

impl Simulation<'_> {
    pub(super) fn execute_transfers(&mut self, year: i16, factor: f64, acc: &mut YearAcc) {
        for (i, transfer) in self.plan.transfers.iter().enumerate() {
            if self.transfer_done[i] {
                continue;
            }
            let Some(due) = self.resolver.trigger_year(self.plan, &transfer.on) else {
                continue;
            };
            if year < due {
                continue;
            }
            let (Some(&from), Some(&to)) = (
                self.index_of.get(transfer.from.as_str()),
                self.index_of.get(transfer.to.as_str()),
            ) else {
                self.transfer_done[i] = true;
                continue;
            };
            // A lock defers the transfer rather than consuming it: it
            // retries each year and fires once the source unlocks.
            if !self.is_unlocked(from, year) {
                continue;
            }
            self.transfer_done[i] = true;
            let requested = transfer
                .amount
                .map_or(self.balances[from], |amount| scale(amount, factor));
            let take = requested.min(self.balances[from]);
            if take <= 0 {
                continue;
            }
            let distributed = self.move_between_accounts(from, to, take, year);
            acc.ordinary += distributed;
            if self.is_penalized(from, year) {
                acc.penalty_base += distributed;
            }
            acc.actions.push(Action::Transfer {
                from: transfer.from.clone(),
                to: transfer.to.clone(),
                amount: take,
            });
        }
    }

    /// Moves `take` and answers the part of it taxed as a distribution.
    /// What a qualified Roth account gives up arrives as money paid in.
    fn move_between_accounts(
        &mut self,
        from: usize,
        to: usize,
        take: Dollars,
        year: i16,
    ) -> Dollars {
        let from_account = &self.plan.accounts[from];
        let to_account = &self.plan.accounts[to];
        let is_paid_in = self.is_qualified_roth(from, year);
        let basis_out = if from_account.keeps_basis() {
            self.remove_basis(from, take)
        } else {
            take
        };
        self.balances[from] -= take;
        self.balances[to] += take;
        let is_distribution = from_account.treatment() == TreatmentClass::Deferred
            && to_account.treatment() == TreatmentClass::Taxable;
        if to_account.keeps_basis() {
            let is_whole = is_distribution || is_paid_in;
            self.bases[to] += if is_whole { take } else { basis_out };
        }
        self.open_roth(to, year);
        if is_distribution { take - basis_out } else { 0 }
    }

    /// The untaxed part of `take` from an account: pro rata over the
    /// account's basis and balance - or, for an IRA, over the owner's IRAs
    /// of its tax treatment together, as Form 8606 has it.
    pub(super) fn remove_basis(&mut self, index: usize, take: Dollars) -> Dollars {
        let (basis, balance) = self.basis_pool(index).fold((0, 0), |(basis, balance), i| {
            (basis + self.bases[i], balance + self.balances[i])
        });
        if basis <= 0 || balance <= 0 {
            return 0;
        }
        let removed = scale(take, basis as f64 / balance as f64)
            .min(take)
            .min(basis);
        self.take_basis(index, removed);
        removed
    }

    /// Takes `removed` out of the basis `index` pools, the account itself
    /// giving up its own first and then the others.
    pub(super) fn take_basis(&mut self, index: usize, removed: Dollars) {
        let mut left = removed;
        let others: Vec<usize> = self.basis_pool(index).filter(|&i| i != index).collect();
        for i in std::iter::once(index).chain(others) {
            let cut = left.min(self.bases[i]);
            self.bases[i] -= cut;
            left -= cut;
        }
    }

    /// The accounts whose basis is one pool with `index`'s: the owner's
    /// IRAs of its tax treatment together, any other account alone.
    pub(super) fn basis_pool(&self, index: usize) -> impl Iterator<Item = usize> + '_ {
        let account = &self.plan.accounts[index];
        let is_pooled = move |(i, other): (usize, &Account)| {
            let is_same_pool = other.kind.is_ira()
                && other.owner == account.owner
                && other.treatment() == account.treatment();
            (i == index || (account.kind.is_ira() && is_same_pool)).then_some(i)
        };
        self.plan.accounts.iter().enumerate().filter_map(is_pooled)
    }

    pub(super) fn take_rmds(
        &mut self,
        year: i16,
        params: &TaxParams,
        snapshot: &[Dollars],
        acc: &mut YearAcc,
    ) {
        for (i, account) in self.plan.accounts.iter().enumerate() {
            if account.treatment() != TreatmentClass::Deferred || snapshot[i] <= 0 {
                continue;
            }
            let Some(owner) = self.plan.person(&account.owner) else {
                continue;
            };
            let age = owner.age_in(year);
            if age < tax::rmd_start_age(owner.birth.year()) {
                continue;
            }
            let amount = tax::rmd(params, age, snapshot[i]).min(self.balances[i]);
            if amount <= 0 {
                continue;
            }
            let untaxed = self.remove_basis(i, amount);
            self.balances[i] -= amount;
            acc.rmds += amount;
            acc.ordinary += amount - untaxed;
            acc.cash += amount;
            *acc.withdrawals.entry(account.id.clone()).or_default() += amount;
            acc.actions.push(Action::Rmd {
                account: account.id.clone(),
                amount,
            });
        }
    }

    pub(super) fn execute_conversions(&mut self, year: i16, acc: &mut YearAcc) {
        for (i, conversion) in self.plan.conversions.iter().enumerate() {
            // A one-shot conversion defers like a transfer, firing once the
            // source unlocks; windowed schedules simply skip locked years.
            let active = match conversion.on.as_ref() {
                Some(on) => {
                    !self.conversion_done[i]
                        && self
                            .resolver
                            .trigger_year(self.plan, on)
                            .is_some_and(|due| year >= due)
                }
                None => self.item_active(year, conversion.span()),
            };
            if !active {
                continue;
            }
            let (Some(&from), Some(&to)) = (
                self.index_of.get(conversion.from.as_str()),
                self.index_of.get(conversion.to.as_str()),
            ) else {
                continue;
            };
            if !self.is_unlocked(from, year) {
                continue;
            }
            if conversion.on.is_some() {
                self.conversion_done[i] = true;
            }
            let take = scale(conversion.amount, self.cola_factor(conversion.cola, year))
                .min(self.balances[from]);
            if take <= 0 {
                continue;
            }
            let untaxed = self.remove_basis(from, take);
            self.balances[from] -= take;
            self.balances[to] += take;
            self.land_conversion(to, year, take - untaxed, untaxed);
            acc.ordinary += take - untaxed;
            acc.conversions += take;
            acc.actions.push(Action::Conversion {
                from: conversion.from.clone(),
                to: conversion.to.clone(),
                amount: take,
            });
        }
    }
}
