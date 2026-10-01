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
