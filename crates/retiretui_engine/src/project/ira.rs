//! The traditional IRA deduction a workplace plan decides: the band each
//! contribution phases out over, what the year's MAGI deducts of it, and
//! what it could not.

use std::collections::BTreeSet;

use crate::params::{ContributionLimits, PhaseOut, TaxParams};
use crate::plan::{Account, AccountKind, Dollars, FilingStatus};

use super::contribute::push_unique;
use super::year::{Simulation, YearAcc};
use super::{Action, ContributionNote, Taxes, scale};

/// The phase-out band a pending traditional IRA contribution is settled
/// against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IraBand {
    /// A workplace plan covers the owner.
    Covered,
    /// None covers the owner, and one covers their spouse on a joint return.
    SpouseCovered,
}

impl IraBand {
    const ALL: [Self; 2] = [Self::Covered, Self::SpouseCovered];

    /// The MAGI band this contribution phases out over; none where there is
    /// no spouse's band, and it is deducted in full.
    fn phase_out(self, limits: &ContributionLimits, status: FilingStatus) -> Option<PhaseOut> {
        match self {
            Self::Covered => Some(*limits.ira_deduction_phase_out.for_status(status)),
            Self::SpouseCovered => limits.spouse_ira_deduction_phase_out(status),
        }
    }

    /// What of the year's pending contributions is settled against this band.
    fn pending(self, pending: &[(usize, IraBand, Dollars)]) -> Dollars {
        pending
            .iter()
            .filter(|&&(_, band, _)| band == self)
            .map(|&(_, _, amount)| amount)
            .sum()
    }
}

/// What each band's pending contributions deduct at `magi`, in
/// [`IraBand::ALL`]'s order.
pub(super) fn ira_deducted(
    params: &TaxParams,
    status: FilingStatus,
    pending: &[(usize, IraBand, Dollars)],
    magi: Dollars,
) -> [Dollars; 2] {
    IraBand::ALL.map(|band| {
        let withheld = band
            .phase_out(&params.limits, status)
            .map_or(0.0, |phase_out| phase_out.position(magi));
        scale(band.pending(pending), 1.0 - withheld)
    })
}

impl Simulation<'_> {
    /// The band a traditional IRA contribution into `account` phases out
    /// over, if any: its owner's while a workplace plan covers them, their
    /// spouse's while one covers the spouse. Any other account's is
    /// deducted in full.
    pub(super) fn ira_band(account: &Account, covered: &BTreeSet<&str>) -> Option<IraBand> {
        if account.kind != AccountKind::Ira {
            return None;
        }
        if covered.contains(account.owner.as_str()) {
            return Some(IraBand::Covered);
        }
        let is_spouse_covered = covered.iter().any(|&person| person != account.owner);
        is_spouse_covered.then_some(IraBand::SpouseCovered)
    }

    /// Once the year's MAGI is settled: what a traditional IRA contribution
    /// could not deduct becomes basis and is said, and a Roth IRA
    /// contribution over the income band is said.
    pub(super) fn settle_ira_bands(
        &mut self,
        params: &TaxParams,
        taxes: &Taxes,
        acc: &mut YearAcc,
    ) {
        for (band, deducted) in IraBand::ALL.into_iter().zip(taxes.ira_deducted) {
            self.keep_not_deducted(band, deducted, acc);
        }
        let band = params
            .limits
            .roth_ira_phase_out
            .for_status(self.plan.household.filing);
        if band.position(taxes.magi) <= 0.0 {
            return;
        }
        for account in &self.plan.accounts {
            if account.kind == AccountKind::Ira && account.roth {
                note_on(
                    &mut acc.actions,
                    &account.id,
                    ContributionNote::RothIraPhaseOut,
                );
            }
        }
    }

    /// Keeps what `band`'s contributions could not deduct as basis, shared
    /// over them pro rata, and says so on each.
    fn keep_not_deducted(&mut self, band: IraBand, deducted: Dollars, acc: &mut YearAcc) {
        let pending = band.pending(&acc.ira_to_settle);
        let withheld = pending - deducted;
        if withheld <= 0 {
            return;
        }
        let entries = acc.ira_to_settle.iter().filter(|&&(_, of, _)| of == band);
        for &(index, _, amount) in entries {
            let part = scale(amount, withheld as f64 / pending as f64);
            self.bases[index] += part;
            let id = &self.plan.accounts[index].id;
            note_on(
                &mut acc.actions,
                id,
                ContributionNote::NotDeducted { amount: part },
            );
        }
    }
}

/// Adds a note to the year's contribution into `id`, where there is one
/// with an employee amount.
fn note_on(actions: &mut [Action], id: &str, note: ContributionNote) {
    let paid = actions.iter_mut().find_map(|action| match action {
        Action::Contribution {
            account,
            employee,
            notes,
            ..
        } if account == id && *employee > 0 => Some(notes),
        _ => None,
    });
    if let Some(notes) = paid {
        push_unique(notes, note);
    }
}
