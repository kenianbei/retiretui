use serde::{Deserialize, Serialize};

use super::Dollars;
use super::escalation::ColaSpec;
use super::triggers::Trigger;

/// What kind of income a source is; the kind decides its tax treatment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IncomeKind {
    /// Wages; ordinary income, source of contributions.
    Salary,
    /// Defined-benefit pension; ordinary income.
    Pension,
    /// Annuity payout; treated as fully ordinary income.
    Annuity,
    /// Net rental income; ordinary income.
    Rental,
    /// Social Security benefit; taxed by the provisional-income rules.
    SocialSecurity,
    /// One-time untaxed receipt (inheritance, gift, sale proceeds).
    Windfall,
    /// Anything else; treated as ordinary income.
    Other,
}

impl IncomeKind {
    /// Every kind, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[
        Self::Salary,
        Self::Pension,
        Self::Annuity,
        Self::Rental,
        Self::SocialSecurity,
        Self::Windfall,
        Self::Other,
    ];

    /// The kind as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Salary => "salary",
            Self::Pension => "pension",
            Self::Annuity => "annuity",
            Self::Rental => "rental",
            Self::SocialSecurity => "social-security",
            Self::Windfall => "windfall",
            Self::Other => "other",
        }
    }
}

/// An income source, in annual today's dollars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Income {
    /// Unique id, which scenarios and references match the income by; a
    /// plan without one is refused by validation.
    #[serde(default)]
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The kind, which decides tax treatment.
    pub kind: IncomeKind,
    /// Owning person id.
    pub owner: String,
    /// Annual amount in today's dollars (for `windfall`, the one-time
    /// amount). A `social-security` income may leave it out, and its annual
    /// benefit at the claim is then computed from the owner's earnings
    /// record and the salary the plan projects; stated, it is used as is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Dollars>,
    /// First year received; absent means from plan start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Trigger>,
    /// Last year received; absent means through the horizon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<Trigger>,
    /// One-time receipt in the trigger's year; excludes `start`/`end`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<Trigger>,
    /// How the amount escalates: plan inflation (`true`, default), frozen
    /// nominal (`false`), or a fixed annual rate of its own.
    #[serde(default)]
    pub cola: ColaSpec,
}

impl Income {
    /// Whether the income's benefit is computed rather than stated: a
    /// `social-security` income with no `amount`.
    #[must_use]
    pub fn is_derived(&self) -> bool {
        self.kind == IncomeKind::SocialSecurity && self.amount.is_none()
    }

    /// Whether the income is `person`'s Social Security benefit.
    #[must_use]
    pub fn is_benefit_of(&self, person: &str) -> bool {
        self.kind == IncomeKind::SocialSecurity && self.owner == person
    }
}
