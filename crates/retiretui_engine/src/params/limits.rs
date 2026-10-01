//! The contribution limits, the phase-out bands they carry, and the other
//! per-year tables a projection applies beside the tax brackets.

use serde::{Deserialize, Serialize};

use super::PerStatus;
use crate::plan::{Dollars, FilingStatus};

/// IRC §223(f)(4).
const STATUTORY_HSA_PENALTY: f64 = 0.20;

fn statutory_hsa_penalty() -> f64 {
    STATUTORY_HSA_PENALTY
}

/// Early-withdrawal parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct EarlyWithdrawal {
    /// Penalty rate on early distributions.
    pub penalty: f64,
    /// Penalty rate on what an HSA pays before 65 that is not medical
    /// spending; the statutory rate where a table leaves it out.
    #[serde(default = "statutory_hsa_penalty")]
    pub hsa_penalty: f64,
}

/// One row of the RMD Uniform Lifetime Table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RmdDivisor {
    /// Age reached during the distribution year.
    pub age: u8,
    /// The divisor applied to the prior year-end balance.
    pub divisor: f64,
}

/// The RMD table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RmdTable {
    /// Rows in ascending age order.
    pub divisors: Vec<RmdDivisor>,
}

/// A MAGI band over which something phases out: whole below `from`, gone
/// at `to`, linear between.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PhaseOut {
    /// MAGI at or below which nothing is lost.
    pub from: Dollars,
    /// MAGI at or above which all is lost.
    pub to: Dollars,
}

impl ContributionLimits {
    /// The band a person no workplace plan covers phases out over while one
    /// covers their spouse: on a joint return, where the table states it.
    #[must_use]
    pub fn spouse_ira_deduction_phase_out(&self, status: FilingStatus) -> Option<PhaseOut> {
        self.ira_deduction_phase_out_spouse
            .filter(|_| status == FilingStatus::MarriedJoint)
    }
}

impl PhaseOut {
    /// How far into the band `magi` is, from 0 at its foot to 1 at its top.
    #[must_use]
    pub fn position(self, magi: Dollars) -> f64 {
        if magi <= self.from {
            return 0.0;
        }
        if magi >= self.to || self.to <= self.from {
            return 1.0;
        }
        (magi - self.from) as f64 / (self.to - self.from) as f64
    }
}

/// Annual contribution limits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContributionLimits {
    /// Elective deferral for 401(k)/403(b)/457(b).
    pub employer_plan: Dollars,
    /// Additional deferral from age 50.
    pub employer_plan_catch_up_50: Dollars,
    /// Replacement catch-up at ages 60-63.
    pub employer_plan_catch_up_60: Dollars,
    /// Everything paid into one defined-contribution plan in a year, by
    /// employee and employer together (415(c)).
    pub overall_plan: Dollars,
    /// SIMPLE IRA deferral.
    pub simple: Dollars,
    /// SIMPLE additional deferral from age 50.
    pub simple_catch_up_50: Dollars,
    /// SIMPLE replacement catch-up at ages 60-63.
    pub simple_catch_up_60: Dollars,
    /// IRA contribution (traditional plus Roth combined).
    pub ira: Dollars,
    /// IRA additional contribution from age 50.
    pub ira_catch_up_50: Dollars,
    /// HSA limit with self-only coverage.
    pub hsa_self: Dollars,
    /// HSA limit with family coverage.
    pub hsa_family: Dollars,
    /// HSA additional contribution from age 55.
    pub hsa_catch_up_55: Dollars,
    /// The MAGI band over which a Roth IRA contribution is no longer
    /// allowed.
    pub roth_ira_phase_out: PerStatus<PhaseOut>,
    /// The MAGI band over which a traditional IRA contribution stops being
    /// deductible for a person a workplace plan covers.
    pub ira_deduction_phase_out: PerStatus<PhaseOut>,
    /// The MAGI band over which a traditional IRA contribution stops being
    /// deductible, on a joint return, for a person no workplace plan covers
    /// whose spouse one does; where a table leaves it out, deducted in full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ira_deduction_phase_out_spouse: Option<PhaseOut>,
}

/// One IRMAA tier: the MAGI threshold it starts above and the annual
/// surcharges per covered person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct IrmaaTier {
    /// The tier applies when the lookback MAGI exceeds this.
    pub magi_over: PerStatus<Dollars>,
    /// Annual Part B surcharge per covered person.
    pub part_b: Dollars,
    /// Annual Part D surcharge per covered person.
    pub part_d: Dollars,
}
