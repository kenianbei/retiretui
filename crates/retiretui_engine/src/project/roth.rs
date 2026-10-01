//! What a Roth account gives up when it is drawn before the draw is
//! qualified: what was paid in, then conversions, then earnings.

use crate::plan::{Account, Dollars, TreatmentClass};

use super::year::Simulation;

/// IRC §408A(d): the tax years a Roth is held before its earnings come out
/// untaxed, and a conversion before it comes out unpenalized.
const SEASONING_YEARS: i16 = 5;

/// The parts of a Roth draw that are taxed and that pay the penalty.
#[derive(Default)]
pub(super) struct RothDraw {
    pub(super) taxed: Dollars,
    pub(super) penalized: Dollars,
}

impl<'a> Simulation<'a> {
    /// Starts the five years of a Roth account that has held nothing.
    pub(super) fn open_roth(&mut self, index: usize, year: i16) {
        if self.plan.accounts[index].treatment() == TreatmentClass::Roth {
            self.seasoned_from[index].get_or_insert(year + SEASONING_YEARS);
        }
    }

    /// Whether everything a Roth account gives up in `year` is untaxed:
    /// its owner 59½ and the account held five years, an IRA since its
    /// owner's first Roth IRA, a workplace plan since its own first money.
    /// An account that is stays so.
    pub(super) fn is_qualified_roth(&self, index: usize, year: i16) -> bool {
        let account = &self.plan.accounts[index];
        account.treatment() == TreatmentClass::Roth
            && !self.is_under_penalty_age(account, year)
            && self
                .basis_pool(index)
                .filter_map(|held| self.seasoned_from[held])
                .min()
                .is_some_and(|from| from <= year)
    }

    /// Lands a conversion in a Roth account. What was already taxed is
    /// basis. What is taxed now is basis in a workplace plan, and in an IRA
    /// a layer its owner's Roth IRAs give up oldest first.
    pub(super) fn land_conversion(
        &mut self,
        to: usize,
        year: i16,
        taxed: Dollars,
        untaxed: Dollars,
    ) {
        let account: &'a Account = &self.plan.accounts[to];
        self.open_roth(to, year);
        self.bases[to] += untaxed;
        if !account.kind.is_ira() {
            self.bases[to] += taxed;
        } else if taxed > 0 {
            self.layers
                .entry(account.owner.as_str())
                .or_default()
                .push((year + SEASONING_YEARS, taxed));
        }
    }

    /// What can leave a Roth account in `year` without paying the penalty.
    pub(super) fn roth_room(&self, index: usize, year: i16) -> Dollars {
        let account = &self.plan.accounts[index];
        if !self.pays_penalty(account, year) {
            return Dollars::MAX;
        }
        if !account.kind.is_ira() {
            return 0;
        }
        let seasoned: Dollars = self
            .layers
            .get(account.owner.as_str())
            .into_iter()
            .flatten()
            .filter(|(seasoned_from, _)| *seasoned_from <= year)
            .map(|(_, left)| left)
            .sum();
        self.pooled_basis(index) + seasoned
    }

    /// Takes the untaxed part of `take` out of a Roth account's records and
    /// answers the rest. A workplace plan gives up what was paid in pro
    /// rata; an owner's Roth IRAs, as one, give it up first, then
    /// conversions oldest first, then earnings. A qualified account's
    /// records are left as they are, and read no more.
    pub(super) fn draw_roth(&mut self, index: usize, take: Dollars, year: i16) -> RothDraw {
        if self.is_qualified_roth(index, year) {
            return RothDraw::default();
        }
        let account: &'a Account = &self.plan.accounts[index];
        let is_penalized = self.pays_penalty(account, year);
        if !account.kind.is_ira() {
            let earnings = take - self.remove_basis(index, take);
            return RothDraw {
                taxed: earnings,
                penalized: if is_penalized { earnings } else { 0 },
            };
        }
        let paid_in = take.min(self.pooled_basis(index));
        self.take_basis(index, paid_in);
        let mut earnings = take - paid_in;
        let mut penalized = 0;
        let layers = self.layers.get_mut(account.owner.as_str());
        for (seasoned_from, left) in layers.into_iter().flatten() {
            let converted = earnings.min(*left);
            *left -= converted;
            earnings -= converted;
            if is_penalized && year < *seasoned_from {
                penalized += converted;
            }
        }
        if is_penalized {
            penalized += earnings;
        }
        RothDraw {
            taxed: earnings,
            penalized,
        }
    }

    fn pooled_basis(&self, index: usize) -> Dollars {
        self.basis_pool(index).map(|held| self.bases[held]).sum()
    }
}
