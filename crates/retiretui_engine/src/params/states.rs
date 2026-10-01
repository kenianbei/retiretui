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
    /// What the employee paid into a workplace plan before tax; never above
    /// nothing.
    Deferral,
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
        Self::Deferral,
    ];

    /// How many sources there are.
    pub const COUNT: usize = Self::ALL.len();

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
            Self::Deferral => "deferral",
        }
    }
}
