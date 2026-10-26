//! The payroll taxes a year's earned income pays: Social Security and
//! Medicare on wages, the self-employment tax on net earnings, and the
//! Additional Medicare Tax on the household's together.

use std::ops::AddAssign;

use crate::params::TaxParams;
use crate::plan::{Dollars, FilingStatus};

/// What one person earned in a year that payroll taxes reach, after what
/// left it before tax: a cafeteria-plan deduction, a payroll HSA
/// contribution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Earnings {
    /// Wages from a job that pays into Social Security.
    pub covered: Dollars,
    /// Wages from every job, which Medicare taxes.
    pub medicare: Dollars,
    /// The taxed share of net earnings from self-employment.
    pub self_employed: Dollars,
}

impl Earnings {
    /// Takes `amount` out of the wages before tax, as far as they go.
    pub fn deduct(&mut self, amount: Dollars) {
        self.covered = (self.covered - amount).max(0);
        self.medicare = (self.medicare - amount).max(0);
    }
}

impl AddAssign for Earnings {
    fn add_assign(&mut self, other: Self) {
        self.covered += other.covered;
        self.medicare += other.medicare;
        self.self_employed += other.self_employed;
    }
}

/// A year's payroll tax, and what of it comes off ordinary income.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Payroll {
    /// Everyone's together, the Additional Medicare Tax included.
    pub total: Dollars,
    /// Half of the self-employment tax, deducted from ordinary income.
    pub deduction: Dollars,
}

/// The year's payroll tax of everyone in the household together: the
/// employee's shares on wages, twice them on self-employment earnings up
/// to what the wage base leaves over the wages, and the Additional
/// Medicare Tax on both over the threshold.
#[must_use]
pub fn payroll(params: &TaxParams, status: FilingStatus, earnings: &[Earnings]) -> Payroll {
    let rates = &params.payroll;
    let wage_base = params
        .social_security
        .benefit
        .as_ref()
        .map_or(Dollars::MAX, |benefit| benefit.wage_base(params.year));
    let mut employee = 0.0;
    let mut self_employment = 0.0;
    let mut medicare_base = 0;
    for person in earnings {
        let covered = person.covered.min(wage_base);
        let net = person.self_employed;
        let net_covered = net.min(wage_base - covered);
        employee += rates.oasdi * covered as f64 + rates.medicare * person.medicare as f64;
        self_employment += 2.0 * (rates.oasdi * net_covered as f64 + rates.medicare * net as f64);
        medicare_base += person.medicare + net;
    }
    let over = (medicare_base - rates.additional_medicare_over.get(status)).max(0);
    let additional = rates.additional_medicare * over as f64;
    Payroll {
        total: (employee + self_employment + additional).round() as Dollars,
        deduction: (self_employment / 2.0).round() as Dollars,
    }
}
