//! The payroll taxes a year's earned income pays: Social Security and
//! Medicare on wages, the self-employment tax on net earnings, and the
//! Additional Medicare Tax on the household's together.

use crate::params::TaxParams;
use crate::plan::{Dollars, FilingStatus};

/// What one person earned in a year that payroll taxes reach, after what
/// left it before tax - a cafeteria-plan deduction, a payroll HSA
/// contribution - which may leave a figure below nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Earnings {
    /// Wages from a job that pays into Social Security.
    pub covered: Dollars,
    /// Wages from every job, which Medicare taxes.
    pub medicare: Dollars,
}

/// The year's payroll tax of everyone in the household together.
#[must_use]
pub fn payroll(params: &TaxParams, status: FilingStatus, earnings: &[Earnings]) -> Dollars {
    let rates = &params.payroll;
    let wage_base = params
        .social_security
        .benefit
        .as_ref()
        .map_or(Dollars::MAX, |benefit| benefit.wage_base(params.year));
    let mut tax = 0.0;
    let mut medicare_wages = 0;
    for person in earnings {
        let covered = person.covered.clamp(0, wage_base);
        let medicare = person.medicare.max(0);
        tax += rates.oasdi * covered as f64 + rates.medicare * medicare as f64;
        medicare_wages += medicare;
    }
    let over = (medicare_wages - rates.additional_medicare_over.get(status)).max(0);
    tax += rates.additional_medicare * over as f64;
    tax.round() as Dollars
}
