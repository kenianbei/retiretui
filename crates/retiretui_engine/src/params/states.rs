//! A state's income tax table, as a year's parameters hold it.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{Bracket, PerStatus, PhaseOut};
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
    /// What each person adds to the deduction from the year they reach 65,
    /// in nominal dollars.
    #[serde(skip_serializing_if = "crate::plan::is_default")]
    pub deduction_at_65: PerStatus<Dollars>,
    /// The federal AGI above which there is no deduction, in nominal
    /// dollars.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deduction_until_agi: Option<PerStatus<Dollars>>,
    /// The federal income tax the state lets be subtracted from income.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub federal_tax_subtraction: Option<FederalTaxSubtraction>,
    /// The credit against its tax the state gives for each person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exemption_credit: Option<ExemptionCredit>,
    /// The tax the state levies on long-term gains apart from income.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gains_excise: Option<GainsExcise>,
}

/// Federal income tax subtracted from a state's income, up to a cap the
/// federal AGI steps down.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct FederalTaxSubtraction {
    /// The most that is subtracted.
    pub cap: Dollars,
    /// How many equal parts the cap is lost in.
    pub steps: NonZeroU32,
    /// The federal AGI the cap is lost across, in nominal dollars: a part at
    /// `from` itself, and a part more at each even step up to `to`, where
    /// the last goes.
    pub phase_out: PerStatus<PhaseOut>,
}

/// A credit against a state's tax for each person, never refunded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ExemptionCredit {
    /// The credit for each person.
    pub per_person: Dollars,
    /// What each person adds to it from the year they reach 65, in nominal
    /// dollars.
    #[serde(default, skip_serializing_if = "crate::plan::is_default")]
    pub at_65: Dollars,
    /// The federal AGI above which there is no credit, in nominal dollars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until_agi: Option<PerStatus<Dollars>>,
    /// Set where the law fixes `per-person` in nominal dollars, so that it
    /// is not inflated past the last known year.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unindexed: bool,
}

/// A tax on the household's long-term gains, whatever its filing status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GainsExcise {
    /// The gains left untaxed.
    pub deduction: Dollars,
    /// Brackets over the gains beyond the deduction.
    pub brackets: Vec<Bracket>,
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
