//! The year's contributions: what each item asks for, held to the room its
//! owner has left under the law's limits, summed into the account it pays
//! into and recorded as one action per account.

use std::collections::{BTreeMap, BTreeSet};

use crate::params::TaxParams;
use crate::plan::{Account, AccountKind, Contribution, Dollars, Payer, TreatmentClass};
use crate::tax::{self, LimitPool};

use super::year::{Simulation, YearAcc};
use super::{Action, ContributionNote, Taxes, scale};

/// Whether everything paid into the account in a year, by whoever pays,
/// is capped as one defined-contribution plan's is.
fn is_capped_plan(kind: AccountKind) -> bool {
    matches!(
        kind,
        AccountKind::K401k
            | AccountKind::K403b
            | AccountKind::SimpleIra
            | AccountKind::K414k
            | AccountKind::SepIra
    )
}

/// What one account is paid in a year, by who pays, and why.
#[derive(Clone, Default)]
struct Paid {
    employee: Dollars,
    employer: Dollars,
    after_tax: Dollars,
    notes: Vec<ContributionNote>,
}

impl Paid {
    fn note(&mut self, note: ContributionNote) {
        push_unique(&mut self.notes, note);
    }
}

fn push_unique(notes: &mut Vec<ContributionNote>, note: ContributionNote) {
    if !notes.contains(&note) {
        notes.push(note);
    }
}

/// The room each person has left this year under each pooled limit.
struct Room<'a> {
    year: i16,
    params: &'a TaxParams,
    left: BTreeMap<(&'a str, LimitPool), Dollars>,
}

impl<'a> Room<'a> {
    /// An employee amount, or anything into an HSA, takes what is left of
    /// its owner's pooled limit for the year; items are filled in the order
    /// the plan lists them.
    fn take(
        &mut self,
        simulation: &Simulation<'a>,
        account: &'a Account,
        by: Payer,
        asked: Dollars,
    ) -> Dollars {
        let is_pooled = by == Payer::Employee || account.kind == AccountKind::Hsa;
        let Some(pool) = tax::limit_pool(account.kind).filter(|_| is_pooled) else {
            return asked;
        };
        let left = self
            .left
            .entry((account.owner.as_str(), pool))
            .or_insert_with(|| simulation.employee_limit(self.params, account, self.year));
        let given = asked.min(*left);
        *left -= given;
        given
    }
}

impl<'a> Simulation<'a> {
    pub(super) fn collect_contributions(
        &mut self,
        year: i16,
        params: &TaxParams,
        acc: &mut YearAcc,
    ) {
        let plan = self.plan;
        let mut paid = vec![Paid::default(); plan.accounts.len()];
        let mut room = Room {
            year,
            params,
            left: BTreeMap::new(),
        };
        let active = self.active_contributions(year);
        let mut covered = BTreeSet::new();
        for by in Payer::ALL {
            for &(index, contribution) in active.iter().filter(|(_, held)| held.by == *by) {
                let account = &plan.accounts[index];
                let (asked, note) = self.asked((index, contribution), &room, &paid[index], acc);
                let capped = asked.min(self.overall_room(index, params, &paid[index]));
                let given = room.take(self, account, *by, capped);
                if given > 0 && is_capped_plan(account.kind) {
                    covered.insert(account.owner.as_str());
                }
                let to = &mut paid[index];
                match by {
                    Payer::Employee => to.employee += given,
                    Payer::Employer => to.employer += given,
                    Payer::AfterTax => to.after_tax += given,
                }
                to.notes.extend(note);
                if capped < asked {
                    to.note(ContributionNote::HeldToOverall);
                }
                if given < capped {
                    to.note(ContributionNote::HeldToLimit);
                }
            }
        }
        for (index, paid) in paid.into_iter().enumerate() {
            if paid.employee > 0 || paid.employer > 0 || paid.after_tax > 0 {
                self.deposit(index, paid, &covered, acc);
            }
        }
    }

