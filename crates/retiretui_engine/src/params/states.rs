//! A state's income tax table, as a year's parameters hold it.

use serde::{Deserialize, Serialize};

use super::{Bracket, PerStatus};
use crate::plan::Dollars;

/// One state's income tax. A state with no income tax is an empty table,
/// which is not the same as a state with none: that one is not modeled.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct StateParams {
    /// Standard deduction by filing status.
    pub deduction: PerStatus<Dollars>,
    /// Brackets by filing status, walked as the federal ones are.
    pub brackets: PerStatus<Vec<Bracket>>,
    /// Whether the federally taxable share of Social Security is taxed.
    pub taxes_social_security: bool,
    /// What the state leaves untaxed, a person at a time.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclusions: Vec<Exclusion>,
    /// Whether what a person pays into a tax-deferred account is taxed in
    /// the year it is paid, a deducted traditional IRA contribution with it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub taxes_deferrals: bool,
    /// Set where the law fixes the deduction in nominal dollars, so that it
    /// is not inflated past the last known year.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub deduction_unindexed: bool,
}

/// Sources of income a state leaves untaxed, whole, for each person old
/// enough.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Exclusion {
    /// The sources left untaxed.
    pub sources: Vec<Source>,
    /// The age, in years, from which they are: throughout the calendar year
    /// it is reached, and after. None leaves them untaxed at any age.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_age: Option<f64>,
}

/// Where a dollar of taxable ordinary income came from, as a state's table
/// names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// Salary, less what the employee paid into an HSA before tax.
    Wages,
    /// Pension income.
    Pension,
    /// What a retirement account pays out that the federal early-withdrawal
    /// penalty does not reach.
    Distribution,
    /// What a retirement account pays out that it does.
    EarlyDistribution,
    /// The taxed part of a Roth conversion.
    Conversion,
    /// Rental, annuity and other income, and what an HSA pays beyond
    /// medical spending.
    Other,
}

impl Source {
    /// Every source, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[
        Self::Wages,
        Self::Pension,
        Self::Distribution,
        Self::EarlyDistribution,
        Self::Conversion,
        Self::Other,
    ];

    /// The source as a parameter file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wages => "wages",
            Self::Pension => "pension",
            Self::Distribution => "distribution",
            Self::EarlyDistribution => "early-distribution",
            Self::Conversion => "conversion",
            Self::Other => "other",
        }
    }
}
