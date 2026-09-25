use serde::{Deserialize, Serialize};

use super::Dollars;
use super::allocation::Allocation;
use super::triggers::Trigger;

/// The wrapper an account lives in. Roth treatment is the separate
/// [`Account::roth`] flag, since several wrappers hold both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountKind {
    /// Employer 401(k).
    #[serde(rename = "401k")]
    K401k,
    /// Employer 403(b).
    #[serde(rename = "403b")]
    K403b,
    /// Governmental 457(b); exempt from the early-withdrawal penalty.
    #[serde(rename = "457b")]
    K457b,
    /// Pension-attached defined-contribution account.
    #[serde(rename = "414k")]
    K414k,
    /// Traditional or rollover IRA.
    #[serde(rename = "ira")]
    Ira,
    /// SEP IRA; employer contributions only.
    #[serde(rename = "sep-ira")]
    SepIra,
    /// SIMPLE IRA.
    #[serde(rename = "simple-ira")]
    SimpleIra,
    /// Health savings account.
    #[serde(rename = "hsa")]
    Hsa,
    /// Taxable brokerage.
    #[serde(rename = "brokerage")]
    Brokerage,
    /// Cash, savings, CDs.
    #[serde(rename = "cash")]
    Cash,
}

impl AccountKind {
    /// Every kind, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[
        Self::K401k,
        Self::K403b,
        Self::K457b,
        Self::K414k,
        Self::Ira,
        Self::SepIra,
        Self::SimpleIra,
        Self::Hsa,
        Self::Brokerage,
        Self::Cash,
    ];

    /// The kind as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::K401k => "401k",
            Self::K403b => "403b",
            Self::K457b => "457b",
            Self::K414k => "414k",
            Self::Ira => "ira",
            Self::SepIra => "sep-ira",
            Self::SimpleIra => "simple-ira",
            Self::Hsa => "hsa",
            Self::Brokerage => "brokerage",
            Self::Cash => "cash",
        }
    }

    /// The tax treatment class for this wrapper with the given Roth flag.
    #[must_use]
    pub fn treatment(self, is_roth: bool) -> TreatmentClass {
        match self {
            Self::Brokerage | Self::Cash => TreatmentClass::Taxable,
            Self::Hsa => TreatmentClass::Hsa,
            _ if is_roth => TreatmentClass::Roth,
            _ => TreatmentClass::Deferred,
        }
    }

    /// Whether this wrapper can hold Roth money.
    #[must_use]
    pub fn supports_roth(self) -> bool {
        matches!(self, Self::K401k | Self::K403b | Self::K457b | Self::Ira)
    }

    /// Whether employer contributions are valid here.
    #[must_use]
    pub fn accepts_employer(self) -> bool {
        matches!(
            self,
            Self::K401k
                | Self::K403b
                | Self::K457b
                | Self::K414k
                | Self::SepIra
                | Self::SimpleIra
                | Self::Hsa
        )
    }

    /// Whether an employee's after-tax contributions are valid here.
    #[must_use]
    pub fn accepts_after_tax(self) -> bool {
        matches!(self, Self::K401k | Self::K403b)
    }

    /// Whether the kind keeps an untaxed part of its balance: cost basis
    /// where gains are taxed, after-tax basis where withdrawals are.
    #[must_use]
    pub fn keeps_basis(self, is_roth: bool) -> bool {
        self.tracks_basis() || self.treatment(is_roth) == TreatmentClass::Deferred
    }

    /// Whether the kind is an IRA, whose after-tax basis the law pools per
    /// person with the owner's other IRAs.
    #[must_use]
    pub fn is_ira(self) -> bool {
        matches!(self, Self::Ira | Self::SepIra | Self::SimpleIra)
    }

    /// Whether employee contributions are valid here.
    #[must_use]
    pub fn accepts_employee(self) -> bool {
        !matches!(self, Self::SepIra)
    }

    /// Whether the account tracks cost basis for capital-gains treatment.
    #[must_use]
    pub fn tracks_basis(self) -> bool {
        matches!(self, Self::Brokerage)
    }
}

/// Tax treatment classes, also the vocabulary of the withdrawal order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TreatmentClass {
    /// Taxed as it is realized: brokerage, cash.
    Taxable,
    /// Tax-deferred: taxed as ordinary income on withdrawal.
    Deferred,
    /// Roth: withdrawals tax-free.
    Roth,
    /// HSA: withdrawals assumed qualified and tax-free.
    Hsa,
}

impl TreatmentClass {
    /// Every class, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[Self::Taxable, Self::Deferred, Self::Roth, Self::Hsa];

    /// The class as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Taxable => "taxable",
            Self::Deferred => "deferred",
            Self::Roth => "roth",
            Self::Hsa => "hsa",
        }
    }
}

/// A fund bucket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// Unique id other plan items reference.
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The wrapper.
    pub kind: AccountKind,
    /// Roth treatment within the wrapper.
    #[serde(default)]
    pub roth: bool,
    /// Owning person id.
    pub owner: String,
    /// Balance at plan start, in dollars.
    pub balance: Dollars,
    /// The untaxed part of the balance at plan start: cost basis on a
    /// brokerage, defaulting to the balance; after-tax basis on a
    /// tax-deferred account, defaulting to nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis: Option<Dollars>,
    /// A fixed annual return, never varying; none earns nothing. Refused
    /// beside an allocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_return: Option<f64>,
    /// What the account is invested in, which its return follows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation: Option<Allocation>,
    /// Until this fires, the account cannot be drained or transferred from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_until: Option<Trigger>,
    /// Drains before every treatment class; lower values first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drain_priority: Option<u32>,
}

impl Account {
    /// What the account is called where it is shown: its name, or its id
    /// where it has none.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// The account's tax treatment class.
    #[must_use]
    pub fn treatment(&self) -> TreatmentClass {
        self.kind.treatment(self.roth)
    }

    /// Whether the account keeps an untaxed part of its balance.
    #[must_use]
    pub fn keeps_basis(&self) -> bool {
        self.kind.keeps_basis(self.roth)
    }

    /// The untaxed part of the balance at plan start.
    #[must_use]
    pub fn starting_basis(&self) -> Dollars {
        let whole = if self.kind.tracks_basis() {
            self.balance
        } else {
            0
        };
        self.basis.unwrap_or(whole)
    }
}