    /// The items paying this year, with the index of the account each pays
    /// into, in the order the plan lists them.
    fn active_contributions(&self, year: i16) -> Vec<(usize, &'a Contribution)> {
        let is_active = |contribution: &&Contribution| self.item_active(year, contribution.span());
        let placed = |contribution: &'a Contribution| {
            let index = *self.index_of.get(contribution.to.as_str())?;
            Some((index, contribution))
        };
        let active = self.plan.contributions.iter().filter(is_active);
        active.filter_map(placed).collect()
    }

    /// What the item asks for this year, and the note that says how where
    /// a figure alone would not. A match reads what the employee has been
    /// paid into the account so far, so employees are asked first.
    fn asked(
        &self,
        (index, contribution): (usize, &Contribution),
        room: &Room,
        paid: &Paid,
        acc: &YearAcc,
    ) -> (Dollars, Option<ContributionNote>) {
        let (year, params) = (room.year, room.params);
        if let Some(amount) = contribution.amount {
            let factor = self.cola_factor(contribution.cola, year);
            return (scale(amount, factor), None);
        }
        if contribution.max {
            let limit = self.employee_limit(params, &self.plan.accounts[index], year);
            return (limit, Some(ContributionNote::Maximum));
        }
        let Some(of) = &contribution.of else {
            return (0, None);
        };
        let gross = acc.income.get(of).copied().unwrap_or(0);
        if let Some(matching) = contribution.matching {
            let matched = paid.employee.min(scale(gross, matching.up_to));
            let note = ContributionNote::Match {
                rate: matching.rate,
                up_to: matching.up_to,
                of: of.clone(),
            };
            return (scale(matched, matching.rate), Some(note));
        }
        let first = contribution
            .start
            .as_ref()
            .and_then(|start| self.resolver.trigger_year(self.plan, start))
            .unwrap_or(self.start_year);
        let years = i32::from(year - first).max(0);
        let Some(rate) = contribution.rate_after(years) else {
            return (0, None);
        };
        let note = ContributionNote::Share {
            rate,
            of: of.clone(),
        };
        (scale(gross, rate), Some(note))
    }

    /// What the account's yearly cap still has room for after what it has
    /// been paid so far; every payer is held to it, in the order they are
    /// asked, so the employer gives way to the employee and after-tax money
    /// takes what is left.
    fn overall_room(&self, index: usize, params: &TaxParams, paid: &Paid) -> Dollars {
        if !is_capped_plan(self.plan.accounts[index].kind) {
            return Dollars::MAX;
        }
        (params.limits.overall_plan - paid.employee - paid.employer - paid.after_tax).max(0)
    }

    fn employee_limit(&self, params: &TaxParams, account: &Account, year: i16) -> Dollars {
        let age = self
            .plan
            .person(&account.owner)
            .map_or(0, |owner| owner.age_in_year(year).clamp(0, 255) as u8);
        tax::employee_limit(params, self.plan.household.filing, account.kind, age)
            .unwrap_or(Dollars::MAX)
    }

    /// Lands the year's payments in the account: pending until growth,
    /// the employee's pre-tax part deducted where the kind defers tax, and
    /// what was never taxed or already was kept as basis. A traditional
    /// IRA contribution by someone a workplace plan covers waits for the
    /// year's MAGI to say how much of it is deducted.
    fn deposit(
        &mut self,
        index: usize,
        mut paid: Paid,
        covered: &BTreeSet<&str>,
        acc: &mut YearAcc,
    ) {
        let account = &self.plan.accounts[index];
        let total = paid.employee + paid.employer + paid.after_tax;
        if paid.after_tax > 0 {
            paid.notes.push(ContributionNote::AfterTax {
                amount: paid.after_tax,
            });
        }
        acc.employee += paid.employee + paid.after_tax;
        acc.employer += paid.employer;
        acc.pending[index] += total;
        acc.actions.push(Action::Contribution {
            account: account.id.clone(),
            employee: paid.employee + paid.after_tax,
            employer: paid.employer,
            notes: paid.notes,
        });
        match account.treatment() {
            TreatmentClass::Deferred => {
                let is_decided_by_magi =
                    account.kind == AccountKind::Ira && covered.contains(account.owner.as_str());
                if is_decided_by_magi {
                    acc.ira_to_settle.push((index, paid.employee));
                } else {
                    acc.ordinary -= paid.employee;
                }
                self.bases[index] += paid.after_tax;
            }
            TreatmentClass::Hsa => acc.ordinary -= paid.employee,
            TreatmentClass::Taxable if account.kind.tracks_basis() => self.bases[index] += total,
            TreatmentClass::Taxable | TreatmentClass::Roth => {}
        }
    }
}

impl Simulation<'_> {
    /// Once the year's MAGI is settled: what a traditional IRA contribution
    /// could not deduct becomes basis and is said, and a Roth IRA
    /// contribution over the income band is said.
    pub(super) fn settle_ira_bands(
        &mut self,
        params: &TaxParams,
        taxes: &Taxes,
        acc: &mut YearAcc,
    ) {
        let pending: Dollars = acc.ira_to_settle.iter().map(|&(_, amount)| amount).sum();
        let withheld = pending - taxes.ira_deducted;
        if withheld > 0 {
            for &(index, amount) in &acc.ira_to_settle {
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
